#!/usr/bin/env python3
"""Guard: no third-party crate may be pinned to a local path in Cargo.lock.

A lockfile entry with no `source` key is path-resolved. For this workspace's own
members that is correct and expected. For anything else it means the lockfile
records a directory on one developer's machine, and the checkout is then
unresolvable anywhere else: `cargo build --locked` fails outright, and a build
without `--locked` silently re-resolves to whatever the registry has, which is a
different and usually older version of the crate.

This is not a hypothetical. Building against a local teksilo checkout, which
CONTRIBUTING describes, uses a gitignored `.cargo/config.toml` carrying

    [patch.crates-io]
    teksilo = { path = "../teksilo/crates/teksilo" }

and a `[patch]` rewrites the lockfile on EVERY cargo invocation, not just on an
explicit update. One `cargo check` drags teksilo's whole tree, and the
text-document and text-typeset trees behind it, into Cargo.lock as bare paths.
Thirty-two such entries reached a commit on this branch before anything noticed.

Restoring it is `git checkout -- Cargo.lock`, and it will happen again on the
next local build. That is the cost of the patch, and it is paid until the
framework is published and the patch is deleted.

Which is why this checks what is COMMITTED and what is STAGED, not what is in
the working tree. Inside a checkout with the patch active the working copy is
expected to be rewritten, and failing on that would make the gate unpassable and
therefore ignored. In CI the tree is clean, so HEAD is what is being tested
anyway. What must never be true is that a broken lockfile is recorded in git.

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


def patched_locally():
    """Whether a `[patch.crates-io]` is in force in this checkout."""
    config = REPO / ".cargo" / "config.toml"
    try:
        return "[patch.crates-io]" in config.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return False


def recorded(lockfile):
    """The copies worth checking, as (label, text) pairs.

    The staged copy always, because that is what a commit will record. The
    working copy only when no local patch is in force: with one, cargo rewrites
    it on every invocation, so failing on it would make the gate unpassable and
    therefore ignored. CI has no patch, so CI checks the file it checked out.

    HEAD is deliberately not checked. Its lockfile is correct relative to the
    manifests of that commit, and comparing it against today's workspace members
    reports a false violation the moment a package is renamed.
    """
    rel = lockfile.relative_to(REPO) if lockfile.is_relative_to(REPO) else lockfile
    out = []
    try:
        out.append(("staged", subprocess.run(
            ["git", "show", f":{rel.as_posix()}"],
            cwd=REPO, capture_output=True, text=True, check=True,
        ).stdout))
    except (OSError, subprocess.CalledProcessError):
        pass
    if not patched_locally() or not out:
        try:
            out.append(("on disk", lockfile.read_text(encoding="utf-8", errors="replace")))
        except OSError:
            pass

    seen, unique = set(), []
    for label, text in out:
        if text in seen:
            continue
        seen.add(text)
        unique.append((label, text))
    return unique


def path_resolved(text):
    """Every entry in a lockfile that has no `source`, by name."""
    out = []
    for block in PACKAGE.split(text)[1:]:
        name = NAME.search(block)
        if name and not SOURCE.search(block):
            out.append(name.group(1))
    return set(out)


def offenders(text, lockfile):
    """Path-resolved entries that should not be.

    Preferred rule: anything path-resolved that is not a member of that
    lockfile's workspace. When the members cannot be read, which is the normal
    state of the generated example workspace after a cleanup, fall back to
    comparing against the committed copy: a crate that was registry-resolved
    there and is path-resolved here has just been rewritten by the local patch,
    whatever the workspace turns out to contain. That fallback is narrower, but
    it catches the failure this guard exists for without needing cargo.
    """
    here = path_resolved(text)
    own = members(lockfile.parent)
    if own is not None:
        return sorted(here - own)

    rel = lockfile.relative_to(REPO) if lockfile.is_relative_to(REPO) else lockfile
    try:
        head = subprocess.run(
            ["git", "show", f"HEAD:{rel.as_posix()}"],
            cwd=REPO, capture_output=True, text=True, check=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        return None
    # Only entries that HEAD resolved from the registry and this one does not.
    # A member renamed since HEAD is path-resolved in both, so it does not show.
    return sorted(here - path_resolved(head))


def main(argv):
    targets = argv[1:] or [str(REPO / p) for p in DEFAULT_LOCKFILES]
    bad = False
    for target in targets:
        path = Path(target)
        if not path.is_file():
            print(f"{target}: missing", file=sys.stderr)
            bad = True
            continue
        rel = path.relative_to(REPO) if path.is_relative_to(REPO) else path
        for label, text in recorded(path):
            found = offenders(text, path)
            if found is None:
                print(f"{rel}: skipped, its workspace members are not on disk")
                break
            if not found:
                continue
            bad = True
            print(f"{rel} ({label}): {len(found)} path-resolved entry(ies) that are "
                  "not workspace members:")
            for name in found:
                print(f"    {name}")
            print("    This lockfile cannot resolve on any other machine.")
            print("    Usually the [patch.crates-io] in .cargo/config.toml, which")
            print("    rewrites the lockfile on every cargo invocation:")
            print(f"        git checkout -- {rel} && git add {rel}")
            print("    If a package was renamed, the lockfile is simply stale:")
            print("        cargo metadata >/dev/null   (with the patch moved aside)")
    if bad:
        return 1
    print("Lockfiles are portable: nothing recorded in git names a local path.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
