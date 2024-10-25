use crate::crypto::{CryptoError, EncryptedRW};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub trait Cachable {
    fn update_cache(&self) -> anyhow::Result<()>;
}

pub struct Cache<T: Serialize + DeserializeOwned + Default, U: EncryptedRW> {
    pub in_mem: T,
    persistent: U,
}

impl<T: Serialize + DeserializeOwned + Default, U: EncryptedRW> Cache<T, U> {
    pub fn new(on_disk: U) -> Self {
        let content = match on_disk.read() {
            Ok(content) => content,
            Err(e) => match e {
                CryptoError::IO(err) => {
                    println!("({}) {}: clearing cache", on_disk.path(), err);
                    return Self {
                        in_mem: T::default(),
                        persistent: on_disk,
                    };
                }
                err => {
                    println!("unrecoverable error: {}", err);
                    std::process::exit(1);
                }
            },
        };

        let in_mem = if let Ok(in_mem) = serde_json::from_slice(&content) {
            in_mem
        } else {
            T::default()
        };
        Self {
            in_mem,
            persistent: on_disk,
        }
    }

    pub fn update(&self) -> anyhow::Result<()> {
        let content = serde_json::to_vec(&self.in_mem)?;
        Ok(self.persistent.write(&content)?)
    }
}

impl<T: Serialize + DeserializeOwned + Default, U: EncryptedRW> Cachable for Cache<T, U> {
    fn update_cache(&self) -> anyhow::Result<()> {
        self.update()
    }
}
