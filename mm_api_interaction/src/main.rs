use serde_derive::{Serialize, Deserialize};
use std::{env::current_exe, fs, io, thread::current};
use tokio_tungstenite::connect_async;
use url::Url;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct Settings {
    pub api_key: String,
}

fn main() -> io::Result<()> {
    let toml_str = fs::read_to_string(
        current_exe()?
            .parent()
            .ok_or(io::ErrorKind::NotFound)?
            .join("config.toml"),
    )?;
    let conf: Settings = toml::from_str(&toml_str)?;
    Ok(())
}
