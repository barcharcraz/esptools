use std::{str::FromStr, collections::HashMap, hash::Hash, backtrace::Backtrace};

// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only
use thiserror::Error;
use url::Url;
pub struct NXMUrl {
    pub game_id: String,
    pub mod_id: u64,
    pub file_id: u64,
    pub key: Option<String>,
    pub expires: Option<u64>,
    pub user_id: Option<u64>,
    pub view: Option<bool>,
    pub extra_params: HashMap<String, String>,
}

#[derive(Error, Debug)]
#[error("invalid nxm url")]
pub struct NXMFromUrlError;

// Example:


impl TryFrom<Url> for NXMUrl {
    type Error = NXMFromUrlError;
    fn try_from(value: Url) -> Result<Self, Self::Error> {
        (|| {
            let mut path_iter = value.path_segments()?;
            let query_map: HashMap<_,_> = value.query_pairs().into_owned().collect();
            Some(Self {
                game_id: value.host_str()?.into(),
                mod_id: path_iter.nth(1)?.parse::<u64>().ok()?,
                file_id: path_iter.nth(1)?.parse::<u64>().ok()?,
                key: query_map.get("key").cloned(),
                expires: query_map.get("expires")?.parse::<u64>().ok(),
                user_id: query_map.get("user_id")?.parse::<u64>().ok(),
                view: {
                    let view = query_map.get("view").as_deref();
                }
                    query_map.get("view").as_deref().(|view|{
                    view == "true" || view.parse::<u64>().ok()? > 0
                }),
                extra_params: query_map,
            })
        })().ok_or(NXMFromUrlError)
    }
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum NXMParseError {
    //#[error("Invalid NXM URL")]
    InvalidNXMUrl(#[from] NXMFromUrlError),
    //#[error("Invalid URL")]
    InvalidURL(#[from] url::ParseError)
}

impl FromStr for NXMUrl {
    type Err = NXMParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::try_from(Url::parse(s)?)?)
    }
}

#[test]
fn test_nxm_from_url() {
    let nxm = NXMUrl::from_str(r"nxm://skyrimspecialedition/mods/80974/files/348116?key=qfKo3dvGjWwdHZJGuJcFxQ&expires=1673851268&user_id=123456").unwrap();
    assert_eq!(nxm.game_id, "skyrimspecialedition");
    assert_eq!(nxm.mod_id, 80974);
    assert_eq!(nxm.file_id, 348116);
    assert_eq!(nxm.key.as_deref(), Some("qfKo3dvGjWwdHZJGuJcFxQ"));
    assert_eq!(nxm.expires, Some(1673851268));
    assert_eq!(nxm.user_id, Some(123456));
}
