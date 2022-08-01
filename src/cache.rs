use std::{borrow::Cow, collections::HashMap, sync::Arc, time::Duration};

use arc_swap::{ArcSwap, Guard};

use once_cell::sync::Lazy;

use serde::Serialize;

use tokio::time::interval;

use tracing::error;

use crate::{config, repo::TreeEntry};

type Cache = HashMap<String, Vec<CacheEntry>>;

// Cache of Tree data
static TREE: Lazy<ArcSwap<Cache>> = Lazy::new(Default::default);

// Unique template compile
static TEMPLATE: Lazy<mustache::Template> = Lazy::new(|| {
    let template = include_str!("../template.mustache");
    mustache::compile_str(template).unwrap()
});

// Final template render cache
static CACHE: Lazy<ArcSwap<String>> = Lazy::new(Default::default);

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
    CACHE.store(template.into());
    TREE.store(cache.into());
    Ok(())
}

pub fn get() -> Guard<Arc<String>> {
    CACHE.load()
}

pub async fn retrieve(path: &str, owner: &str) -> Option<Vec<u8>> {
    let lock = TREE.load();
    for entry in lock.get(path)? {
        if entry.config_entry == owner {
            let repo = config::retrieve(owner)?;
            return repo.retrieve(&entry.tree.sha).await;
        }
    }
    None
}
