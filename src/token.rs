use crate::crypto::{CryptoError, EncryptedRW};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default)]
struct OnDisk {
    tokens: HashMap<String, String>,
}

pub struct TokenStore<T: EncryptedRW> {
    file: T,
    on_disk: OnDisk,
}

impl<T: EncryptedRW> TokenStore<T> {
    pub fn new(file: T) -> Self {
        let content = match file.read() {
            Ok(content) => content,
            Err(e) => match e {
                CryptoError::DecryptError(err) => {
                    println!("{}", err);
                    std::process::exit(1);
                }
                CryptoError::IO(_) => {
                    println!("IO error, clearing disk content");
                    return Self {
                        file,
                        on_disk: OnDisk::default(),
                    };
                }
                CryptoError::EncryptError(_) => {
                    panic!("this should be impossible")
                }
            },
        };

        let Ok(on_disk) = serde_json::from_slice(&content) else {
            return Self {
                file,
                on_disk: OnDisk::default(),
            };
        };
        Self { file, on_disk }
    }

    fn write(&self) -> Result<()> {
        let as_vec = serde_json::to_vec(&self.on_disk)?;
        self.file.write(&as_vec)?;
        Ok(())
    }

    pub fn add_token(&mut self, domain: &str, token: &str) -> anyhow::Result<()> {
        print!("input token for {}: ", domain);
        self.on_disk
            .tokens
            .insert(domain.to_string(), token.to_string());
        self.write()
    }

    pub fn list_domains(&self) -> Vec<String> {
        self.on_disk.tokens.keys().cloned().collect()
    }

    pub fn get(&self, domain: &str) -> Option<String> {
        self.on_disk.tokens.get(domain).cloned()
    }
}
