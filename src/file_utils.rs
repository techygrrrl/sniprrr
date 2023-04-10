use std::{fs, io};
use std::fs::DirBuilder;
use std::io::ErrorKind::NotFound;
use crate::models::Snippet;

pub fn write_messages_to_file(data: &str) -> io::Result<()> {
    let app_config_path = dirs::config_dir();

    if app_config_path.is_none() {
        return Err(io::Error::new(NotFound, "No app config dir"));
    }

    // Safe to unwrap, just checked.
    let app_config_path = app_config_path.unwrap();
    let app_config_path = app_config_path.join("sniprrr");

    DirBuilder::new()
        .recursive(true)
        .create(&app_config_path)?;

    let app_config_path = app_config_path.join("messages.json");

    fs::write(app_config_path, data)
}

pub fn load_messages_from_file() -> Vec<Snippet> {
    let app_config_path = dirs::config_dir();

    if app_config_path.is_none() {
        return vec![];
    }

    let app_config_path = app_config_path.unwrap();
    let app_config_path = app_config_path
        .join("sniprrr")
        .join("messages.json");

    if !app_config_path.exists() {
        return vec![];
    }
    
    match fs::read_to_string(app_config_path) {
        Ok(file_contents) => {
            let snippets = serde_json::from_str::<Vec<Snippet>>(&file_contents);

            match snippets {
                Ok(snippets) => snippets,
                Err(_) => vec![],
            }
        }
        Err(_) => vec![],
    }
}
