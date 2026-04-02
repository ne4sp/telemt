"""Insert ProxySharedState::new() before last bool in handle_client_stream call sites."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "src" / "proxy" / "tests"

PAT1 = re.compile(
    r"(Arc::new\(BeobachtenStore::new\(\)\),)\n(\s*)(false|true),(\s*\)\))",
    re.MULTILINE,
)
PAT_HARNESS = re.compile(
    r"(harness\.beobachten,)\n(\s*)(false|true),(\s*\)\)?;?)",
    re.MULTILINE,
)


def repl_beob(m: re.Match[str]) -> str:
    indent = m.group(2)
    return (
        f"{m.group(1)}\n{indent}ProxySharedState::new(),\n"
        f"{indent}{m.group(3)},{m.group(4)}"
    )


def ensure_import(text: str) -> str:
    if "ProxySharedState::new()" not in text:
        return text
    if "use crate::proxy::ProxySharedState" in text:
        return text
    if text.startswith("use super::*;"):
        return text.replace(
            "use super::*;",
            "use super::*;\nuse crate::proxy::ProxySharedState;",
            1,
        )
    if text.startswith("//!"):
        idx = text.find("\n\n")
        if idx != -1:
            return text[: idx + 2] + "use crate::proxy::ProxySharedState;\n" + text[idx + 2 :]
    return "use crate::proxy::ProxySharedState;\n" + text


def main() -> None:
    changed: list[Path] = []
    for path in sorted(ROOT.rglob("*.rs")):
        orig = path.read_text(encoding="utf-8")
        t = PAT1.sub(repl_beob, orig)
        t = PAT_HARNESS.sub(repl_beob, t)
        if t != orig:
            t = ensure_import(t)
            path.write_text(t, encoding="utf-8")
            changed.append(path)
    print("updated", len(changed), "files")
    for p in changed:
        print(p.relative_to(ROOT.parent.parent.parent.parent))


if __name__ == "__main__":
    main()
