use std::sync::{Arc, Mutex};

use keyring_core::{Entry, Error as KeyringError};
use zeroize::Zeroizing;

use crate::error::AppError;

const SERVICE_NAME: &str = "Koofr-GUI";

#[derive(Clone)]
pub struct CredentialManager {
    gate: Arc<Mutex<()>>,
}

impl CredentialManager {
    pub fn initialize() -> Result<Self, AppError> {
        let store =
            windows_native_keyring_store::Store::new().map_err(|_| AppError::CredentialStore)?;
        keyring_core::set_default_store(store);
        Ok(Self {
            gate: Arc::new(Mutex::new(())),
        })
    }

    pub async fn save(&self, email: String, password: Zeroizing<String>) -> Result<(), AppError> {
        let gate = Arc::clone(&self.gate);
        tokio::task::spawn_blocking(move || {
            let _guard = gate.lock().map_err(|_| AppError::CredentialStore)?;
            Entry::new(SERVICE_NAME, &email)
                .and_then(|entry| entry.set_password(password.as_str()))
                .map_err(|_| AppError::CredentialStore)
        })
        .await
        .map_err(|_| AppError::CredentialStore)?
    }

    pub async fn load(&self, email: String) -> Result<Option<Zeroizing<String>>, AppError> {
        let gate = Arc::clone(&self.gate);
        tokio::task::spawn_blocking(move || {
            let _guard = gate.lock().map_err(|_| AppError::CredentialStore)?;
            let entry = Entry::new(SERVICE_NAME, &email).map_err(|_| AppError::CredentialStore)?;
            match entry.get_password() {
                Ok(password) => Ok(Some(Zeroizing::new(password))),
                Err(KeyringError::NoEntry) => Ok(None),
                Err(_) => Err(AppError::CredentialStore),
            }
        })
        .await
        .map_err(|_| AppError::CredentialStore)?
    }

    pub async fn delete(&self, email: String) -> Result<(), AppError> {
        let gate = Arc::clone(&self.gate);
        tokio::task::spawn_blocking(move || {
            let _guard = gate.lock().map_err(|_| AppError::CredentialStore)?;
            let entry = Entry::new(SERVICE_NAME, &email).map_err(|_| AppError::CredentialStore)?;
            match entry.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
                Err(_) => Err(AppError::CredentialStore),
            }
        })
        .await
        .map_err(|_| AppError::CredentialStore)?
    }
}
