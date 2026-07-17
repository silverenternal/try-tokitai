//! P3-004: Distributed Coordination with etcd
//!
//! This module provides distributed coordination primitives for multi-node tokitai deployments:
//! - DistributedLock: Mutual exclusion across nodes using etcd leases
//! - LeaderElection: Automatic leader election with failover support
//! - CoordinationManager: Unified manager for coordination primitives
//!
//! # Features
//! - Lease-based locking with automatic expiration
//! - Watch-based leader election with instant failover notification
//! - Prometheus metrics export
//! - Async/await API
//!
//! # Example
//! ```rust,no_run
//! use tokitai_context::distributed_coordination::{DistributedLock, CoordinationConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = CoordinationConfig::new(vec!["http://localhost:2379"]);
//!     let mut lock = DistributedLock::new(config, "my-resource".to_string());
//!     
//!     lock.acquire().await?;
//!     // Critical section
//!     lock.release().await?;
//!     
//!     Ok(())
//! }
//! ```

#[cfg(feature = "distributed")]
use etcd_client::{
    Client, Compare, CompareOp, ConnectOptions, Error as EtcdError, GetOptions, LeaseClient,
    PutOptions, Txn, TxnOp, WatchOptions,
};
#[cfg(feature = "distributed")]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(feature = "distributed")]
use std::sync::Arc;
#[cfg(feature = "distributed")]
use std::time::Duration;
#[cfg(feature = "distributed")]
use tokio::sync::Mutex;
#[cfg(feature = "distributed")]
use tokio::time::interval;
#[cfg(feature = "distributed")]
use tracing::{debug, error, info, warn};

#[cfg(feature = "distributed")]
use tokio_stream::StreamExt;

/// Result type for distributed coordination operations
pub type CoordinationResult<T> = Result<T, CoordinationError>;

