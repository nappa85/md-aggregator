use reqwest::IntoUrl;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use tracing::error;

#[derive(Deserialize)]
pub struct Repo {
    pub token: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
}

impl Repo {
    pub async fn get(&self) -> Result<Vec<super::TreeEntry>, ()> {
        let tree: Tree = self
            .call(format!(
                "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
                self.owner, self.repo, self.branch
            ))
            .await?;
        Ok(tree.tree.into_iter().map(Into::into).collect())
    }

    pub async fn retrieve(&self, sha: &str) -> Option<Vec<u8>> {
        let res: Blob = self
            .call(format!(
                "https://api.github.com/repos/{}/{}/git/blobs/{}",
                self.owner, self.repo, sha
            ))
            .await
            .ok()?;
        base64::decode(res.content)
            .map_err(|e| error!("GitHub base64 decode error: {}", e))
            .ok()
    }

    async fn call<U: IntoUrl, T: DeserializeOwned>(&self, url: U) -> Result<T, ()> {
        super::CLIENT
            .get(url)
            .header("Authorization", format!("token {}", self.token.as_str()))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "md-aggregator")
            .send()
            .await
            .map_err(|e| error!("GitHub request error: {}", e))?
            .json()
            .await
            .map_err(|e| error!("GitHub response deserialize error: {}", e))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // fuck
struct Tree {
    pub sha: String,
    pub url: String,
    pub tree: Vec<TreeEntry>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeEntry {
    pub path: String,
    pub mode: String,
    #[serde(rename = "type")]
    pub _type: EntryType,
    pub sha: String,
    pub size: Option<usize>,
    pub url: String,
}

impl Into<super::TreeEntry> for TreeEntry {
    fn into(self) -> super::TreeEntry {
        super::TreeEntry {
            is_dir: self._type == EntryType::Tree,
            path: self.path,
            sha: self.sha,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    Blob,
    Tree,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // fuck
struct Blob {
    pub sha: String,
    pub node_id: String,
    pub size: usize,
    pub url: String,
    pub content: String,
    pub encoding: String,
}
