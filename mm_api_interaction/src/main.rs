// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use serde_derive::{Serialize, Deserialize};
use std::{env::current_exe, fs, io};




#[derive(Serialize, Deserialize, Debug)]
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
    let _conf: Settings = toml::from_str(&toml_str).unwrap();
    println!("{:?}", _conf);
    Ok(())
}
