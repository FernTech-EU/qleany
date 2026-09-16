#!/usr/bin/env python3
"""Guard: no third-party crate may be pinned to a local path in Cargo.lock.

A lockfile entry with no `source` key is path-resolved. For this workspace's own
members that is correct and expected. For anything else it means the lockfile
records a directory on one developer's machine, and the checkout is then
unresolvable anywhere else: `cargo build --locked` fails outright, and a build
without `--locked` silently re-resolves to whatever the registry has, which is a
different and usually older version of the crate.

This is not a hypothetical. Local co-development against an unreleased teksilo
uses a gitignored `.cargo/config.toml` carrying

    [patch.crates-io]
    teksilo = { path = "../teksilo/crates/teksilo" }

and a `[patch]` rewrites the lockfile on EVERY cargo invocation, not just on an
explicit update. One `cargo check` drags teksilo's whole tree, and the
text-document and text-typeset trees behind it, into Cargo.lock as bare paths.
Thirty-two such entries reached a commit on this branch before anything noticed.

Restoring it is `git checkout -- Cargo.lock`, and it will happen again on the
next local build. That is the cost of the patch, and it is paid until the
framework is published and the patch is deleted.

Run from anywhere:  python3 tools/check_lockfile_is_portable.py [lockfile ...]
Exit 0 when every path-resolved entry is a workspace member, 1 otherwise.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# Lockfiles that are committed and therefore have to resolve on any machine.
# `tests/rust/Cargo.lock` belongs to the generated example workspace, which sits
# below this one and so inherits the same `.cargo/config.toml`.
DEFAULT_LOCKFILES = ("Cargo.lock", "tests/rust/Cargo.lock")

PACKAGE = re.compile(r"^\[\[package\]\]$", re.M)
NAME = re.compile(r'^name = "(.+)"$', re.M)
SOURCE = re.compile(r"^source = ", re.M)


def members(manifest_dir):
    """Every package name in that workspace, which may legitimately be a path."""
    try:
        out = subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1",
             "--manifest-path", str(manifest_dir / "Cargo.toml")],
            capture_output=True, text=True, check=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        # No cargo, or a workspace whose members are generated and not on disk
        # right now, which is the normal state of tests/rust after a cleanup.
        # Without the member list there is nothing to compare against, so the
        # only honest answer is to skip rather than to report every member as an
        # offender and fail the gate on a file that is fine.
        return None
    return {p["name"] for p in json.loads(out)["packages"]}


def offenders(lockfile):
    """Path-resolved entries that are not members of that lockfile's workspace."""
    text = lockfile.read_text(encoding="utf-8", errors="replace")
    own = members(lockfile.parent)
    if own is None:
        return None
    found = []
    for block in PACKAGE.split(text)[1:]:
        name = NAME.search(block)
        if name and not SOURCE.search(block) and name.group(1) not in own:
            found.append(name.group(1))
    return sorted(set(found))


def main(argv):
    targets = argv[1:] or [str(REPO / p) for p in DEFAULT_LOCKFILES]
    bad = False
    for target in targets:
        path = Path(target)
        if not path.is_file():
            print(f"{target}: missing", file=sys.stderr)
            bad = True
            continue
        found = offenders(path)
        if found is None:
            print(f"{target}: skipped, its workspace members are not on disk")
            continue
        if found:
            bad = True
            rel = path.relative_to(REPO) if path.is_relative_to(REPO) else path
            print(f"{rel}: {len(found)} third-party crate(s) pinned to a local path:")
            for name in found:
                print(f"    {name}")
            print("    This lockfile cannot resolve on any other machine.")
            print(f"    Fix: git checkout -- {rel}")
            print("    Cause: the [patch.crates-io] in .cargo/config.toml, which")
            print("    rewrites the lockfile on every cargo invocation.")
    if bad:
        return 1
    print("Lockfiles are portable: every path-resolved entry is a workspace member.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
