use std::sync::{Arc, Mutex};

use super::profile_store::ProfileStore;
use crate::models::AppSettings;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<ProfileStore>,
    pub settings: Arc<Mutex<AppSettings>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            store: Arc::new(ProfileStore::default()),
            settings: Arc::new(Mutex::new(AppSettings::default())),
        }
    }
}
