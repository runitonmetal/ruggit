use age::scrypt::{Identity, Recipient};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Decryption failed")]
    DecryptError(#[from] age::DecryptError),
    #[error("Encryption failed")]
    EncryptError(#[from] age::EncryptError),
    #[error("IO error")]
    IO(#[from] std::io::Error),
}

pub trait EncryptedRW {
    fn read(&self) -> Result<Vec<u8>, CryptoError>;
    fn write(&self, content: &[u8]) -> Result<(), CryptoError>;
    fn path(&self) -> String;
}

trait Encrypted {
    fn decrypt(&self, content: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn encrypt(&self, content: &[u8]) -> Result<Vec<u8>, CryptoError>;
}

pub struct PasswdProtectedFile {
    path: PathBuf,
    identity: Identity,
    recipient: Recipient,
}

impl PasswdProtectedFile {
    pub fn new(passwd: &str, path: PathBuf) -> Self {
        Self {
            recipient: Recipient::new(passwd.into()),
            identity: Identity::new(passwd.into()),
            path,
        }
    }
}

impl Encrypted for PasswdProtectedFile {
    fn decrypt(&self, content: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Ok(age::decrypt(&self.identity, content)?)
    }

    fn encrypt(&self, content: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Ok(age::encrypt(&self.recipient, content)?)
    }
}

impl EncryptedRW for PasswdProtectedFile {
    fn read(&self) -> Result<Vec<u8>, CryptoError> {
        let content = std::fs::read(&self.path)?;
        self.decrypt(&content)
    }

    fn write(&self, content: &[u8]) -> Result<(), CryptoError> {
        let content = self.encrypt(content)?;
        Ok(std::fs::write(&self.path, content)?)
    }

    fn path(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}
