#!/usr/bin/env python3
"""Update the list of released versions of direct dependencies.

THE NETWORK LIVES HERE, NOT IN THE TEST RUN. A test hitting the network turns red on a train or an aeroplane,
and it is silenced first thing. Therefore crates.io is queried by this script - on command - while the guard
(`crates/qymcad/src/dependency_ratchet.rs`) checks against what is recorded here.

    python3 tools/check_deps.py            # show the gap
    python3 tools/check_deps.py --refresh  # query crates.io and update tools/deps.toml

The note describing what a dependency does in the program is written by a human: it is needed not by a machine,
but by whoever decides whether to raise a version or not. A new dependency without a note fails the guard on purpose.
"""

import json
import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEPS = ROOT / "tools" / "deps.toml"


def declared() -> dict[str, str]:
    """Direct dependencies from all manifests: name -> declared version."""
    out: dict[str, str] = {}
    for manifest in [ROOT / "Cargo.toml", *sorted((ROOT / "crates").glob("*/Cargo.toml"))]:
        text = manifest.read_text(encoding="utf-8")
        # ANY section ending in `dependencies]`, including platform-specific ones
        # (`[target.'cfg(windows)'.build-dependencies]`).
        for block in re.findall(r"^\[[^\]]*dependencies\]\n(.*?)(?=^\[|\Z)", text, re.S | re.M):
            for line in block.splitlines():
                line = line.split("#")[0].strip()
                m = re.match(r'^([a-zA-Z0-9_-]+)\s*=\s*(.+)$', line)
                if not m:
                    continue
                name, rhs = m.group(1), m.group(2)
                if "path =" in rhs or "workspace = true" in rhs or "workspace.dependencies" in rhs:
                    continue  # internal crates and workspace inheritance are handled separately
                v = re.search(r'version\s*=\s*"([^"]+)"', rhs) or re.match(r'^"([^"]+)"', rhs)
                if v:
                    out[name] = v.group(1)
    return out


def read_notes() -> dict[str, dict]:
    """Parse tools/deps.toml without external libraries: it is intentionally simple."""
    if not DEPS.exists():
        return {}
    notes, cur = {}, None
    for line in DEPS.read_text(encoding="utf-8").splitlines():
        line = line.split("#")[0].strip() if line.strip().startswith("#") else line
        m = re.match(r'^\[([a-zA-Z0-9_-]+)\]$', line.strip())
        if m:
            cur = m.group(1)
            notes[cur] = {}
            continue
        m = re.match(r'^([a-z_]+)\s*=\s*"(.*)"$', line.strip())
        if m and cur:
            notes[cur][m.group(1)] = m.group(2)
    return notes


def latest(name: str) -> str | None:
    url = f"https://crates.io/api/v1/crates/{name}"
    req = urllib.request.Request(url, headers={"User-Agent": "qymcad-dep-check (github.com/QymIs-Tech/QymCAD)"})
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return json.load(r)["crate"]["max_stable_version"]
    except Exception as e:  # network is not required to work
        print(f"  !! {name}: {e}", file=sys.stderr)
        return None


def gap(declared_v: str, latest_v: str) -> int:
    """Number of releases behind.

    For `0.x` versions, the SECOND number counts as a release: 0.29 -> 0.36 is seven releases, not zero.
    The Rust community itself treats it that way, and counting otherwise hides the largest lag.
    """
    d = [int(x) for x in re.findall(r"\d+", declared_v)[:2]] or [0]
    l = [int(x) for x in re.findall(r"\d+", latest_v)[:2]] or [0]
    d += [0] * (2 - len(d))
    l += [0] * (2 - len(l))
    if d[0] == 0 and l[0] == 0:
        return max(0, l[1] - d[1])
    return max(0, l[0] - d[0])


def main() -> int:
    refresh = "--refresh" in sys.argv
    have, notes = declared(), read_notes()

    if refresh:
        print(">>> querying crates.io")
        lines = [
            "# WHAT HAS BEEN RELEASED - a snapshot updated by `python3 tools/check_deps.py --refresh`.",
            "#",
            "# The `dependency_ratchet` guard checks against this file rather than the network: a test hitting",
            "# the network turns red on a train or an aeroplane, and it is silenced first thing.",
            "#",
            "# `what` is written by a HUMAN and the update leaves it untouched. The note is needed not by a machine,",
            "# but by whoever decides whether to raise a version or not.",
            "",
        ]
        for name in sorted(have):
            v = latest(name)
            what = notes.get(name, {}).get("what", "")
            lines.append(f"[{name}]")
            lines.append(f'latest = "{v or notes.get(name, {}).get("latest", "?")}"')
            lines.append(f'what = "{what}"')
            lines.append("")
        DEPS.write_text("\n".join(lines), encoding="utf-8")
        print(f">>> written to {DEPS.relative_to(ROOT)}")
        notes = read_notes()

    rows, total = [], 0
    for name in sorted(have):
        note = notes.get(name, {})
        lv = note.get("latest", "?")
        g = gap(have[name], lv) if lv != "?" else 0
        total += g
        rows.append((g, name, have[name], lv, note.get("what", "")))
    rows.sort(reverse=True)

    print(f"\n{'gap':>7}  {'crate':<22} {'declared':<10} {'latest':<10} purpose")
    for g, name, d, l, what in rows:
        mark = f"{g}" if g else "."
        print(f"{mark:>7}  {name:<22} {d:<10} {l:<10} {what}")
    print(f"\ntotal releases behind: {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
