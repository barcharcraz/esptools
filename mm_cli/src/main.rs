// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use clap::{Args, Parser, Subcommand};
use log::{info, log};
use serde::{Deserialize, Serialize};
use std::{
    env::current_exe,
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    str::FromStr,
    stringify
};
use strum::{EnumDiscriminants, EnumString, IntoStaticStr};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct mm_cli {
    #[command(subcommand)]
    command: mm_cli_subcommands,
}

#[derive(Subcommand)]
enum mm_cli_subcommands {
    Api(api_cli),
    Config(config_cli),
}

#[derive(Args)]
struct api_cli {}

#[derive(Args)]
struct config_cli {
    #[command(subcommand)]
    command: config_cli_commands,
}

#[derive(Subcommand)]
enum config_cli_commands {
    Get { key: String },
    Set { key: String, value: String },
    List,
    Clear
}

impl config_cli {
    fn run(self) -> io::Result<()> {
        use config_cli_commands::*;
        Ok(match self.command {
            List => {
                println!("{:?}", Settings::load_or_default());
            }
            Set { key, value } => {
                let mut settings = Settings::load_or_default()?;
                settings.set(&key, &value);
                settings.save().unwrap();
            }
            Get { key } => {
                let settings = Settings::load_or_default()?;
                println!("{}: {}", key, settings.get(&key));
            }
            Clear => {
                Settings::load_or_default()?.save()?
            }
            _ => (),
        })
    }
}

macro_rules! stamp_out_settings {
    ($($vis:vis $name:ident : $typ:ty)*) => {
        #[derive(Serialize, Deserialize, Debug, Default)]
        struct Settings {
            $($vis $name: Option<$typ>)*
        }

        impl Settings {
            fn set(&mut self, key: &str, value: &str) {
                match key {
                    $(stringify!($name) => self.$name = Some(<$typ>::from_str(value).unwrap()))*,
                    _ => todo!()
                }
            }
            fn get(&self, key: &str) -> String {
                match key {
                    $(stringify!($name) => self.$name.as_deref().map_or("".to_string(), |v| v.to_string()))*,
                    _ => todo!()
                }
            }
        }

    };
}
stamp_out_settings! {
    api_key: String
}
// #[derive(Serialize, Deserialize, Debug, Default)]
// struct Settings {
//     pub api_key: Option<String>,
// }

impl Settings {
    fn default_path() -> io::Result<PathBuf> {
        let path = current_exe()?
            .parent()
            .ok_or(io::ErrorKind::NotFound)?.join("config.toml");
        Ok(path)
    }
    fn load_from_default_path() -> io::Result<Self> {
        info!("Loading from default path");
        Ok(toml::from_str(&fs::read_to_string(Self::default_path()?)?)?)
    }
    fn load_or_default() -> io::Result<Self> {
        match Self::load_from_default_path() {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Ok(c) => Ok(c),
            Err(e) => Err(e),
        }
    }
    fn save(&self) -> io::Result<()> {
        let mut f = File::create(Self::default_path()?)?;
        f.write_all(toml::to_vec(self).unwrap().as_ref())?;
        Ok(())
    }
}

fn main() {
    env_logger::init();
    let cli = mm_cli::parse();
    use mm_cli_subcommands::*;
    match cli.command {
        Config(conf) => conf.run().unwrap(),
        _ => (),
    }
}
