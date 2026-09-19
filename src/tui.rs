use arboard::Clipboard;
use chrono::Utc;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::process::Command;

use crate::storage::{Note, NoteStore};

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    NewTitle,
    NewContent,
    ConfirmDelete,
}

pub struct App {
    pub store: NoteStore,
    pub notes: Vec<Note>,
    pub filtered_indices: Vec<usize>,
    pub selected_idx: usize,
    pub input_mode: InputMode,
    pub search_query: String,
    pub new_title: String,
    pub new_content: String,
    pub status_msg: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(store: NoteStore) -> Result<Self, Box<dyn std::error::Error>> {
        let notes = store.list()?;
        let count = notes.len();
        let mut app = Self {
            store,
            notes,
            filtered_indices: (0..count).collect(),
            selected_idx: 0,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            new_title: String::new(),
            new_content: String::new(),
            status_msg: None,
            should_quit: false,
        };
        app.update_filter();
        Ok(app)
    }

    pub fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.notes.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let mut matches: Vec<(usize, i64)> = self
                .notes
                .iter()
                .enumerate()
                .filter_map(|(idx, note)| {
                    let target = format!("{} {}", note.title, note.content);
                    matcher
                        .fuzzy_match(&target, &self.search_query)
                        .map(|score| (idx, score))
                })
                .collect();
            matches.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered_indices = matches.into_iter().map(|(idx, _)| idx).collect();
        }

        if self.selected_idx >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.selected_idx = self.filtered_indices.len() - 1;
        }
    }

    pub fn selected_note(&self) -> Option<&Note> {
        if self.filtered_indices.is_empty() {
            None
        } else {
            self.filtered_indices
                .get(self.selected_idx)
                .and_then(|&idx| self.notes.get(idx))
        }
    }

    pub fn copy_current_note(&mut self) {
        if let Some(note) = self.selected_note() {
            let text = format!("{}\n\n{}", note.title, note.content);
            match Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_text(text) {
                    Ok(_) => self.status_msg = Some("✓ Nota copiada al portapapeles".to_string()),
                    Err(e) => {
                        self.status_msg = Some(format!("Error al copiar: {}", e));
                    }
                },
                Err(e) => {
                    self.status_msg = Some(format!("Clipboard no disponible: {}", e));
                }
            }
        }
    }

    pub fn edit_in_external_editor(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(note) = self.selected_note() {
            let note_id = note.id.clone();
            let temp_dir = std::env::temp_dir();
            let temp_path = temp_dir.join(format!("quicknote_{}.md", note_id));
            let initial_content = format!("# {}\n\n{}", note.title, note.content);
            std::fs::write(&temp_path, initial_content)?;

            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| "nano".to_string());

            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;

            let status = Command::new(&editor).arg(&temp_path).status();

            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen)?;

            if let Ok(exit_status) = status {
                if exit_status.success() {
                    let edited = std::fs::read_to_string(&temp_path)?;
                    let mut lines = edited.lines();
                    let first_line = lines.next().unwrap_or("").trim_start_matches('#').trim();
                    let title = if first_line.is_empty() {
                        "Sin título".to_string()
                    } else {
                        first_line.to_string()
                    };
                    let content = lines.collect::<Vec<&str>>().join("\n").trim().to_string();

                    if let Some(n) = self.notes.iter_mut().find(|n| n.id == note_id) {
                        n.title = title;
                        n.content = content;
                        n.updated_at = Utc::now();
                        self.store.save(n)?;
                        self.status_msg = Some("✓ Nota actualizada".to_string());
                    }
                }
            }
            let _ = std::fs::remove_file(temp_path);
        }
        Ok(())
    }

    pub fn delete_selected(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(note) = self.selected_note() {
            let id = note.id.clone();
            self.store.delete(&id)?;
            self.notes.retain(|n| n.id != id);
            self.update_filter();
            self.status_msg = Some("✓ Nota eliminada".to_string());
        }
        self.input_mode = InputMode::Normal;
        Ok(())
    }

    pub fn save_new_note(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let title = if self.new_title.trim().is_empty() {
            "Nota rápida".to_string()
        } else {
            self.new_title.trim().to_string()
        };
        let content = self.new_content.trim().to_string();

        let note = Note::new(title, content, Vec::new());
        self.store.save(&note)?;
        self.notes.insert(0, note);
        self.new_title.clear();
        self.new_content.clear();
        self.input_mode = InputMode::Normal;
        self.update_filter();
        self.selected_idx = 0;
        self.status_msg = Some("✓ Nueva nota guardada".to_string());
        Ok(())
    }
}

