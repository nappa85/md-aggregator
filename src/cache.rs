use std::{
    borrow::Cow,
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use arc_swap::{ArcSwap, Guard};

use include_uri::include_str_from_url;

use once_cell::sync::Lazy;

use serde::Serialize;

use tokio::time::interval;

use tracing::error;

use crate::{config, repo::TreeEntry};

type Cache = HashMap<String, Vec<CacheEntry>>;

// Article cache in seconds
const CACHE_LIFETIME: u64 = 3600;

// Cache of Tree data
static TREE: Lazy<ArcSwap<Cache>> = Lazy::new(Default::default);

// Unique template compile
static TEMPLATE: Lazy<mustache::Template> = Lazy::new(|| {
    let template = include_str!("../template.mustache");
    mustache::compile_str(template).unwrap()
});

// Final template render cache
static TEMPLATE_CACHE: Lazy<ArcSwap<String>> = Lazy::new(Default::default);

// Articles contents cache
static ARTICLE_CACHE: Lazy<ArcSwap<rpds::HashTrieMapSync<String, ArticleCache>>> =
    Lazy::new(Default::default);

struct ArticleCache {
    article: Vec<u8>,
    timestamp: u64,
}

#[derive(Clone, Serialize)]
struct CacheEntry {
    config_entry: &'static str,
    tree: TreeEntry,
}

impl CacheEntry {
    fn display(&self) -> Cow<'_, str> {
        let leaf = self.tree.path.split('/').last().unwrap_or_default();
        if self.tree.is_dir {
            leaf.into()
        } else {
            // remove .md extension
            format!("{} ({})", &leaf[0..(leaf.len() - 3)], self.config_entry).into()
        }
    }
}

pub async fn init() -> Result<(), ()> {
    // Force template loading
    Lazy::force(&TEMPLATE);

    let mut interval = interval(Duration::from_secs(3600));
    interval.tick().await;

    // First load is sync, then becomes async
    renew().await?;

    tokio::spawn(async move {
        loop {
            interval.tick().await;

            renew_ignoring_errors().await;
        }
    });

    Ok(())
}

async fn renew() -> Result<(), ()> {
    let config = config::get();
    let lock = TREE.load();
    let mut cache = HashMap::clone(lock.as_ref());
    for (key, repo) in config {
        let tree = repo.get().await?;
        insert(&mut cache, key.as_str(), tree);
    }
    finalize(cache)
}

async fn renew_ignoring_errors() {
    let config = config::get();
    let lock = TREE.load();
    let mut cache = HashMap::clone(lock.as_ref());
    for (key, repo) in config {
        if let Ok(tree) = repo.get().await {
            insert(&mut cache, key.as_str(), tree);
        }
    }
    finalize(cache).ok();
}

fn insert(cache: &mut Cache, key: &'static str, tree: Vec<TreeEntry>) {
    cache
        .iter_mut()
        .for_each(|(_, entries)| entries.retain(|entry| entry.config_entry != key));
    for t in tree {
        let entry = cache.entry(t.path.clone()).or_insert_with(Vec::new);
        entry.push(CacheEntry {
            config_entry: key,
            tree: t,
        });
    }
}

#[derive(Serialize)]
struct RenderEntry<'a> {
    path: &'a str,
    is_dir: bool,
    entry: &'a CacheEntry,
    display: Cow<'a, str>,
}

fn finalize(cache: Cache) -> Result<(), ()> {
    let mut data = HashMap::new();
    let mut temp = cache
        .iter()
        .flat_map(|(path, tree)| {
            tree.iter().map(|entry| RenderEntry {
                path,
                is_dir: entry.tree.is_dir,
                entry,
                display: entry.display(),
            })
        })
        .filter(|entry| entry.path.ends_with(".md") || entry.is_dir)
        .collect::<Vec<_>>();
    temp.sort_unstable_by_key(|entry| entry.path);
    data.insert("tree", temp);
    let template = TEMPLATE
        .render_to_string(&data)
        .map_err(|e| error!("Mustache render error: {}", e))?;
    TEMPLATE_CACHE.store(template.into());
    TREE.store(cache.into());
    Ok(())
}

pub fn get() -> Guard<Arc<String>> {
    TEMPLATE_CACHE.load()
}

pub async fn retrieve(path: &str, owner: &str) -> Option<Vec<u8>> {
    let lock = TREE.load();
    for entry in lock.get(path)? {
        if entry.config_entry == owner {
            let repo = config::retrieve(owner)?;
            let cache = ARTICLE_CACHE.load();
            let mut cache = Cow::Borrowed(cache.as_ref());

            // get article from cache before possibly deleting it
            let mut article = cache.get(&entry.tree.sha).map(|ac| ac.article.clone());

            // cleanup outdated articles
            let now = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs();
            let to_be_removed = cache
                .iter()
                .filter_map(|(k, v)| (v.timestamp + CACHE_LIFETIME < now).then(|| k.clone()))
                .collect::<Vec<_>>();
            if !to_be_removed.is_empty() {
                let mut new_cache = cache.into_owned();
                for key in to_be_removed {
                    new_cache.remove_mut(&key);
                }
                cache = Cow::Owned(new_cache);
            }

            // retrieve article if necessary
            if article.is_none() {
                article = repo.retrieve(&entry.tree.sha).await;
                if let Some(article) = &article {
                    let mut new_cache = cache.into_owned();
                    new_cache.insert_mut(
                        entry.tree.sha.clone(),
                        ArticleCache {
                            article: article.clone(),
                            timestamp: now,
                        },
                    );
                    cache = Cow::Owned(new_cache);
                }
            }

            // store new cache version if necessary
            if let Cow::Owned(cache) = cache {
                ARTICLE_CACHE.store(cache.into());
            }

            return article;
        }
    }
    None
}

pub const fn static_cache(path: &str) -> Option<(&'static str, &'static str)> {
    match path.as_bytes() {
        b"/style.css" => Some(("text/css", include_str!("../style.css"))),
        b"/markdown.css" => Some(("text/css", include_str!("../markdown.css"))),
        b"/script.js" => Some(("text/javascript", include_str!("../script.js"))),
        b"/showdown/showdown.min.js" => Some((
            "text/javascript",
            include_str_from_url!("https://unpkg.com/showdown@2.1.0/dist/showdown.min.js"),
        )),
        b"/highlightjs/styles/default.min.css" => Some((
            "text/css",
            include_str_from_url!(
                "https://unpkg.com/@highlightjs/cdn-assets@11.7.0/styles/default.min.css"
            ),
        )),
        b"/highlightjs/styles/github-dark.min.css" => Some((
            "text/css",
            include_str_from_url!(
                "https://unpkg.com/@highlightjs/cdn-assets@11.7.0/styles/github-dark.min.css"
            ),
        )),
        b"/highlightjs/highlight.min.js" => Some((
            "text/javascript",
            include_str_from_url!(
                "https://unpkg.com/@highlightjs/cdn-assets@11.7.0/highlight.min.js"
            ),
        )),
        b"/highlightjs/elixir.min.js" => Some((
            "text/javascript",
            include_str_from_url!("https://unpkg.com/@highlightjs/cdn-assets@11.7.0/elixir.min.js"),
        )),
        b"/highlightjs/go.min.js" => Some((
            "text/javascript",
            include_str_from_url!("https://unpkg.com/@highlightjs/cdn-assets@11.7.0/go.min.js"),
        )),
        b"/highlightjs/rust.min.js" => Some((
            "text/javascript",
            include_str_from_url!("https://unpkg.com/@highlightjs/cdn-assets@11.7.0/rust.min.js"),
        )),
        _ => None,
    }
}
