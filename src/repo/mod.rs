use std::time::Duration;

use once_cell::sync::Lazy;

use reqwest::Client;

use serde::{Deserialize, Serialize};

mod github;
mod gitlab;

// Unique http client
static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .connect_timeout(Duration::from_secs(60))
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap()
});

#[derive(Deserialize)]
#[serde(tag = "flavour", rename_all = "lowercase")]
pub enum Repo {
    Github(github::Repo),
    Gitlab(gitlab::Repo),
}

#[derive(Clone, Debug, Serialize)]
pub struct TreeEntry {
    pub path: String,
    pub is_dir: bool,
    pub sha: String,
}

impl Repo {
    pub async fn get(&self) -> Result<Vec<TreeEntry>, ()> {
        match self {
            Repo::Github(r) => r.get().await,
            Repo::Gitlab(r) => r.get().await,
        }
    }

    pub async fn retrieve(&self, sha: &str) -> Option<String> {
        match self {
            Repo::Github(r) => r.retrieve(sha).await,
            Repo::Gitlab(r) => r.retrieve(sha).await,
        }
    }
}
