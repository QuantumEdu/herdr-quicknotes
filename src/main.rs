use clap::{Parser, Subcommand};
use std::process;

mod storage;
mod tui;

use storage::{Note, NoteStore};
use tui::{App, run_tui};

#[derive(Parser)]
#[command(name = "herdr-quicknotes")]
#[command(about = "Fast, popup notes manager for Herdr with fuzzy search, CLI & clipboard support")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the interactive popup TUI (default)
    Popup,
    /// Add a new note via CLI
    Add {
        #[arg(short, long)]
        title: String,
        #[arg(short, long)]
        content: String,
        #[arg(short, long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },
    /// List all notes in JSON or table format
    List {
        #[arg(long)]
        json: bool,
    },
    /// Get content of a specific note by ID
    Get {
        id: String,
    },
    /// Delete a note by ID
    Delete {
        id: String,
    },
    /// Copy note content to system clipboard
    Copy {
        id: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let store = NoteStore::new()?;

    match cli.command {
        None | Some(Commands::Popup) => {
            let app = App::new(store)?;
            run_tui(app)?;
        }
        Some(Commands::Add { title, content, tags }) => {
            let note = Note::new(title, content, tags.unwrap_or_default());
            store.save(&note)?;
            println!("✓ Nota creada: [{}] {}", note.id, note.title);
        }
        Some(Commands::List { json }) => {
            let notes = store.list()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&notes)?);
            } else {
                if notes.is_empty() {
                    println!("No hay notas registradas.");
                } else {
                    println!("{:<10} {:<30} {:<20}", "ID", "TÍTULO", "ACTUALIZADA");
                    println!("{:-<62}", "");
                    for n in notes {
                        let date = n.updated_at.format("%Y-%m-%d %H:%M").to_string();
                        println!("{:<10} {:<30} {:<20}", n.id, n.title, date);
                    }
                }
            }
        }
        Some(Commands::Get { id }) => {
            if let Some(note) = store.get(&id)? {
                println!("# {}\n", note.title);
                println!("ID: {}", note.id);
                println!("Fecha: {}", note.updated_at.format("%Y-%m-%d %H:%M:%S"));
                println!("\n{}", note.content);
            } else {
                eprintln!("Error: Nota '{}' no encontrada", id);
                process::exit(1);
            }
        }
        Some(Commands::Delete { id }) => {
            if store.delete(&id)? {
                println!("✓ Nota '{}' eliminada", id);
            } else {
                eprintln!("Error: Nota '{}' no encontrada", id);
                process::exit(1);
            }
        }
        Some(Commands::Copy { id }) => {
            if let Some(note) = store.get(&id)? {
                let text = format!("{}\n\n{}", note.title, note.content);
                let mut clipboard = arboard::Clipboard::new()?;
                clipboard.set_text(text)?;
                println!("✓ Contenido de [{}] copiado al portapapeles", note.id);
            } else {
                eprintln!("Error: Nota '{}' no encontrada", id);
                process::exit(1);
            }
        }
    }

    Ok(())
}
