use std::sync::{Arc, mpsc::Sender};

use crate::editor::ChangeEvent;

pub(crate) trait ChangeEventNotifier: Send + Sync {
    fn notify(&self, event: ChangeEvent);
}

#[derive(Clone)]
pub(crate) struct SenderNotifier {
    sender: Sender<ChangeEvent>,
}

impl SenderNotifier {
    pub(crate) fn new(sender: Sender<ChangeEvent>) -> Self {
        Self { sender }
    }
}

impl ChangeEventNotifier for SenderNotifier {
    fn notify(&self, event: ChangeEvent) {
        notify_sender(&self.sender, event);
    }
}

pub(crate) type SharedChangeEventNotifier = Arc<dyn ChangeEventNotifier>;

pub(crate) fn shared_notifier(sender: Sender<ChangeEvent>) -> SharedChangeEventNotifier {
    Arc::new(SenderNotifier::new(sender))
}

pub(crate) fn notify_sender(sender: &Sender<ChangeEvent>, event: ChangeEvent) {
    let _ = sender.send(event);
}
