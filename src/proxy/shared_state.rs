//! Per-process (or per-test) shared proxy state for handshake throttling and middle-relay dedup.
//!
//! Replaces `OnceLock` globals so multiple isolated proxy instances or parallel tests can coexist.

use dashmap::DashMap;
use std::collections::{BTreeSet, HashMap};
use std::collections::hash_map::RandomState;
use std::net::IpAddr;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Tracked per client IP for pre-authentication handshake throttling.
#[derive(Clone, Copy)]
pub struct AuthProbeState {
    pub fail_streak: u32,
    pub blocked_until: Instant,
    pub last_seen: Instant,
}

/// Global saturation bucket when the per-IP auth-probe map is under pressure.
#[derive(Clone, Copy)]
pub struct AuthProbeSaturationState {
    pub fail_streak: u32,
    pub blocked_until: Instant,
    pub last_seen: Instant,
}

#[derive(Default)]
pub(crate) struct DesyncDedupRotationState {
    pub(crate) current_started_at: Option<Instant>,
}

#[derive(Default)]
pub(crate) struct RelayIdleCandidateRegistry {
    pub(crate) by_conn_id: HashMap<u64, RelayIdleCandidateMeta>,
    pub(crate) ordered: BTreeSet<(u64, u64)>,
    pub(crate) pressure_event_seq: u64,
    pub(crate) pressure_consumed_seq: u64,
}

#[derive(Clone, Copy)]
pub(crate) struct RelayIdleCandidateMeta {
    pub(crate) mark_order_seq: u64,
    pub(crate) mark_pressure_seq: u64,
}

/// Mutable cross-connection state injected into handshake and middle-relay paths.
pub struct ProxySharedState {
    pub auth_probe: DashMap<IpAddr, AuthProbeState>,
    pub auth_probe_saturation: Mutex<Option<AuthProbeSaturationState>>,
    pub auth_probe_eviction_hasher: RandomState,
    pub desync_dedup: DashMap<u64, Instant>,
    pub desync_dedup_previous: DashMap<u64, Instant>,
    pub desync_hasher: RandomState,
    pub desync_full_cache_last_emit_at: Mutex<Option<Instant>>,
    pub desync_dedup_rotation_state: Mutex<DesyncDedupRotationState>,
    pub relay_idle_registry: Mutex<RelayIdleCandidateRegistry>,
    pub relay_idle_mark_seq: AtomicU64,
}

impl ProxySharedState {
    /// Builds a fresh isolated state (production server uses one `Arc` for the whole runtime).
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            auth_probe: DashMap::new(),
            auth_probe_saturation: Mutex::new(None),
            auth_probe_eviction_hasher: RandomState::new(),
            desync_dedup: DashMap::new(),
            desync_dedup_previous: DashMap::new(),
            desync_hasher: RandomState::new(),
            desync_full_cache_last_emit_at: Mutex::new(None),
            desync_dedup_rotation_state: Mutex::new(DesyncDedupRotationState::default()),
            relay_idle_registry: Mutex::new(RelayIdleCandidateRegistry::default()),
            relay_idle_mark_seq: AtomicU64::new(0),
        })
    }
}
