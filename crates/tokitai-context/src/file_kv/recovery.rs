//! WAL recovery operations for FileKV

use crate::error::ContextResult;
use tracing::info;

use super::FileKV;
use crate::wal::WalOperation;
use base64::{Engine, engine::general_purpose::STANDARD};

impl FileKV {
    /// Recover data from WAL after crash
    ///
    /// Replays all WAL entries to restore data that was written but not yet flushed
    /// to segments.
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of entries replayed
    /// * `Err(ContextError)` - On recovery failure
    #[tracing::instrument(skip_all)]
    pub fn recover(&self) -> ContextResult<usize> {
        if let Some(ref wal) = self.wal {
            let mut wal_guard = wal.lock();
            let entries = wal_guard.read_entries()?;
            let count = entries.len();

            if count == 0 {
                return Ok(0);
            }

            info!("Replaying {} WAL entries for recovery", count);

            for entry in &entries {
                match &entry.operation {
                    WalOperation::Add { session: key, hash: _, layer: _ } => {
                        if let Some(payload) = &entry.payload {
                            let parts: Vec<&str> = payload.split(':').collect();
                            if parts.len() >= 3 {
                                if let Ok(len) = parts[0].parse::<usize>() {
                                    if let Ok(value_bytes) = STANDARD.decode(parts[2]) {
                                        if value_bytes.len() == len {
                                            let _ = self.memtable.insert(key.clone(), &value_bytes);
                                            info!("Replayed Add for key: {}", key);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    WalOperation::Delete { session: key, .. } => {
                        let _ = self.memtable.delete(key);
                        info!("Replayed Delete for key: {}", key);
                    }
                    _ => {}
                }
            }

            wal_guard.clear()?;
            info!("Recovery completed, replayed {} entries", count);
            Ok(count)
        } else {
            Ok(0)
        }
    }
}
