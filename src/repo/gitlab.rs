use reqwest::{IntoUrl, Url};

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
            .call_recursive(format!(
                "https://gitlab.com/api/v4/projects/{}/repository/tree?recursive=1&per_page=100&pagination=keyset{}",
                self.id,
                self.branch.as_deref().map(|branch| format!("&ref={}", branch)).unwrap_or_default(),
            ))
            .await?;
        Ok(tree.into_iter().map(Into::into).collect())
    }

    pub async fn retrieve(&self, sha: &str) -> Option<Vec<u8>> {
        let res: Blob =
            self.call(format!("https://gitlab.com/api/v4/projects/{}/repository/blobs/{}", self.id, sha)).await.ok()?;
        base64::decode(res.content.trim()).map_err(|e| error!("GitLab base64 decode error: {e}\n{res:?}")).ok()
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

    async fn call_recursive<U, T>(&self, url: U) -> Result<Vec<T>, ()>
    where
        U: IntoUrl,
        T: DeserializeOwned,
    {
        let mut url = url.into_url().map_err(|e| error!("Invalid GitLab URL: {}", e))?;
        let mut acc = vec![];

        loop {
            let mut req = super::CLIENT.get(url).header("User-Agent", "md-aggregator");
            if let Some(token) = self.token.as_deref() {
                req = req.header("Authorization", format!("Bearer {}", token));
            }
            let res = req.send().await.map_err(|e| error!("GitLab request error: {}", e))?;

            // link: <https://gitlab.com/api/v4/projects/...>; rel="next", <https://gitlab.com/api/v4/projects/...>; rel="first", <https://gitlab.com/api/v4/projects/...>; rel="last"
            let temp_url = if let Some(v) = res.headers().get("link") {
                let header = v.to_str().map_err(|e| error!("GitLab Header decode error: {}", e))?;
                if let Some(next) = header.split(", ").find(|s| s.ends_with(">; rel=\"next\"")) {
                    Some(Url::parse(&next[1..(next.len() - 13)]).map_err(|e| error!("Invalid GitLab URL: {}", e))?)
                } else {
                    None
                }
            } else {
                None
            };

            let t: Vec<T> = res.json().await.map_err(|e| error!("GitLab response deserialize error: {}", e))?;

            acc.extend(t);

            if let Some(u) = temp_url {
                url = u;
            } else {
                break;
            }
        }

        Ok(acc)
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

impl From<TreeEntry> for super::TreeEntry {
    fn from(te: TreeEntry) -> Self {
        super::TreeEntry { is_dir: te._type == EntryType::Tree, path: te.path, sha: te.id }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
