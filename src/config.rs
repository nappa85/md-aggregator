use std::collections::HashMap;

use once_cell::sync::Lazy;

use crate::repo::Repo;

static CONFIG: Lazy<HashMap<String, Repo>> = Lazy::new(|| {
    let contents = include_str!("../config.toml");
    toml::from_str(contents).unwrap()
});

pub  fn init() {
    Lazy::force(&CONFIG);
}

pub fn get() -> &'static HashMap<String, Repo> {
    &CONFIG
}

pub fn retrieve(key: &str) -> Option<&'static Repo> {
    get().get(key)
}
