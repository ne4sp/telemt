use super::*;
use crate::proxy::ProxySharedState;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn poison_saturation_mutex(shared: &Arc<ProxySharedState>) {
    let shared = Arc::clone(shared);
    let poison_thread = std::thread::spawn(move || {
        let _guard = shared
            .auth_probe_saturation
            .lock()
            .expect("saturation mutex must be lockable for poison setup");
        panic!("intentional poison for saturation mutex resilience test");
    });
    let _ = poison_thread.join();
}

#[test]
fn auth_probe_saturation_note_recovers_after_mutex_poison() {
    let shared = ProxySharedState::new();
    poison_saturation_mutex(&shared);

    let now = Instant::now();
    super::auth_probe_note_saturation(shared.as_ref(), now);

    assert!(
        super::auth_probe_saturation_is_throttled(shared.as_ref(), now),
        "poisoned saturation mutex must not disable saturation throttling"
    );
}

#[test]
fn auth_probe_saturation_check_recovers_after_mutex_poison() {
    let shared = ProxySharedState::new();
    poison_saturation_mutex(&shared);

    {
        let mut guard = super::auth_probe_saturation_state_lock(shared.as_ref());
        *guard = Some(AuthProbeSaturationState {
            fail_streak: AUTH_PROBE_BACKOFF_START_FAILS,
            blocked_until: Instant::now() + Duration::from_millis(10),
            last_seen: Instant::now(),
        });
    }

    assert!(
        super::auth_probe_saturation_is_throttled(shared.as_ref(), Instant::now()),
        "throttle check must recover poisoned saturation mutex and stay fail-closed"
    );
}

#[test]
fn clear_auth_probe_state_clears_saturation_even_if_poisoned() {
    let shared = ProxySharedState::new();
    poison_saturation_mutex(&shared);

    super::auth_probe_note_saturation(shared.as_ref(), Instant::now());
    assert!(super::auth_probe_saturation_is_throttled(
        shared.as_ref(),
        Instant::now()
    ));

    shared.auth_probe.clear();
    if let Ok(mut g) = shared.auth_probe_saturation.lock() {
        *g = None;
    }

    assert!(
        !super::auth_probe_saturation_is_throttled(shared.as_ref(), Instant::now()),
        "clearing state must clear saturation state even after poison"
    );
}
