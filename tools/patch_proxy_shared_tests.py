#!/usr/bin/env python3
"""One-off batch patch: inject ProxySharedState into telemt proxy tests."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def ensure_use_proxy_shared(content: str) -> str:
    if "ProxySharedState" in content and "crate::proxy::ProxySharedState" in content:
        return content
    if "use crate::proxy::ProxySharedState" in content:
        return content
    lines = content.splitlines(keepends=True)
    insert_at = 0
    for i, line in enumerate(lines):
        if line.startswith("use "):
            insert_at = i + 1
    use_line = "use crate::proxy::ProxySharedState;\n"
    if use_line not in content:
        lines.insert(insert_at, use_line)
    return "".join(lines)


def inject_proxy_shared_in_tokio_tests(content: str, triggers: tuple[str, ...]) -> str:
    parts = content.split("#[tokio::test]")
    out = [parts[0]]
    for chunk in parts[1:]:
        block = "#[tokio::test]" + chunk
        if any(t in block for t in triggers):
            m = re.search(r"async fn\s+\w+[^{]*\{", block)
            if m and "let proxy_shared = ProxySharedState::new();" not in block[:400]:
                end = m.end()
                line_start = block.rfind("\n", 0, end) + 1
                indent = re.match(r"(\s*)", block[line_start:end]).group(1)
                ins = f"{indent}    let proxy_shared = ProxySharedState::new();\n"
                block = block[:end] + ins + block[end:]
        out.append(block)
    return "".join(out)


def patch_tls_handshake_args(content: str) -> str:
    return re.sub(
        r"(\s*)&replay_checker,\n\1&rng,\n\1None,",
        r"\1&replay_checker,\n\1proxy_shared.as_ref(),\n\1&rng,\n\1None,",
        content,
    )


def patch_tls_handshake_args_some_cache(content: str) -> str:
    return re.sub(
        r"(\s*)&replay_checker,\n\1&rng,\n\1Some\(",
        r"\1&replay_checker,\n\1proxy_shared.as_ref(),\n\1&rng,\n\1Some(",
        content,
    )


def patch_mtproto_handshake_args(content: str) -> str:
    return re.sub(
        r"(\s*)&replay_checker,\n\1(false|true),\n\1(None|Some\([^\)]*\)),",
        r"\1&replay_checker,\n\1proxy_shared.as_ref(),\n\1\2,\n\1\3,",
        content,
    )


def patch_handle_client_stream(content: str) -> str:
    return re.sub(
        r"(ip_tracker,\n)(\s*)(beobachten,\n)(\s*)(true|false),",
        r"\1\2\3\2proxy_shared.clone(),\n\4\5,",
        content,
    )


def patch_client_handler_new(content: str) -> str:
    return re.sub(
        r"(beobachten,\n)(\s*)(true|false),\n(\s*)(real_peer_report,)",
        r"\1\2proxy_shared.clone(),\n\3\4,\n\5",
        content,
    )


def process_handshake_test_file(path: Path) -> None:
    raw = path.read_text(encoding="utf-8")
    if "handle_tls_handshake" not in raw and "handle_mtproto_handshake" not in raw:
        return
    c = raw
    c = re.sub(r"fn auth_probe_test_guard\(\)[^{]*\{[^}]*\}\n\n", "", c, flags=re.DOTALL)
    c = re.sub(r"auth_probe_test_lock\(\)\s*\n\s*\.lock\(\)[^;]*;\n", "", c)
    c = c.replace("auth_probe_test_lock()", "()")
    c = re.sub(r"let _guard = auth_probe_test_lock\(\)[^;]*;\n", "", c)
    c = re.sub(r"let _probe_guard = auth_probe_test_lock\(\)[^;]*;\n", "", c)
    c = re.sub(r"let _\w+_guard = auth_probe_test_lock\(\)[^;]*;\n", "", c)
    c = re.sub(r"clear_auth_probe_state_for_testing\(\);\n", "", c)
    c = re.sub(r"AUTH_PROBE_STATE\.get\(\)\.map\(\|state\| state\.len\(\)\)\.unwrap_or\(0\)", "proxy_shared.auth_probe.len()", c)
    c = patch_tls_handshake_args(c)
    c = patch_tls_handshake_args_some_cache(c)
    c = patch_mtproto_handshake_args(c)
    c = ensure_use_proxy_shared(c)
    c = inject_proxy_shared_in_tokio_tests(
        c, ("handle_tls_handshake", "handle_mtproto_handshake")
    )
    if c != raw:
        path.write_text(c, encoding="utf-8")


def process_client_test_file(path: Path) -> None:
    raw = path.read_text(encoding="utf-8")
    if "handle_client_stream" not in raw and "ClientHandler::new" not in raw:
        return
    c = raw
    c = patch_handle_client_stream(c)
    c = patch_client_handler_new(c)
    c = ensure_use_proxy_shared(c)
    c = inject_proxy_shared_in_tokio_tests(c, ("handle_client_stream", "ClientHandler::new"))
    if c != raw:
        path.write_text(c, encoding="utf-8")


def main() -> None:
    tests = ROOT / "src" / "proxy" / "tests"
    for path in sorted(tests.glob("handshake*.rs")):
        process_handshake_test_file(path)
    for path in sorted(tests.glob("client*.rs")):
        process_client_test_file(path)


if __name__ == "__main__":
    main()
