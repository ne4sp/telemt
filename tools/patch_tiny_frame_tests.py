from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def patch_tiny(path: Path) -> None:
    t = path.read_text(encoding="utf-8")
    if "use crate::proxy::ProxySharedState;" not in t:
        t = t.replace("use super::*;\n", "use super::*;\nuse crate::proxy::ProxySharedState;\n")
    t = t.replace(
        """fn make_forensics(conn_id: u64, started_at: Instant) -> RelayForensicsState {
    RelayForensicsState {
        trace_id: 0xB100_0000 + conn_id,
        conn_id,
        user: format!("tiny-frame-debt-user-{conn_id}"),
        peer: "127.0.0.1:50000".parse().expect("peer parse must succeed"),
        peer_hash: hash_ip("127.0.0.1".parse().expect("ip parse must succeed")),
        started_at,
        bytes_c2me: 0,
        bytes_me2c: Arc::new(AtomicU64::new(0)),
        desync_all_full: false,
    }
}""",
        """fn make_forensics(
    shared: &ProxySharedState,
    conn_id: u64,
    started_at: Instant,
) -> RelayForensicsState {
    let peer_ip: std::net::IpAddr = "127.0.0.1".parse().expect("ip parse must succeed");
    RelayForensicsState {
        trace_id: 0xB100_0000 + conn_id,
        conn_id,
        user: format!("tiny-frame-debt-user-{conn_id}"),
        peer: "127.0.0.1:50000".parse().expect("peer parse must succeed"),
        peer_hash: super::hash_ip(shared, peer_ip),
        started_at,
        bytes_c2me: 0,
        bytes_me2c: Arc::new(AtomicU64::new(0)),
        desync_all_full: false,
    }
}""",
    )
    t = t.replace(
        """    session_started_at: Instant,
) -> Result<Option<(PooledBuffer, bool)>> {
    run_relay_test_step_timeout(
        "tiny-frame debt read step",
        read_client_payload_with_idle_policy(
            crypto_reader,
            proto_tag,
            1024,
            buffer_pool,
            forensics,
            frame_counter,
            stats,
            idle_policy,
            idle_state,
            last_downstream_activity_ms,
            session_started_at,
        ),
    )
    .await
}""",
        """    session_started_at: Instant,
    proxy_shared: &ProxySharedState,
) -> Result<Option<(PooledBuffer, bool)>> {
    run_relay_test_step_timeout(
        "tiny-frame debt read step",
        read_client_payload_with_idle_policy(
            crypto_reader,
            proto_tag,
            1024,
            buffer_pool,
            forensics,
            frame_counter,
            stats,
            idle_policy,
            idle_state,
            last_downstream_activity_ms,
            session_started_at,
            proxy_shared,
        ),
    )
    .await
}""",
    )
    t = re.sub(
        r"make_forensics\((\d+),\s*",
        r"make_forensics(proxy_shared.as_ref(), \1, ",
        t,
    )
    t = re.sub(
        r"make_forensics\(500 \+ case_idx,\s*",
        r"make_forensics(proxy_shared.as_ref(), 500 + case_idx, ",
        t,
    )
    t = re.sub(
        r"(\nasync fn [^{]+\{\n)",
        r"\1    let proxy_shared = ProxySharedState::new();\n",
        t,
    )
    t = re.sub(
        r"(read_bounded\([\s\S]*?session_started_at,)\n(\s*)\)",
        r"\1\n\2    proxy_shared.as_ref(),\n\2)",
        t,
    )
    path.write_text(t, encoding="utf-8")


def main() -> None:
    tests = ROOT / "src" / "proxy" / "tests"
    patch_tiny(tests / "middle_relay_tiny_frame_debt_security_tests.rs")
    print("patched middle_relay_tiny_frame_debt_security_tests.rs")


if __name__ == "__main__":
    main()
