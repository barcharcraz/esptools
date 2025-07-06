// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only
#![feature(error_generic_member_access)]
use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};
use enum_dispatch::enum_dispatch;
use mm_api_interaction::{api::sync::download_link, nxm::NXMUrl};
use mm_store::{OsTreeRepo, mutable_tree::MutableTree, ObjectType, Checksum, RepoRead};
use serde::{Deserialize, Serialize};
use std::{
    env::current_exe,
    fs::{self, File},
    io::{self, Write, Read},
    path::PathBuf,
    str::FromStr,
    stringify,
};

#[enum_dispatch(MmCliSubcommands)]
trait MmCliCommand {
    fn run(self) -> anyhow::Result<()>;
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct MmCli {
    #[command(subcommand)]
    command: MmCliSubcommands,
}

#[derive(Subcommand)]
#[enum_dispatch]
enum MmCliSubcommands {
    Api(ApiCli),
    Config(ConfigCli),
    Store(StoreCli)
}

#[derive(Args)]
struct ApiCli {
    #[command(subcommand)]
    command: ApiCliCommands,
}

#[derive(Subcommand)]
enum ApiCliCommands {
    DownloadLink { nxmurl: String },
}

impl MmCliCommand for ApiCli {
    fn run(self) -> anyhow::Result<()> {
        use ApiCliCommands::*;
        match self.command {
            DownloadLink { nxmurl } => {
                let settings = Settings::load_or_default()?;
                let nxm = NXMUrl::from_str(&nxmurl).unwrap();
                println!(
                    "Downlaod Link: {}",
                    download_link(
                        settings.apikey.unwrap(),
                        &nxm.game_id,
                        nxm.file_id,
                        nxm.mod_id,
                        nxm.key.as_deref(),
                        nxm.expires
                    )
                    .unwrap()
                );
                Ok(())
            }
        }
    }
}

#[derive(Args)]
struct ConfigCli {
    #[command(subcommand)]
    command: ConfigCliCommands,
}

#[derive(Subcommand)]
enum ConfigCliCommands {
    Get { key: String },
    Set { key: String, value: String },
    List,
    Clear,
}

impl MmCliCommand for ConfigCli {
    fn run(self) -> anyhow::Result<()> {
        use ConfigCliCommands::*;
        Ok(match self.command {
            List => {
                println!("{:#?}", Settings::load_or_default()?);
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
            Clear => Settings::load_or_default()?.save()?
        })
    }
}

#[derive(Args)]
struct StoreCli {
    repo_dir: Utf8PathBuf,
    #[command(subcommand)]
    command: StoreCliCommands
}
#[derive(Subcommand)]
enum StoreCliCommands {
    WriteDirTree {
        dir: Utf8PathBuf
    },
    Init,
    DumpRepo,
    CatFile {
        #[arg(id="type")]
        typ: ObjectType,
        checksum: Checksum
    }
}

impl MmCliCommand for StoreCli {
    fn run(self) -> anyhow::Result<()> {
        
        use StoreCliCommands::*;
        Ok(match self.command {
            WriteDirTree { dir } => {
                let mut repo = OsTreeRepo::open(&self.repo_dir)?;
                let mut mtree = MutableTree::new();
                repo.write_dirpath_to_mtree(&dir, &mut mtree)?;
            },
            Init => {
                OsTreeRepo::create(&self.repo_dir)?;
            }
            DumpRepo => {
                let repo = OsTreeRepo::open(&self.repo_dir)?;
                println!("{:?}", repo);
            }
            CatFile { typ, checksum } => {
                let repo = OsTreeRepo::open(&self.repo_dir)?;
                let Some(mut object) = repo.try_get(typ, &checksum)? else {
                    println!("Object: {:?} not found in repo", checksum);
                    return Ok(());
                };
                use ObjectType::*;
                match typ {
                    File => {
                        let mut content = String::new();
                        object.read_to_string(&mut content)?;
                        println!("{}", content);
                    }
                    _ => println!("unsupported object type.")
                }
            }
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
    apikey: String
}
// #[derive(Serialize, Deserialize, Debug, Default)]
// struct Settings {
//     pub api_key: Option<String>,
// }

impl Settings {
    fn default_path() -> io::Result<PathBuf> {
        let path = current_exe()?
            .parent()
            .ok_or(io::ErrorKind::NotFound)?
            .join("config.toml");
        Ok(path)
    }
    fn load_or_default() -> anyhow::Result<Self> {
        match fs::read_to_string(Self::default_path()?) {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Ok(s) => Ok(toml::from_str(&s)?),
            Err(e) => Err(e.into()),
        }
    }
    fn save(&self) -> io::Result<()> {
        let mut f = File::create(Self::default_path()?)?;
        write!(f, "{}", toml::to_string(self).unwrap())?;
        Ok(())
    }
}

fn main() {
    env_logger::init();
    let cli = MmCli::parse();
    cli.command.run().unwrap();
}
