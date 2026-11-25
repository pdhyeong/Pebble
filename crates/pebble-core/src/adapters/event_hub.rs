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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Collect(Mutex<Vec<String>>);

    impl UiEvents for Collect {
        fn emit(&self, event: UiEvent) {
            if let UiEvent::ListeningOn { address } = event {
                self.0.lock().unwrap().push(address);
            }
        }
    }

    fn listening(address: &str) -> UiEvent {
        UiEvent::ListeningOn {
            address: address.to_string(),
        }
    }

    #[test]
    fn emit_reaches_subscribers_and_the_next_sink() {
        let next = Arc::new(Collect::default());
        let hub = EventHub::new(Some(next.clone()));
        let mut first = hub.subscribe();
        let mut second = hub.subscribe();
        hub.emit(listening("/a"));
        for rx in [&mut first, &mut second] {
            assert!(
                matches!(rx.try_recv().unwrap(), UiEvent::ListeningOn { address } if address == "/a")
            );
        }
        assert_eq!(*next.0.lock().unwrap(), vec!["/a".to_string()]);
    }

    #[test]
    fn emit_without_subscribers_still_reaches_the_next_sink() {
        let next = Arc::new(Collect::default());
        let hub = EventHub::new(Some(next.clone()));
        hub.emit(listening("/b"));
        assert_eq!(*next.0.lock().unwrap(), vec!["/b".to_string()]);
        let mut late = hub.subscribe();
        assert!(late.try_recv().is_err());
        let mut via_sender = hub.sender().subscribe();
        hub.emit(listening("/c"));
        assert!(
            matches!(via_sender.try_recv().unwrap(), UiEvent::ListeningOn { address } if address == "/c")
        );
    }
}
