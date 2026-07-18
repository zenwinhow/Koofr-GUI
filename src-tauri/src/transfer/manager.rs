use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use tokio_util::sync::CancellationToken;

use crate::error::AppError;

#[derive(Default)]
pub struct TransferManager {
    active: Mutex<HashMap<String, CancellationToken>>,
}

impl TransferManager {
    pub fn register(&self, transfer_id: &str) -> Result<CancellationToken, AppError> {
        validate_transfer_id(transfer_id)?;
        let mut active = self.active();
        if active.contains_key(transfer_id) {
            return Err(AppError::DuplicateTransfer);
        }
        let token = CancellationToken::new();
        active.insert(transfer_id.to_owned(), token.clone());
        Ok(token)
    }

    pub fn finish(&self, transfer_id: &str) {
        self.active().remove(transfer_id);
    }

    pub fn cancel(&self, transfer_id: &str) -> Result<bool, AppError> {
        validate_transfer_id(transfer_id)?;
        let token = self.active().get(transfer_id).cloned();
        if let Some(token) = token {
            token.cancel();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn cancel_all(&self) {
        for token in self.active().values() {
            token.cancel();
        }
    }

    fn active(&self) -> MutexGuard<'_, HashMap<String, CancellationToken>> {
        match self.active.lock() {
            Ok(active) => active,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

fn validate_transfer_id(transfer_id: &str) -> Result<(), AppError> {
    let parsed =
        uuid::Uuid::parse_str(transfer_id).map_err(|_| AppError::InvalidInput("transfer id"))?;
    if parsed.is_nil() {
        return Err(AppError::InvalidInput("transfer id"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::TransferManager;

    #[test]
    fn transfer_ids_are_unique_and_cancellable() {
        let manager = TransferManager::default();
        let id = uuid::Uuid::new_v4().to_string();
        let token = manager.register(&id).expect("register transfer");
        assert!(manager.register(&id).is_err());
        assert!(manager.cancel(&id).expect("cancel transfer"));
        assert!(token.is_cancelled());
        manager.finish(&id);
        assert!(!manager.cancel(&id).expect("cancel missing transfer"));

        let first = manager
            .register(&uuid::Uuid::new_v4().to_string())
            .expect("register first transfer");
        let second = manager
            .register(&uuid::Uuid::new_v4().to_string())
            .expect("register second transfer");
        manager.cancel_all();
        assert!(first.is_cancelled());
        assert!(second.is_cancelled());
    }
}
