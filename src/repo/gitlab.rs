use reqwest::IntoUrl;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use tracing::error;

#[derive(Deserialize)]
pub struct Repo {
    pub token: Option<String>,
    pub id: u64,
    pub branch: Option<String>,
}

impl Repo {
    pub async fn get(&self) -> Result<Vec<super::TreeEntry>, ()> {
        let tree: Vec<TreeEntry> = self
            .call(format!(
                "https://gitlab.com/api/v4/projects/{}/repository/tree?recursive=1&per_page=100{}",
                self.id,
                self.branch
                    .as_deref()
                    .map(|branch| format!("&ref={}", branch))
                    .unwrap_or_default(),
            ))
            .await?;
        Ok(tree.into_iter().map(Into::into).collect())
    }

    pub async fn retrieve(&self, sha: &str) -> Option<String> {
        let res: Blob = self
            .call(format!(
                "https://gitlab.com/api/v4/projects/{}/repository/blobs/{}",
                self.id, sha
            ))
            .await
            .ok()?;
        return Some(res.content);
    }

    async fn call<U: IntoUrl, T: DeserializeOwned>(&self, url: U) -> Result<T, ()> {
        let mut req = super::CLIENT.get(url).header("User-Agent", "md-aggregator");
        if let Some(token) = self.token.as_deref() {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        req.send()
            .await
            .map_err(|e| error!("GitLab request error: {}", e))?
            .json()
            .await
            .map_err(|e| error!("GitLab response deserialize error: {}", e))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeEntry {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub _type: EntryType,
    pub path: String,
    pub mode: String,
}

impl Into<super::TreeEntry> for TreeEntry {
    fn into(self) -> super::TreeEntry {
        super::TreeEntry {
            is_dir: self._type == EntryType::Tree,
            path: self.path,
            sha: self.id,
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
    pub size: usize,
    pub encoding: String,
    pub content: String,
    pub sha: String,
}
