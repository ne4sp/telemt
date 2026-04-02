use super::*;
use crate::proxy::ProxySharedState;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::Ordering;

#[test]
fn blackhat_registry_poison_recovers_with_fail_closed_reset_and_pressure_accounting() {
    let shared = ProxySharedState::new();

    let _ = catch_unwind(AssertUnwindSafe(|| {
        let mut guard = shared
            .relay_idle_registry
            .lock()
            .expect("registry lock must be acquired before poison");
        guard.by_conn_id.insert(
            999,
            RelayIdleCandidateMeta {
                mark_order_seq: 1,
                mark_pressure_seq: 0,
            },
        );
        guard.ordered.insert((1, 999));
        panic!("intentional poison for idle-registry recovery");
    }));

    // Helper lock must recover from poison, reset stale state, and continue.
    assert!(super::mark_relay_idle_candidate(shared.as_ref(), 42));
    assert_eq!(super::oldest_relay_idle_candidate(shared.as_ref()), Some(42));

    let before = super::relay_pressure_event_seq(shared.as_ref());
    super::note_relay_pressure_event(shared.as_ref());
    let after = super::relay_pressure_event_seq(shared.as_ref());
    assert!(
        after > before,
        "pressure accounting must still advance after poison"
    );
}

#[test]
fn clear_state_helper_must_reset_poisoned_registry_for_deterministic_fifo_tests() {
    let shared = ProxySharedState::new();

    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _guard = shared
            .relay_idle_registry
            .lock()
            .expect("registry lock must be acquired before poison");
        panic!("intentional poison while lock held");
    }));

    {
        let mut guard = super::relay_idle_candidate_registry_lock(shared.as_ref());
        *guard = Default::default();
    }
    shared.relay_idle_mark_seq.store(0, Ordering::Relaxed);

    assert_eq!(super::oldest_relay_idle_candidate(shared.as_ref()), None);
    assert_eq!(super::relay_pressure_event_seq(shared.as_ref()), 0);

    assert!(super::mark_relay_idle_candidate(shared.as_ref(), 7));
    assert_eq!(super::oldest_relay_idle_candidate(shared.as_ref()), Some(7));
}
