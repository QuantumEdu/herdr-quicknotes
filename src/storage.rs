use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Note {
    pub fn new(title: String, content: String, tags: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            title,
            content,
            created_at: now,
            updated_at: now,
            tags,
        }
    }
}

pub struct NoteStore {
    base_dir: PathBuf,
}

impl NoteStore {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let base_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("herdr")
            .join("quicknotes");

        fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    pub fn list(&self) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
        let mut notes = Vec::new();
        if !self.base_dir.exists() {
            return Ok(notes);
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(data) = fs::read_to_string(&path) {
                    if let Ok(note) = serde_json::from_str::<Note>(&data) {
                        notes.push(note);
                    }
                }
            }
        }

        notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(notes)
    }

    pub fn get(&self, id: &str) -> Result<Option<Note>, Box<dyn std::error::Error>> {
        let path = self.base_dir.join(format!("{}.json", id));
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(path)?;
        let note = serde_json::from_str::<Note>(&data)?;
        Ok(Some(note))
    }

    pub fn save(&self, note: &Note) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.base_dir.join(format!("{}.json", note.id));
        let data = serde_json::to_string_pretty(note)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let path = self.base_dir.join(format!("{}.json", id));
        if path.exists() {
            fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