pub fn run_tui(mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    while !app.should_quit {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                        KeyCode::Char('j') | KeyCode::Down => {
                            if !app.filtered_indices.is_empty()
                                && app.selected_idx + 1 < app.filtered_indices.len()
                            {
                                app.selected_idx += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.selected_idx > 0 {
                                app.selected_idx -= 1;
                            }
                        }
                        KeyCode::Char('/') => {
                            app.input_mode = InputMode::Search;
                            app.search_query.clear();
                            app.update_filter();
                        }
                        KeyCode::Char('n') => {
                            app.input_mode = InputMode::NewTitle;
                            app.new_title.clear();
                            app.new_content.clear();
                        }
                        KeyCode::Char('y') => {
                            app.copy_current_note();
                        }
                        KeyCode::Char('e') => {
                            app.edit_in_external_editor()?;
                        }
                        KeyCode::Char('d') | KeyCode::Char('x') => {
                            if app.selected_note().is_some() {
                                app.input_mode = InputMode::ConfirmDelete;
                            }
                        }
                        _ => {}
                    },
                    InputMode::Search => match key.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                            app.update_filter();
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                            app.update_filter();
                        }
                        _ => {}
                    },
                    InputMode::NewTitle => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Enter => {
                            app.input_mode = InputMode::NewContent;
                        }
                        KeyCode::Backspace => {
                            app.new_title.pop();
                        }
                        KeyCode::Char(c) => {
                            app.new_title.push(c);
                        }
                        _ => {}
                    },
                    InputMode::NewContent => match key.code {
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Enter => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                app.save_new_note()?;
                            } else {
                                app.save_new_note()?;
                            }
                        }
                        KeyCode::Backspace => {
                            app.new_content.pop();
                        }
                        KeyCode::Char(c) => {
                            app.new_content.push(c);
                        }
                        _ => {}
                    },
                    InputMode::ConfirmDelete => match key.code {
                        KeyCode::Char('y') | KeyCode::Enter => {
                            app.delete_selected()?;
                        }
                        _ => {
                            app.input_mode = InputMode::Normal;
                        }
                    },
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & search
            Constraint::Min(5),    // Main panes
            Constraint::Length(2), // Help footer & status
        ])
        .split(f.area());

    // 1. Header & search bar
    let search_style = match app.input_mode {
        InputMode::Search => Style::default().fg(Color::Yellow),
        _ => Style::default().fg(Color::DarkGray),
    };
    let search_title = if app.search_query.is_empty() {
        " Buscar con '/' | [n] Nueva | [y] Copiar | [e] Editar | [d] Borrar | [q] Salir "
    } else {
        " Filtrando notas "
    };
    let search_bar = Paragraph::new(format!(" 🔍 {}", app.search_query)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(search_title)
            .border_style(search_style),
    );
    f.render_widget(search_bar, chunks[0]);

    // 2. Main content: Left list, Right preview
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(chunks[1]);

    // List of notes
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(i, &note_idx)| {
            let note = &app.notes[note_idx];
            let is_selected = i == app.selected_idx;
            let time_str = note.updated_at.format("%d/%m %H:%M").to_string();

            let title_span = if is_selected {
                Span::styled(
                    format!(" ▶ {}", note.title),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw(format!("   {}", note.title))
            };

            let date_span = Span::styled(
                format!(" [{}]", time_str),
                Style::default().fg(Color::DarkGray),
            );

            ListItem::new(Line::from(vec![title_span, date_span]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Notas ({}) ", app.filtered_indices.len()))
        .border_style(Style::default().fg(Color::Blue));
    let list_widget = List::new(items).block(list_block);
    f.render_widget(list_widget, main_chunks[0]);

    // Note preview
    let preview_content = if let Some(note) = app.selected_note() {
        let header = format!(
            "ID: {} | Actualizada: {}\n------------------------------------------------------------\n",
            note.id,
            note.updated_at.format("%Y-%m-%d %H:%M:%S")
        );
        format!("{}\n{}", header, note.content)
    } else {
        "No hay notas seleccionadas o encontradas.\nPresiona [n] para crear una nueva nota.".to_string()
    };

    let preview_title = app
        .selected_note()
        .map(|n| format!(" Vista Previa: {} ", n.title))
        .unwrap_or_else(|| " Vista Previa ".to_string());

    let preview_widget = Paragraph::new(preview_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(preview_title)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(preview_widget, main_chunks[1]);

    // 3. Footer status
    let status_text = if let Some(ref msg) = app.status_msg {
        Span::styled(
            msg,
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "herdr-quicknotes v0.1.0 • QuantumEdu",
            Style::default().fg(Color::DarkGray),
        )
    };
    let footer = Paragraph::new(Line::from(vec![Span::raw(" "), status_text]));
    f.render_widget(footer, chunks[2]);

    // Overlays / Popups for modal modes
    match app.input_mode {
        InputMode::NewTitle => {
            let area = centered_rect(60, 20, f.area());
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Nueva Nota: Título (Enter para continuar, Esc cancelar) ")
                .border_style(Style::default().fg(Color::Yellow));
            let input = Paragraph::new(app.new_title.as_str()).block(block);
            f.render_widget(input, area);
        }
        InputMode::NewContent => {
            let area = centered_rect(70, 40, f.area());
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Nueva Nota: Contenido (Enter guardar, Esc cancelar) ")
                .border_style(Style::default().fg(Color::Yellow));
            let input = Paragraph::new(app.new_content.as_str()).block(block);
            f.render_widget(input, area);
        }
        InputMode::ConfirmDelete => {
            let area = centered_rect(50, 20, f.area());
            f.render_widget(Clear, area);
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Confirmar Eliminación ")
                .border_style(Style::default().fg(Color::Red));
            let text = Paragraph::new("¿Eliminar la nota seleccionada?\n\n[y / Enter] Sí   [Cualquier otra tecla] Cancelar")
                .block(block)
                .style(Style::default().fg(Color::White));
            f.render_widget(text, area);
        }
        _ => {}
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
