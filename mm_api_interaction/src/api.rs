use crate::nxm;
use reqwest::{Method, Request, header::HeaderValue};
use std::result::Result;
use url::Url;
const API_ROOT: &str = "https://api.nexusmods.com";
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DownloadLink {
    pub name: String,
    pub short_name: String,
    pub uri: String,
}

pub struct NexusApi {
    apikey: String,
}

pub struct Game {
    api: NexusApi,
    game_id: String,
}

pub struct Mod {
    game: Game,
    id: u64,
}

pub struct File {
    mod_: Mod,
    id: u64,
}

impl NexusApi {
    fn new(apikey: String) -> Self {
        Self { apikey }
    }
    fn game(self, game_id: String) -> Game {
        Game { api: self, game_id }
    }
}

impl Game {
    fn mod_(self, id: u64) -> Mod {
        Mod { game: self, id }
    }
}

impl Mod {
    fn file(self, id: u64) -> File {
        File { mod_: self, id }
    }
}

impl File {
    pub fn download_link(
        &self,
        key: Option<&str>,
        expires: Option<&str>,
    ) -> Request {
        let mut url = Url::parse(&format!(
            "{}/v1/games/{}/mods/{}/files/{}/download_link.json",
            API_ROOT, self.mod_.game.game_id, self.mod_.id, self.id
        ))
        .unwrap();
        {
            let mut query_pairs = url.query_pairs_mut();
            if let Some(key) = key {
                query_pairs.append_pair("key", key);
            }
            if let Some(expires) = expires {
                query_pairs.append_pair("expires", &expires.to_string());
            }
        }
        let mut req = Request::new(Method::GET, url);
        req.headers_mut().append("apikey", HeaderValue::try_from(&self.mod_.game.api.apikey).unwrap());
        req
    }
}

pub mod sync {
    use super::API_ROOT;
    use log::info;
    use url::Url;
    pub fn download_link(
        apikey: String,
        game_domain_name: &str,
        id: u64,
        mod_id: u64,
        key: Option<&str>,
        expires: Option<u64>,
    ) -> Result<String, reqwest::Error> {
        let mut url = Url::parse(API_ROOT)
            .unwrap()
            .join(&format!(
                "/v1/games/{}/mods/{}/files/{}/download_link.json",
                game_domain_name, mod_id, id
            ))
            .unwrap();

        {
            let mut query_pairs = url.query_pairs_mut();
            if let Some(key) = key {
                query_pairs.append_pair("key", key);
            }
            if let Some(expires) = expires {
                query_pairs.append_pair("expires", &expires.to_string());
            }
            query_pairs.finish();
        }
        let mut client = reqwest::blocking::Client::new();
        let resp = client.get(url).header("apikey", apikey).send()?;
        Ok(resp.text()?)
    }
}
