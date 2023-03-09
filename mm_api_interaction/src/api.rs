use crate::nxm;

const API_ROOT: &str = "https://api.nexusmods.com";

pub struct DownloadLink {
    pub name: String,
    pub short_name: String,
    pub uri: String
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
        resp.
        Ok(resp.text()?)
    }
}
