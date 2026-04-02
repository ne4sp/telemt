use super::*;
use crate::proxy::ProxySharedState;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

#[test]
fn desync_all_full_bypass_does_not_initialize_or_grow_dedup_cache() {
    let shared = ProxySharedState::new();
    let initial_len = shared.desync_dedup.len();
    let now = Instant::now();

    for i in 0..20_000u64 {
        assert!(
            super::should_emit_full_desync(shared.as_ref(), 0xD35E_D000_0000_0000u64 ^ i, true, now),
            "desync_all_full path must always emit"
        );
    }

    let after_len = shared.desync_dedup.len();
    assert_eq!(
        after_len, initial_len,
        "desync_all_full bypass must not allocate or accumulate dedup entries"
    );
}

#[test]
fn desync_all_full_bypass_keeps_existing_dedup_entries_unchanged() {
    let shared = ProxySharedState::new();
    let seed_time = Instant::now() - Duration::from_secs(7);
    shared.desync_dedup.insert(0xAAAABBBBCCCCDDDD, seed_time);
    shared.desync_dedup.insert(0x1111222233334444, seed_time);

    let now = Instant::now();
    for i in 0..2048u64 {
        assert!(
            super::should_emit_full_desync(shared.as_ref(), 0xF011_F000_0000_0000u64 ^ i, true, now),
            "desync_all_full must bypass suppression and dedup refresh"
        );
    }

    assert_eq!(
        shared.desync_dedup.len(),
        2,
        "bypass path must not mutate dedup cardinality"
    );
    assert_eq!(
        *shared
            .desync_dedup
            .get(&0xAAAABBBBCCCCDDDD)
            .expect("seed key must remain"),
        seed_time,
        "bypass path must not refresh existing dedup timestamps"
    );
    assert_eq!(
        *shared
            .desync_dedup
            .get(&0x1111222233334444)
            .expect("seed key must remain"),
        seed_time,
        "bypass path must not touch unrelated dedup entries"
    );
}

#[test]
fn edge_all_full_burst_does_not_poison_later_false_path_tracking() {
    let shared = ProxySharedState::new();
    let now = Instant::now();
    for i in 0..8192u64 {
        assert!(super::should_emit_full_desync(
            shared.as_ref(),
            0xABCD_0000_0000_0000 ^ i,
            true,
            now
        ));
    }

    let tracked_key = 0xDEAD_BEEF_0000_0001u64;
    assert!(
        super::should_emit_full_desync(shared.as_ref(), tracked_key, false, now),
        "first false-path event after all_full burst must still be tracked and emitted"
    );

    assert!(shared.desync_dedup.get(&tracked_key).is_some());
}

#[test]
fn adversarial_mixed_sequence_true_steps_never_change_cache_len() {
    let shared = ProxySharedState::new();
    for i in 0..256u64 {
        shared
            .desync_dedup
            .insert(0x1000_0000_0000_0000 ^ i, Instant::now());
    }

    let mut seed = 0xC0DE_CAFE_BAAD_F00Du64;
    for i in 0..4096u64 {
        seed ^= seed << 7;
        seed ^= seed >> 9;
        seed ^= seed << 8;

        let flag_all_full = (seed & 0x1) == 1;
        let key = 0x7000_0000_0000_0000u64 ^ i ^ seed;
        let before = shared.desync_dedup.len();
        let _ = super::should_emit_full_desync(shared.as_ref(), key, flag_all_full, Instant::now());
        let after = shared.desync_dedup.len();

        if flag_all_full {
            assert_eq!(after, before, "all_full step must not mutate dedup length");
        }
    }
}

#[test]
fn light_fuzz_all_full_mode_always_emits_and_stays_bounded() {
    let shared = ProxySharedState::new();
    let mut seed = 0x1234_5678_9ABC_DEF0u64;
    let before = shared.desync_dedup.len();

    for _ in 0..20_000 {
        seed ^= seed << 7;
        seed ^= seed >> 9;
        seed ^= seed << 8;
        let key = seed ^ 0x55AA_55AA_55AA_55AAu64;
        assert!(super::should_emit_full_desync(shared.as_ref(), key, true, Instant::now()));
    }

    let after = shared.desync_dedup.len();
    assert_eq!(after, before);
    assert!(after <= DESYNC_DEDUP_MAX_ENTRIES);
}

#[test]
fn stress_parallel_all_full_storm_does_not_grow_or_mutate_cache() {
    let shared = ProxySharedState::new();
    let seed_time = Instant::now() - Duration::from_secs(2);
    for i in 0..1024u64 {
        shared
            .desync_dedup
            .insert(0x8888_0000_0000_0000 ^ i, seed_time);
    }
    let before_len = shared.desync_dedup.len();

    let emits = Arc::new(AtomicUsize::new(0));
    let mut workers = Vec::new();
    for worker in 0..16u64 {
        let emits = Arc::clone(&emits);
        let shared = Arc::clone(&shared);
        workers.push(thread::spawn(move || {
            let now = Instant::now();
            for i in 0..4096u64 {
                let key = 0xFACE_0000_0000_0000u64 ^ (worker << 20) ^ i;
                if super::should_emit_full_desync(shared.as_ref(), key, true, now) {
                    emits.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    for worker in workers {
        worker.join().expect("worker must not panic");
    }

    assert_eq!(emits.load(Ordering::Relaxed), 16 * 4096);
    assert_eq!(
        shared.desync_dedup.len(),
        before_len,
        "parallel all_full storm must not mutate cache len"
    );
}
