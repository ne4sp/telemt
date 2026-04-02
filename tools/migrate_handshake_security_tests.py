"""Migrate handshake_security_tests.rs from globals to ProxySharedState."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PATH = ROOT / "src" / "proxy" / "tests" / "handshake_security_tests.rs"


def main() -> None:
    t = PATH.read_text(encoding="utf-8")

    if "use crate::proxy::ProxySharedState;" not in t:
        t = t.replace(
            "use super::*;\n",
            "use super::*;\nuse crate::proxy::ProxySharedState;\n",
        )

    helper = """
fn auth_probe_fail_streak_test(shared: &ProxySharedState, ip: std::net::IpAddr) -> Option<u32> {
    shared
        .auth_probe
        .get(&super::normalize_auth_probe_ip(ip))
        .map(|e| e.fail_streak)
}

"""
    insert_at = t.find("fn make_valid_tls_handshake")
    if insert_at != -1 and "auth_probe_fail_streak_test" not in t:
        t = t[:insert_at] + helper + t[insert_at:]

    t = re.sub(
        r"auth_probe_fail_streak_for_testing\(",
        "auth_probe_fail_streak_test(shared.as_ref(), ",
        t,
    )

    t = re.sub(
        r"let state = DashMap::new\(\);",
        "let shared = ProxySharedState::new();\n    let state = &shared.auth_probe;",
        t,
    )

    t = t.replace(
        "auth_probe_record_failure_with_state(&state,",
        "super::auth_probe_record_failure_with_state(shared.as_ref(),",
    )

    t = t.replace(
        "auth_probe_saturation_is_throttled_at_for_testing(",
        "super::auth_probe_saturation_is_throttled(shared.as_ref(), ",
    )

    t = re.sub(
        r"auth_probe_eviction_offset\(",
        "super::auth_probe_eviction_offset(shared.as_ref(), ",
        t,
    )

    t = re.sub(r"clear_auth_probe_state_for_testing\(\);\n", "", t)
    t = re.sub(r"let _probe_guard = auth_probe_test_lock\(\)[^;]*;\n", "", t)
    t = re.sub(r"let _guard = auth_probe_test_lock\(\)[^;]*;\n", "", t)
    t = re.sub(
        r"let _guard = auth_probe_test_lock\(\)\s*\n\s*\.lock\(\)[^;]*;\n", "", t
    )

    t = re.sub(
        r"(\s*)&replay_checker,\n\1&rng,\n\1None,",
        r"\1&replay_checker,\n\1shared.as_ref(),\n\1&rng,\n\1None,",
        t,
    )
    t = re.sub(
        r"(\s*)&replay_checker,\n\1&rng,\n\1Some\(",
        r"\1&replay_checker,\n\1shared.as_ref(),\n\1&rng,\n\1Some(",
        t,
    )
    t = re.sub(
        r"(\s*)&replay_checker,\n\1(false|true),\n\1(None|Some\([^\)]*\)),",
        r"\1&replay_checker,\n\1shared.as_ref(),\n\1\2,\n\1\3,",
        t,
    )

    parts = t.split("#[tokio::test")
    out = [parts[0]]
    for chunk in parts[1:]:
        block = "#[tokio::test" + chunk
        if (
            "handle_tls_handshake" in block
            or "handle_mtproto_handshake" in block
            or "auth_probe_fail_streak_test(shared.as_ref()" in block
        ):
            if "let shared = ProxySharedState::new();" in block[:800]:
                out.append(block)
                continue
            m = re.search(r"async fn\s+\w+[^{]*\{", block)
            if m:
                end = m.end()
                line_start = block.rfind("\n", 0, end) + 1
                indent = re.match(r"(\s*)", block[line_start:end]).group(1)
                ins = f"{indent}    let shared = ProxySharedState::new();\n"
                block = block[:end] + ins + block[end:]
        out.append(block)
    t = "".join(out)

    t = t.replace(
        "auth_probe_record_failure(peer_ip,",
        "super::auth_probe_record_failure(shared.as_ref(), peer_ip,",
    )

    PATH.write_text(t, encoding="utf-8")
    print("written", PATH)


if __name__ == "__main__":
    main()
