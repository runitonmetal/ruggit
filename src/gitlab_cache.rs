use crate::cache::Cache;
use crate::crypto::EncryptedRW;
use crate::gapi::{GitlabResourceMeta, GitlabVariable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ResourceIdentifier = String;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Resource {
    pub meta: GitlabResourceMeta,
    pub variables: Vec<GitlabVariable>,
}

#[derive(Serialize, Deserialize, Default)]
struct ResourceMap {
    data: HashMap<ResourceIdentifier, Resource>,
}

pub struct CachedResources<Crypto: EncryptedRW> {
    inner: Cache<ResourceMap, Crypto>,
}

impl<Crypto: EncryptedRW> CachedResources<Crypto> {
    pub fn new(on_disk: Crypto) -> Self {
        Self {
            inner: Cache::new(on_disk),
        }
    }

    pub fn insert(&mut self, meta: &GitlabResourceMeta, variables: &[GitlabVariable]) {
        let resource = Resource {
            meta: meta.clone(),
            variables: variables.to_vec(),
        };
        let identifier = 'a: {
            if let Some(path) = &meta.full_path {
                break 'a path;
            } else if let Some(path) = &meta.path_with_namespace {
                break 'a path;
            }
            panic!("gitlab resource with no path")
        };
        self.inner
            .in_mem
            .data
            .insert(identifier.to_string(), resource);
        if self.inner.update().is_err() {
            println!("failed to cache resource map");
        }
    }

    pub fn get(&self, identifier: &ResourceIdentifier) -> Option<Resource> {
        self.inner.in_mem.data.get(identifier).cloned()
    }

    pub fn list(&self) -> Vec<ResourceIdentifier> {
        self.inner
            .in_mem
            .data
            .keys()
            .map(|k| k.to_owned())
            .collect()
    }
}
