use std::{collections::HashMap, io};

use once_cell::sync::Lazy;

use tokio::{fs, sync::OnceCell};

use crate::repo::Repo;

static CONFIG: Lazy<Config> = Lazy::new(Default::default);

pub struct Config {
    inner: OnceCell<HashMap<String, Repo>>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            inner: OnceCell::new(),
        }
    }
}

impl Config {
    pub async fn init() -> Result<(), io::Error> {
        CONFIG
            .inner
            .get_or_try_init(|| async {
                let contents = fs::read_to_string("config.toml").await?;
                toml::from_str(&contents).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            })
            .await?;
        Ok(())
    }

    pub fn get() -> &'static HashMap<String, Repo> {
        CONFIG.inner.get().unwrap()
    }

    pub fn retrieve(key: &str) -> Option<&'static Repo> {
        Self::get().get(key)
    }
}
