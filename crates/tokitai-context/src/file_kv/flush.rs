//! Background flush thread for FileKV
//!
//! This module handles:
//! - Background flush thread spawning
//! - Flush trigger mechanism
//! - Periodic flush scheduling

use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::memtable::MemTable;

/// Flush message for background thread communication
pub enum FlushMessage {
    /// Trigger an immediate flush
    Trigger,
    /// Stop the background thread
    Stop,
}

/// Flush trigger handle for external triggering
pub struct FlushTrigger {
    trigger: Arc<AtomicBool>,
    sender: Option<Sender<FlushMessage>>,
}

impl FlushTrigger {
    pub fn new() -> Self {
        Self {
            trigger: Arc::new(AtomicBool::new(false)),
            sender: None,
        }
    }

    pub fn with_background_thread(interval_ms: u64, memtable: Arc<MemTable>) -> Self {
        let (tx, rx) = mpsc::channel();
        let trigger = Arc::new(AtomicBool::new(false));
        let trigger_clone = trigger.clone();

        std::thread::spawn(move || {
            background_flush_thread(rx, interval_ms, memtable, trigger_clone);
        });

        Self {
            trigger,
            sender: Some(tx),
        }
    }

    /// Check if flush is requested
    pub fn is_requested(&self) -> bool {
        self.trigger.load(Ordering::Relaxed)
    }

    /// Mark flush as completed
    pub fn mark_completed(&self) {
        self.trigger.store(false, Ordering::Relaxed);
    }

    /// Request a flush
    pub fn request(&self) {
        self.trigger.store(true, Ordering::Relaxed);
    }

    /// Send a flush trigger message (if background thread exists)
    pub fn send_trigger(&self) -> bool {
        if let Some(ref sender) = self.sender {
            sender.send(FlushMessage::Trigger).is_ok()
        } else {
            false
        }
    }

    /// Stop the background thread
    pub fn stop(&self) {
        if let Some(ref sender) = self.sender {
            let _ = sender.send(FlushMessage::Stop);
        }
    }
}

impl Default for FlushTrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for FlushTrigger {
    fn clone(&self) -> Self {
        Self {
            trigger: self.trigger.clone(),
            sender: self.sender.clone(),
        }
    }
}

/// Background flush thread function
fn background_flush_thread(
    rx: Receiver<FlushMessage>,
    interval_ms: u64,
    memtable: Arc<MemTable>,
    flush_trigger: Arc<AtomicBool>,
) {
    loop {
        match rx.recv_timeout(Duration::from_millis(interval_ms)) {
            Ok(FlushMessage::Stop) => break,
            Ok(FlushMessage::Trigger) => {
                flush_trigger.store(true, Ordering::Relaxed);
            }
            Err(_) => {
                // Timeout - check if MemTable should flush
                if memtable.should_flush() {
                    flush_trigger.store(true, Ordering::Relaxed);
                }
            }
        }
    }
}
