use std::sync::Arc;
use tokio::sync::broadcast;

use crate::application::events::UiEvent;
use crate::application::ports::UiEvents;

pub const EVENT_CAPACITY: usize = 1024;

pub struct EventHub {
    sender: broadcast::Sender<UiEvent>,
    next: Option<Arc<dyn UiEvents>>,
}

impl EventHub {
    pub fn new(next: Option<Arc<dyn UiEvents>>) -> Self {
        let (sender, _) = broadcast::channel(EVENT_CAPACITY);
        Self { sender, next }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<UiEvent> {
        self.sender.subscribe()
    }

    pub fn sender(&self) -> broadcast::Sender<UiEvent> {
        self.sender.clone()
    }
}

impl UiEvents for EventHub {
    fn emit(&self, event: UiEvent) {
        if self.sender.receiver_count() > 0 {
            let _ = self.sender.send(event.clone());
        }
        if let Some(next) = &self.next {
            next.emit(event);
        }
    }
}
