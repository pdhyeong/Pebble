use std::sync::{Arc, Mutex, PoisonError, RwLock};

use crate::domain::activity::{ActivityLog, ActivityRecord};
use crate::domain::settings::Settings;

#[derive(Clone)]
pub struct SettingsStore(Arc<RwLock<Settings>>);

impl SettingsStore {
    pub fn new(initial: Settings) -> Self {
        Self(Arc::new(RwLock::new(initial)))
    }

    pub fn get(&self) -> Settings {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn update(&self, change: impl FnOnce(&mut Settings)) {
        change(&mut self.0.write().unwrap_or_else(PoisonError::into_inner));
    }
}

#[derive(Clone, Default)]
pub struct ActivityStore(Arc<Mutex<ActivityLog>>);

impl ActivityStore {
    pub fn push(&self, record: ActivityRecord) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(record);
    }

    pub fn snapshot(&self) -> Vec<ActivityRecord> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .snapshot()
    }
}