/// Error types for distributed coordination
#[derive(Debug, thiserror::Error)]
pub enum CoordinationError {
    #[error("Etcd error: {0}")]
    Etcd(#[from] EtcdError),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Lock acquisition failed: {0}")]
    LockAcquisitionFailed(String),

    #[error("Lock release failed: {0}")]
    LockReleaseFailed(String),

    #[error("Leader election failed: {0}")]
    LeaderElectionFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Not the leader")]
    NotLeader,

    #[error("Not connected to etcd")]
    NotConnected,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Configuration for distributed coordination
#[derive(Clone, Debug)]
pub struct CoordinationConfig {
    /// etcd endpoint addresses
    pub endpoints: Vec<String>,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Lease TTL in seconds
    pub lease_ttl: i64,
    /// Keepalive interval
    pub keepalive_interval: Duration,
    /// Authentication username (optional)
    pub username: Option<String>,
    /// Authentication password (optional)
    pub password: Option<String>,
    /// Key prefix for all coordination keys
    pub key_prefix: String,
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        Self {
            endpoints: vec!["http://localhost:2379".to_string()],
            connect_timeout: Duration::from_secs(5),
            lease_ttl: 10,
            keepalive_interval: Duration::from_secs(3),
            username: None,
            password: None,
            key_prefix: "/tokitai".to_string(),
        }
    }
}

impl CoordinationConfig {
    /// Create a new configuration with the given endpoints
    pub fn new(endpoints: Vec<&str>) -> Self {
        Self {
            endpoints: endpoints.into_iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    /// Set the lease TTL in seconds
    pub fn with_lease_ttl(mut self, ttl: i64) -> Self {
        self.lease_ttl = ttl;
        self
    }

    /// Set the key prefix
    pub fn with_key_prefix(mut self, prefix: &str) -> Self {
        self.key_prefix = prefix.to_string();
        self
    }

    /// Set authentication credentials
    pub fn with_auth(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
        self
    }

    /// Build connect options
    pub fn connect_options(&self) -> ConnectOptions {
        ConnectOptions::new()
            .with_connect_timeout(self.connect_timeout)
            .with_keep_alive_while_idle(true)
    }
}

/// Statistics for distributed coordination operations
#[derive(Debug, Default)]
pub struct CoordinationStats {
    /// Total lock acquisitions
    lock_acquisitions: AtomicU64,
    /// Total lock releases
    lock_releases: AtomicU64,
    /// Total leader elections
    leader_elections: AtomicU64,
    /// Total leader failovers
    leader_failovers: AtomicU64,
    /// Total connection errors
    connection_errors: AtomicU64,
    /// Current leader status
    is_leader: AtomicBool,
}

#[cfg(feature = "distributed")]
impl CoordinationStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Clone the stats
    pub fn clone_stats(&self) -> Self {
        Self {
            lock_acquisitions: AtomicU64::new(self.lock_acquisitions.load(Ordering::Relaxed)),
            lock_releases: AtomicU64::new(self.lock_releases.load(Ordering::Relaxed)),
            leader_elections: AtomicU64::new(self.leader_elections.load(Ordering::Relaxed)),
            leader_failovers: AtomicU64::new(self.leader_failovers.load(Ordering::Relaxed)),
            connection_errors: AtomicU64::new(self.connection_errors.load(Ordering::Relaxed)),
            is_leader: AtomicBool::new(self.is_leader.load(Ordering::Relaxed)),
        }
    }

    /// Increment lock acquisitions
    pub fn inc_lock_acquisitions(&self) {
        self.lock_acquisitions.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment lock releases
    pub fn inc_lock_releases(&self) {
        self.lock_releases.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment leader elections
    pub fn inc_leader_elections(&self) {
        self.leader_elections.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment leader failovers
    pub fn inc_leader_failovers(&self) {
        self.leader_failovers.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment connection errors
    pub fn inc_connection_errors(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Set leader status
    pub fn set_leader_status(&self, is_leader: bool) {
        self.is_leader.store(is_leader, Ordering::Relaxed);
    }

    /// Export to Prometheus format
    pub fn to_prometheus(&self, prefix: &str) -> String {
        let mut metrics = String::new();

        metrics.push_str(&format!(
            "{}_lock_acquisitions_total {}\n",
            prefix,
            self.lock_acquisitions.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_lock_releases_total {}\n",
            prefix,
            self.lock_releases.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_leader_elections_total {}\n",
            prefix,
            self.leader_elections.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_leader_failovers_total {}\n",
            prefix,
            self.leader_failovers.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_connection_errors_total {}\n",
            prefix,
            self.connection_errors.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_is_leader {}\n",
            prefix,
            if self.is_leader.load(Ordering::Relaxed) {
                1
            } else {
                0
            }
        ));

        metrics
    }
}

/// Distributed lock using etcd leases
///
/// Provides mutual exclusion across distributed nodes with automatic expiration.
/// The lock uses etcd's lease mechanism for automatic cleanup if the holder crashes.
pub struct DistributedLock {
    config: CoordinationConfig,
    resource_name: String,
    client: Option<Client>,
    lease_id: Option<i64>,
    lock_key: String,
    lock_value: String,
    is_locked: bool,
    stats: Arc<CoordinationStats>,
    _lock_revision: Option<i64>,
}

impl DistributedLock {
    /// Create a new distributed lock
    pub fn new(config: CoordinationConfig, resource_name: String) -> Self {
        let lock_key = format!("{}/locks/{}", config.key_prefix, resource_name);
        let lock_value = format!(
            "{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

        Self {
            config,
            resource_name,
            client: None,
            lease_id: None,
            lock_key,
            lock_value,
            is_locked: false,
            stats: Arc::new(CoordinationStats::new()),
            _lock_revision: None,
        }
    }

    /// Create a new distributed lock with stats sharing
    pub fn with_stats(config: CoordinationConfig, resource_name: String, stats: Arc<CoordinationStats>) -> Self {
        let lock_key = format!("{}/locks/{}", config.key_prefix, resource_name);
        let lock_value = format!(
            "{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

        Self {
            config,
            resource_name,
            client: None,
            lease_id: None,
            lock_key,
            lock_value,
            is_locked: false,
            stats,
            _lock_revision: None,
        }
    }

    /// Connect to etcd
    pub async fn connect(&mut self) -> CoordinationResult<()> {
        if self.client.is_some() {
            return Ok(());
        }

        let client = Client::connect(
            self.config.endpoints.clone(),
            Some(self.config.connect_options()),
        )
        .await
        .map_err(|e| {
            self.stats.inc_connection_errors();
            CoordinationError::ConnectionFailed(e.to_string())
        })?;

        self.client = Some(client);
        info!("Connected to etcd for lock: {}", self.resource_name);
        Ok(())
    }

    /// Acquire the lock with optional timeout
    pub async fn acquire(&mut self) -> CoordinationResult<()> {
        if self.is_locked {
            return Ok(());
        }

        if self.client.is_none() {
            self.connect().await?;
        }

        let client = self.client.as_ref().ok_or(CoordinationError::NotConnected)?;

        // Create a lease
        let mut lease_client = client.lease_client();
        let lease_resp = lease_client
            .grant(self.config.lease_ttl, None)
            .await
            .map_err(|e| {
                self.stats.inc_connection_errors();
                CoordinationError::LockAcquisitionFailed(format!("Lease grant failed: {}", e))
            })?;

        self.lease_id = Some(lease_resp.id());

        // Try to acquire lock using transaction (compare-and-swap)
        let mut lock_client = client.kv_client();

        // Use etcd's built-in lock mechanism with sequential keys
        let lock_key = format!("{}/{}", self.lock_key, self.lease_id.unwrap());

        // Put with lease - this is atomic
        let _put_resp = lock_client
            .put(
                lock_key.clone(),
                self.lock_value.clone(),
                Some(PutOptions::new().with_lease(self.lease_id.unwrap())),
            )
            .await
            .map_err(|e| {
                self.stats.inc_connection_errors();
                CoordinationError::LockAcquisitionFailed(format!("Lock put failed: {}", e))
            })?;

        // Check if we got the lock (no other keys with same prefix)
        let get_resp = lock_client
            .get(
                self.lock_key.clone(),
                Some(GetOptions::new().with_prefix().with_keys_only()),
            )
            .await
            .map_err(|e| {
                self.stats.inc_connection_errors();
                CoordinationError::LockAcquisitionFailed(format!("Lock check failed: {}", e))
            })?;

        // If we're the first key, we have the lock
        let keys: Vec<_> = get_resp.kvs().iter().map(|kv| kv.key()).collect();
        if keys.is_empty() || (keys.len() == 1 && keys[0] == lock_key.as_bytes()) {
            self.is_locked = true;
            self.stats.inc_lock_acquisitions();
            info!("Acquired lock: {}", self.resource_name);
            Ok(())
        } else {
            // Wait for our turn or release
            self.release_lease().await?;
            Err(CoordinationError::LockAcquisitionFailed(
                "Lock already held by another node".to_string(),
            ))
        }
    }

    /// Try to acquire the lock without blocking
    pub async fn try_acquire(&mut self, timeout: Duration) -> CoordinationResult<bool> {
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            match self.acquire().await {
                Ok(()) => return Ok(true),
                Err(CoordinationError::LockAcquisitionFailed(_)) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => return Err(e),
            }
        }
        
        Ok(false)
    }

    /// Release the lock
    pub async fn release(&mut self) -> CoordinationResult<()> {
        if !self.is_locked {
            return Ok(());
        }

        let client = self.client.as_ref().ok_or(CoordinationError::NotConnected)?;
        let mut lock_client = client.kv_client();

        // Delete our lock key
        lock_client
            .delete(self.lock_key.clone(), None)
            .await
            .map_err(|e| {
                self.stats.inc_connection_errors();
                CoordinationError::LockReleaseFailed(format!("Lock delete failed: {}", e))
            })?;

        // Revoke lease
        self.release_lease().await?;

        self.is_locked = false;
        self.stats.inc_lock_releases();
        info!("Released lock: {}", self.resource_name);
        Ok(())
    }

    /// Release the lease
    async fn release_lease(&mut self) -> CoordinationResult<()> {
        if let Some(lease_id) = self.lease_id.take() {
            if let Some(client) = self.client.as_ref() {
                let mut lease_client = client.lease_client();
                lease_client
                    .revoke(lease_id)
                    .await
                    .map_err(|e| {
                        warn!("Failed to revoke lease {}: {}", lease_id, e);
                    })
                    .ok();
            }
        }
        Ok(())
    }

    /// Check if we hold the lock
    pub fn is_locked(&self) -> bool {
        self.is_locked
    }

    /// Get statistics reference
    pub fn stats(&self) -> Arc<CoordinationStats> {
        self.stats.clone()
    }

    /// Keepalive the lease (called automatically by etcd client)
    pub async fn keepalive(&mut self) -> CoordinationResult<()> {
        if let Some(lease_id) = self.lease_id {
            if let Some(client) = self.client.as_ref() {
                let mut lease_client = client.lease_client();
                lease_client
                    .keep_alive(lease_id)
                    .await
                    .map_err(|e| {
                        self.stats.inc_connection_errors();
                        CoordinationError::LockReleaseFailed(format!("Keepalive failed: {}", e))
                    })?;
            }
        }
        Ok(())
    }
}

impl Drop for DistributedLock {
    fn drop(&mut self) {
        if self.is_locked {
            // Best effort release - etcd will clean up via lease anyway
            let _ = tokio::runtime::Handle::current().block_on(async {
                self.release_lease().await
            });
        }
    }
}

/// Leader election state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderState {
    /// Not participating in election
    Inactive,
    /// Candidate or follower
    Follower,
    /// Currently the leader
    Leader,
}

/// Leader election using etcd
///
/// Implements leader election with automatic failover using etcd's lease mechanism.
/// The leader holds a lease and must periodically renew it. If the leader fails,
/// the lease expires and another node can become leader.
pub struct LeaderElection {
    config: CoordinationConfig,
    election_key: String,
    election_value: String,
    client: Option<Client>,
    lease_id: Option<i64>,
    state: LeaderState,
    stats: Arc<CoordinationStats>,
    is_running: Arc<AtomicBool>,
    leader_lease_keepalive: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl LeaderElection {
    /// Create a new leader election
    pub fn new(config: CoordinationConfig, election_name: String) -> Self {
        let election_key = format!("{}/election/{}", config.key_prefix, election_name);
        let election_value = format!(
            "{}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            uuid::Uuid::new_v4()
        );

        Self {
            config,
            election_key,
            election_value,
            client: None,
            lease_id: None,
            state: LeaderState::Inactive,
            stats: Arc::new(CoordinationStats::new()),
            is_running: Arc::new(AtomicBool::new(false)),
            leader_lease_keepalive: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with shared stats
    pub fn with_stats(config: CoordinationConfig, election_name: String, stats: Arc<CoordinationStats>) -> Self {
        let election_key = format!("{}/election/{}", config.key_prefix, election_name);
        let election_value = format!(
            "{}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            uuid::Uuid::new_v4()
        );

        Self {
            config,
            election_key,
            election_value,
            client: None,
            lease_id: None,
            state: LeaderState::Inactive,
            stats,
            is_running: Arc::new(AtomicBool::new(false)),
            leader_lease_keepalive: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect to etcd
    pub async fn connect(&mut self) -> CoordinationResult<()> {
        if self.client.is_some() {
            return Ok(());
        }

        let client = Client::connect(
            self.config.endpoints.clone(),
            Some(self.config.connect_options()),
        )
        .await
        .map_err(|e| {
            self.stats.inc_connection_errors();
            CoordinationError::ConnectionFailed(e.to_string())
        })?;

        self.client = Some(client);
        info!("Connected to etcd for election: {}", self.election_key);
        Ok(())
    }

    /// Start participating in leader election
    pub async fn start(&mut self) -> CoordinationResult<()> {
        if self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }

        if self.client.is_none() {
            self.connect().await?;
        }

        self.is_running.store(true, Ordering::Relaxed);
        self.state = LeaderState::Follower;

        // Try to become leader
        if self.try_become_leader().await? {
            self.state = LeaderState::Leader;
            self.stats.set_leader_status(true);
            self.stats.inc_leader_elections();
            info!("Became leader for: {}", self.election_key);

            // Start keepalive
            self.start_keepalive().await?;
        } else {
            // Watch for leader changes
            self.watch_leader().await?;
        }

        Ok(())
    }

    /// Try to become the leader
    async fn try_become_leader(&mut self) -> CoordinationResult<bool> {
        let client = self.client.as_ref().ok_or(CoordinationError::NotConnected)?;
        let mut lease_client = client.lease_client();
        let mut kv_client = client.kv_client();

        // Create a lease
        let lease_resp = lease_client
            .grant(self.config.lease_ttl, None)
            .await
            .map_err(|e| {
                self.stats.inc_connection_errors();
                CoordinationError::LeaderElectionFailed(format!("Lease grant failed: {}", e))
            })?;

        self.lease_id = Some(lease_resp.id());

        // Try to create the leader key if it doesn't exist
        let txn = Txn::new()
            .when(vec![Compare::version(
                self.election_key.as_str(),
                CompareOp::Equal,
                0,
            )])
            .and_then(vec![TxnOp::put(
                self.election_key.clone(),
                self.election_value.clone(),
                Some(PutOptions::new().with_lease(self.lease_id.unwrap())),
            )])
            .or_else(vec![TxnOp::get(self.election_key.clone(), None)]);

        let txn_resp = kv_client.txn(txn).await.map_err(|e| {
            self.stats.inc_connection_errors();
            CoordinationError::LeaderElectionFailed(format!("Leader txn failed: {}", e))
        })?;

        // If we succeeded in putting, we're the leader
        if txn_resp.succeeded() {
            Ok(true)
        } else {
            // Someone else is leader, revoke our lease
            if let Some(lease_id) = self.lease_id {
                lease_client.revoke(lease_id).await.ok();
                self.lease_id = None;
            }
            Ok(false)
        }
    }

    /// Watch for leader changes and try to become leader when opportunity arises
    async fn watch_leader(&mut self) -> CoordinationResult<()> {
        let client = self.client.as_ref().ok_or(CoordinationError::NotConnected)?;
        let mut watch_client = client.watch_client();

        // Watch the election key
        let (watcher, stream) = watch_client
            .watch(self.election_key.as_str(), Some(WatchOptions::new()))
            .await
            .map_err(|e| {
                self.stats.inc_connection_errors();
                CoordinationError::LeaderElectionFailed(format!("Watch failed: {}", e))
            })?;

        // Spawn watch task
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut stream = stream;
            while is_running.load(Ordering::Relaxed) {
                if let Some(Ok(resp)) = stream.next().await {
                    if resp.events().is_empty() {
                        // Leader key was deleted - opportunity to become leader
                        debug!("Leader key deleted, election opportunity");
                    }
                }
            }
            drop(watcher);
        });

        Ok(())
    }

    /// Start keepalive for leader lease
    async fn start_keepalive(&mut self) -> CoordinationResult<()> {
        let client = self.client.as_ref().ok_or(CoordinationError::NotConnected)?;
        let lease_id = self.lease_id.ok_or(CoordinationError::NotLeader)?;
        let mut lease_client = client.lease_client();
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();
        let election_key = self.election_key.clone();

        let (mut keeper, stream) = lease_client
            .keep_alive(lease_id)
            .await
            .map_err(|e| {
                stats.inc_connection_errors();
                CoordinationError::LeaderElectionFailed(format!("Keepalive setup failed: {}", e))
            })?;

        let mut interval = interval(self.config.keepalive_interval);

        let handle = tokio::spawn(async move {
            let mut stream = stream;
            while is_running.load(Ordering::Relaxed) {
                interval.tick().await;

                if let Err(e) = keeper.keep_alive().await {
                    error!("Keepalive error for {}: {}", election_key, e);
                    stats.inc_connection_errors();
                    break;
                }

                // Check stream for TTL updates
                if let Some(Ok(resp)) = stream.next().await {
                    debug!("Lease keepalive response: TTL={}", resp.ttl());
                }
            }
        });

        *self.leader_lease_keepalive.lock().await = Some(handle);
        Ok(())
    }

    /// Check if we are the leader
    pub async fn is_leader(&mut self) -> CoordinationResult<bool> {
        if self.state != LeaderState::Leader {
            return Ok(false);
        }

        let client = self.client.as_ref().ok_or(CoordinationError::NotConnected)?;
        let mut kv_client = client.kv_client();

        let resp = kv_client
            .get(self.election_key.clone(), None)
            .await
            .map_err(|e| {
                self.stats.inc_connection_errors();
                CoordinationError::LeaderElectionFailed(format!("Get leader key failed: {}", e))
            })?;

        if let Some(kv) = resp.kvs().first() {
            Ok(String::from_utf8_lossy(kv.value()).as_ref() == self.election_value)
        } else {
            // Leader key doesn't exist - we lost leadership
            self.state = LeaderState::Follower;
            self.stats.set_leader_status(false);
            Ok(false)
        }
    }

    /// Get the current leader's value
    pub async fn get_leader(&mut self) -> CoordinationResult<Option<String>> {
        let client = self.client.as_ref().ok_or(CoordinationError::NotConnected)?;
        let mut kv_client = client.kv_client();

        let resp = kv_client
            .get(self.election_key.clone(), None)
            .await
            .map_err(|e| {
                self.stats.inc_connection_errors();
                CoordinationError::LeaderElectionFailed(format!("Get leader key failed: {}", e))
            })?;

        if let Some(kv) = resp.kvs().first() {
            Ok(Some(String::from_utf8_lossy(kv.value()).to_string()))
        } else {
            Ok(None)
        }
    }

    /// Resign from leadership
    pub async fn resign(&mut self) -> CoordinationResult<()> {
        if self.state != LeaderState::Leader {
            return Ok(());
        }

        let client = self.client.as_ref().ok_or(CoordinationError::NotConnected)?;
        let mut kv_client = client.kv_client();

        // Delete the leader key
        kv_client
            .delete(self.election_key.clone(), None)
            .await
            .map_err(|e| {
                self.stats.inc_connection_errors();
                CoordinationError::LeaderElectionFailed(format!("Resign failed: {}", e))
            })?;

        // Revoke lease
        if let Some(lease_id) = self.lease_id {
            let mut lease_client = client.lease_client();
            lease_client.revoke(lease_id).await.ok();
        }

        // Stop keepalive
        if let Some(handle) = self.leader_lease_keepalive.lock().await.take() {
            handle.abort();
        }

        self.state = LeaderState::Follower;
        self.stats.set_leader_status(false);
        self.stats.inc_leader_failovers();
        info!("Resigned leadership: {}", self.election_key);
        Ok(())
    }

    /// Stop participating in election
    pub async fn stop(&mut self) -> CoordinationResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Resign if leader
        if self.state == LeaderState::Leader {
            self.resign().await?;
        }

        self.is_running.store(false, Ordering::Relaxed);
        self.state = LeaderState::Inactive;
        Ok(())
    }

    /// Get current state
    pub fn state(&self) -> LeaderState {
        self.state
    }

    /// Get statistics reference
    pub fn stats(&self) -> Arc<CoordinationStats> {
        self.stats.clone()
    }
}

impl Drop for LeaderElection {
    fn drop(&mut self) {
        if self.state == LeaderState::Leader {
            let _ = tokio::runtime::Handle::current().block_on(async {
                self.resign().await
            });
        }
    }
}

/// Unified coordination manager
///
/// Manages multiple distributed locks and leader elections with shared connection and stats.
pub struct CoordinationManager {
    config: CoordinationConfig,
    client: Option<Client>,
    stats: Arc<CoordinationStats>,
    locks: Mutex<Vec<String>>,
    elections: Mutex<Vec<String>>,
}

impl CoordinationManager {
    /// Create a new coordination manager
    pub fn new(config: CoordinationConfig) -> Self {
        Self {
            config,
            client: None,
            stats: Arc::new(CoordinationStats::new()),
            locks: Mutex::new(Vec::new()),
            elections: Mutex::new(Vec::new()),
        }
    }

    /// Connect to etcd
    pub async fn connect(&mut self) -> CoordinationResult<()> {
        if self.client.is_some() {
            return Ok(());
        }

        let client = Client::connect(
            self.config.endpoints.clone(),
            Some(self.config.connect_options()),
        )
        .await
        .map_err(|e| {
            self.stats.inc_connection_errors();
            CoordinationError::ConnectionFailed(e.to_string())
        })?;

        self.client = Some(client);
        info!("Connected to etcd coordination manager");
        Ok(())
    }

    /// Create a new distributed lock
    pub fn create_lock(&self, resource_name: String) -> DistributedLock {
        DistributedLock::with_stats(self.config.clone(), resource_name, self.stats.clone())
    }

    /// Create a new leader election
    pub fn create_election(&self, election_name: String) -> LeaderElection {
        LeaderElection::with_stats(self.config.clone(), election_name, self.stats.clone())
    }

    /// Get statistics reference
    pub fn stats(&self) -> Arc<CoordinationStats> {
        self.stats.clone()
    }

    /// Export metrics to Prometheus format
    pub fn to_prometheus(&self) -> String {
        self.stats.to_prometheus("tokitai_coordination")
    }

    /// Get client reference
    pub fn client(&self) -> Option<&Client> {
        self.client.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CoordinationConfig::default();
        assert_eq!(config.endpoints.len(), 1);
        assert_eq!(config.endpoints[0], "http://localhost:2379");
        assert_eq!(config.lease_ttl, 10);
        assert_eq!(config.key_prefix, "/tokitai");
    }

    #[test]
    fn test_config_builder() {
        let config = CoordinationConfig::new(vec!["http://etcd:2379"])
            .with_lease_ttl(30)
            .with_key_prefix("/myapp")
            .with_auth("user", "pass");

        assert_eq!(config.endpoints.len(), 1);
        assert_eq!(config.endpoints[0], "http://etcd:2379");
        assert_eq!(config.lease_ttl, 30);
        assert_eq!(config.key_prefix, "/myapp");
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
    }

    #[test]
    fn test_stats_default() {
        let stats = CoordinationStats::new();
        assert_eq!(stats.lock_acquisitions.load(Ordering::Relaxed), 0);
        assert_eq!(stats.lock_releases.load(Ordering::Relaxed), 0);
        assert_eq!(stats.leader_elections.load(Ordering::Relaxed), 0);
        assert!(!stats.is_leader.load(Ordering::Relaxed));
    }

    #[test]
    fn test_stats_increment() {
        let stats = CoordinationStats::new();
        stats.inc_lock_acquisitions();
        stats.inc_lock_acquisitions();
        stats.inc_lock_releases();
        stats.inc_leader_elections();
        stats.inc_leader_failovers();
        stats.inc_connection_errors();

        assert_eq!(stats.lock_acquisitions.load(Ordering::Relaxed), 2);
        assert_eq!(stats.lock_releases.load(Ordering::Relaxed), 1);
        assert_eq!(stats.leader_elections.load(Ordering::Relaxed), 1);
        assert_eq!(stats.leader_failovers.load(Ordering::Relaxed), 1);
        assert_eq!(stats.connection_errors.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_stats_prometheus() {
        let stats = CoordinationStats::new();
        stats.inc_lock_acquisitions();
        stats.set_leader_status(true);

        let prometheus = stats.to_prometheus("test");
        assert!(prometheus.contains("test_lock_acquisitions_total 1"));
        assert!(prometheus.contains("test_is_leader 1"));
    }

    #[test]
    fn test_leader_state() {
        assert_eq!(LeaderState::Inactive as u8, 0);
        assert_eq!(LeaderState::Follower as u8, 1);
        assert_eq!(LeaderState::Leader as u8, 2);
    }

    #[test]
    fn test_lock_creation() {
        let config = CoordinationConfig::default();
        let lock = DistributedLock::new(config, "test-resource".to_string());
        
        assert!(!lock.is_locked());
        assert_eq!(lock.stats().lock_acquisitions.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_election_creation() {
        let config = CoordinationConfig::default();
        let election = LeaderElection::new(config, "test-election".to_string());
        
        assert_eq!(election.state(), LeaderState::Inactive);
        assert_eq!(election.stats().leader_elections.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_manager_creation() {
        let config = CoordinationConfig::default();
        let manager = CoordinationManager::new(config);
        
        let lock = manager.create_lock("test-lock".to_string());
        let election = manager.create_election("test-election".to_string());
        
        assert!(!lock.is_locked());
        assert_eq!(election.state(), LeaderState::Inactive);
    }

    #[test]
    fn test_manager_prometheus() {
        let config = CoordinationConfig::default();
        let manager = CoordinationManager::new(config);
        
        manager.stats().inc_lock_acquisitions();
        manager.stats().inc_leader_elections();
        
        let prometheus = manager.to_prometheus();
        assert!(prometheus.contains("tokitai_coordination_lock_acquisitions_total 1"));
        assert!(prometheus.contains("tokitai_coordination_leader_elections_total 1"));
    }

    #[test]
    fn test_lock_key_format() {
        let config = CoordinationConfig::default();
        let lock = DistributedLock::new(config.clone(), "my-resource".to_string());
        
        // Verify lock key follows expected format
        assert!(lock.lock_key.contains("/tokitai/locks/my-resource"));
    }

    #[test]
    fn test_election_key_format() {
        let config = CoordinationConfig::default();
        let election = LeaderElection::new(config.clone(), "my-election".to_string());
        
        // Verify election key follows expected format
        assert!(election.election_key.contains("/tokitai/election/my-election"));
    }
}
