use std::{fs, io};
use std::fs::DirBuilder;
use std::io::ErrorKind::NotFound;

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