# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Finding the app binary and the MCP client, and keeping their versions honest.

A probe that spells out `/home/someone/Devel/…` is a demonstration of a bug,
not a test of one: it passes on exactly one machine and cannot be run by CI, by
a contributor, or from a git worktree. Every path here is derived or
overridable.

**The version check is the part that earns its keep.** The client and the app
exchange a framed protocol that `teksilo_automation::client` says plainly is not
frozen — 0.9.3 replaced the socket announce with an endpoint descriptor and
bounded the token handshake. A client older than the app fails at *connect*
time with a symptom naming neither version, so the comparison has to happen
before anything is spawned.

**How the app's teksilo version is read is a fixed bug.** The prototype parsed
`[dependencies].teksilo` out of a crate manifest and understood a bare string
and `{ version = "…" }`. It returned `None` for
`teksilo = { workspace = true, features = [...] }` — the modern default — so the
check became a silent no-op and the stale-client protection it exists to provide
was, in the repository it shipped in, dead. `cargo metadata` resolves the graph
and answers with the version that will actually be linked, whatever shape the
manifest states it in.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

#: Override the app binary (an installed build, a release build, another
#: checkout). Absolute path; must exist.
APP_BIN_ENV = "TEKSILO_APP_BIN"

#: Override the automation MCP client binary.
MCP_BIN_ENV = "TEKSILO_MCP_BIN"

#: `1`/`yes` to install the MCP client without asking, `0`/`never` to refuse.
#: Unset means: ask, if there is a terminal to ask on.
MCP_AUTOINSTALL_ENV = "TEKSILO_MCP_AUTOINSTALL"

#: The client's crate and executable name.
MCP_BIN = "teksilo-automation-mcp"

_metadata_cache: dict[str, dict] = {}


class ResolveError(RuntimeError):
    """A binary, a manifest or a version could not be resolved."""


# ---------------------------------------------------------------------------
# The project
# ---------------------------------------------------------------------------


def project_dir(start: str | os.PathLike | None = None) -> Path:
    """The Cargo project these probes belong to.

    Walks up from the probe *file* rather than the working directory, so it is
    right however the probe was invoked — and, in a worktree checkout, it is
    that worktree's root. Guessing from the cwd would silently drive the main
    checkout's binary and report its behaviour as the worktree's.
    """
    here = Path(start or __file__).resolve()
    for candidate in (here, *here.parents):
        if (candidate / "Cargo.toml").is_file():
            return candidate
    return Path.cwd()


def cargo_metadata(project: str | os.PathLike | None = None) -> dict:
    """`cargo metadata --format-version 1`, cached per project directory.

    The full graph, not `--no-deps`: the point is to see the *resolved* version
    of a dependency the manifest may only name.
    """
    root = str(project_dir(project) if project is None else Path(project).resolve())
    if root in _metadata_cache:
        return _metadata_cache[root]
    cargo = shutil.which("cargo")
    if not cargo:
        raise ResolveError("cargo is not on $PATH, so the resolved graph cannot be read")
    done = subprocess.run(
        [cargo, "metadata", "--format-version", "1"],
        cwd=root, capture_output=True, text=True,
    )
    if done.returncode != 0:
        tail = "\n".join((done.stderr or done.stdout).splitlines()[-15:])
        raise ResolveError(f"`cargo metadata` failed in {root}:\n{tail}")
    data = json.loads(done.stdout)
    _metadata_cache[root] = data
    return data


def teksilo_version(project: str | os.PathLike | None = None) -> str | None:
    """The teksilo version this project **resolves**, or `None` if it has none.

    Read from `cargo metadata`'s package list, which is the resolved graph, so
    it answers correctly for `teksilo = "0.12"`, for
    `teksilo = { workspace = true }`, for a `[patch]`, and for a path
    dependency. Parsing `[dependencies].teksilo` out of a manifest — which is
    what this replaces — answered only the first of those and returned `None`
    for the rest, turning the version check into a no-op.

    `None` is not an error: an app may legitimately not depend on teksilo by
    that name (a workspace that renames it), and the caller then skips the
    comparison rather than refusing to run.
    """
    try:
        data = cargo_metadata(project)
    except (ResolveError, json.JSONDecodeError, OSError):
        return None
    versions = [
        pkg["version"]
        for pkg in data.get("packages", [])
        if pkg.get("name") == "teksilo" and pkg.get("version")
    ]
    if not versions:
        return None
    # More than one only happens with two major versions in one graph; the
    # newest is the one whose bridge protocol the client must be able to speak.
    return max(versions, key=version_tuple)


#: `major[.minor[.patch]][-prerelease][+build]`, anywhere in a string — so a
#: whole `--version` line can be handed over without pre-trimming.
_VERSION_RE = re.compile(r"(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:-([0-9A-Za-z.\-]+))?")


def version_tuple(text: str | None) -> tuple[int, ...]:
    """`"0.9.4"` -> `(0, 9, 4, 1)`, comparable with `<`.

    The trailing element is a **pre-release rank**: 0 for `0.10.0-rc.1`, 1 for
    `0.10.0`, so a release always outranks its own pre-releases. Splitting on
    `.` and taking the leading digits of each chunk — which is the obvious
    implementation, and the one the prototype used — reads `0.10.0-rc.1` as
    `(0, 10, 0, 1)` and therefore as *newer* than `0.10.0`, which is exactly
    backwards and would wave through a client the check exists to refuse.

    An unparseable string is `()`, which compares below everything.
    """
    found = _VERSION_RE.search(str(text or ""))
    if not found:
        return ()
    major, minor, patch, prerelease = found.groups()
    return (int(major), int(minor or 0), int(patch or 0), 0 if prerelease else 1)


# ---------------------------------------------------------------------------
# Binaries
# ---------------------------------------------------------------------------


def target_dirs(project: str | os.PathLike | None = None) -> list[Path]:
    """Cargo's output directories, honouring `$CARGO_TARGET_DIR`.

    Checked first, because a contributor or CI runner that redirects the target
    directory has a real binary on disk that `<project>/target` never finds —
    and the resulting "build it with cargo build" message would send them to
    rebuild something they had already built.
    """
    root = project_dir(project) if project is None else Path(project)
    dirs: list[Path] = []
    override = os.environ.get("CARGO_TARGET_DIR")
    if override:
        dirs.append(Path(override))
    try:
        directory = cargo_metadata(root).get("target_directory")
        if directory:
            dirs.append(Path(directory))
    except (ResolveError, json.JSONDecodeError, OSError):
        pass
    dirs.append(Path(root) / "target")
    seen: list[Path] = []
    for path in dirs:
        if path not in seen:
            seen.append(path)
    return seen


def _exe(name: str) -> str:
    return f"{name}.exe" if sys.platform == "win32" else name


def app_bin_names(project: str | os.PathLike | None = None) -> list[str]:
    """Every runnable binary this workspace's own members declare."""
    try:
        data = cargo_metadata(project)
    except (ResolveError, json.JSONDecodeError, OSError):
        return []
    members = set(data.get("workspace_members") or [])
    names: list[str] = []
    for pkg in data.get("packages", []):
        if members and pkg.get("id") not in members:
            continue
        for target in pkg.get("targets", []):
            if "bin" in (target.get("kind") or []) and target.get("name") not in names:
                names.append(target["name"])
    return names


def app_binary(name: str | None = None, *, project: str | os.PathLike | None = None,
               profile: str = "debug") -> str:
    """Path to the app's built binary, or `$TEKSILO_APP_BIN`.

    `debug` and not `release` on purpose: the automation bridge every probe
    drives is `#[cfg(debug_assertions)]`, so a release binary has no endpoint to
    attach to and the probe would wait out its whole timeout for a bridge that
    can never exist.

    With no `name`, the workspace's binary targets are read from `cargo
    metadata`; exactly one is used, and several are refused by name rather than
    guessed at.
    """
    override = os.environ.get(APP_BIN_ENV)
    if override:
        if not os.path.exists(override):
            raise ResolveError(f"${APP_BIN_ENV} points at {override}, which does not exist")
        return override

    candidates = [name] if name else app_bin_names(project)
    if not candidates:
        raise ResolveError(
            "no binary target found in this workspace. Pass `name=` or set "
            f"${APP_BIN_ENV}."
        )
    if len(candidates) > 1:
        raise ResolveError(
            "this workspace declares several binaries (" + ", ".join(candidates) +
            "). Pass the one to drive as `name=`, or set " + f"${APP_BIN_ENV}."
        )

    binary = _exe(candidates[0])
    tried = [Path(target) / profile / binary for target in target_dirs(project)]
    for path in tried:
        if path.exists():
            return str(path)
    listing = "\n  ".join(str(p) for p in tried)
    raise ResolveError(
        f"cannot find the `{candidates[0]}` {profile} binary. Tried:\n  {listing}\n"
        f"Build it with:\n  cargo build\n"
        f"or set ${APP_BIN_ENV} to an existing binary. "
        "It must be a **debug** build: the automation bridge is "
        "`#[cfg(debug_assertions)]`."
    )


def mcp_install_command(version: str | None = None) -> list[str]:
    """The command that puts the MCP client on `$PATH`.

    `--locked` on purpose, and not this module's idea: the client and the app
    exchange framed JSON, and a dependency resolved differently on the two sides
    is one more way for them to disagree about the wire.
    `teksilo_automation::client::install_command` composes the same string from
    the toolkit's own version; this is that command, reachable *before* the app
    has been launched — which is when a probe needs it, since it must spawn the
    client itself.
    """
    cmd = ["cargo", "install", MCP_BIN]
    if version:
        cmd += ["--version", version]
    return cmd + ["--locked"]


def mcp_version(binary: str | None = None) -> str | None:
    """The MCP client's own version, or `None` if it will not say.

    Deliberately tolerant: a client that cannot be asked is not a reason to
    refuse to run, only a reason to skip the comparison.
    """
    binary = binary or os.environ.get(MCP_BIN_ENV) or shutil.which(MCP_BIN)
    if not binary:
        return None
    try:
        out = subprocess.run([binary, "--version"], capture_output=True,
                             text=True, timeout=30).stdout
    except (OSError, subprocess.SubprocessError):
        return None
    found = re.search(r"(\d+\.\d+\.\d+\S*)", out or "")
    return found.group(1) if found else None


def _wants_install(prompt: str) -> bool:
    """Ask whether to install. The env var wins; otherwise ask, if anyone can answer.

    A probe run by CI or by an agent has no terminal, and a prompt there is not a
    question, it is a hang. So the absence of a TTY means "no" and the caller
    raises with the command spelled out instead.
    """
    choice = os.environ.get(MCP_AUTOINSTALL_ENV, "").strip().lower()
    if choice in ("1", "y", "yes", "true", "always"):
        return True
    if choice in ("0", "n", "no", "false", "never"):
        return False
    try:
        if not sys.stdin.isatty():
            return False
        return input(prompt).strip().lower() in ("y", "yes")
    except (OSError, EOFError, KeyboardInterrupt):
        return False


def install_mcp(version: str | None = None) -> str:
    """Run `cargo install teksilo-automation-mcp` and return the resulting path."""
    cmd = mcp_install_command(version)
    if not shutil.which("cargo"):
        raise ResolveError(
            f"cargo is not on $PATH, so `{MCP_BIN}` cannot be installed "
            f"automatically. Install Rust, then run:\n  {' '.join(cmd)}"
        )
    print(f"installing : {' '.join(cmd)}\n  (this builds the client once; it takes a "
          "couple of minutes and then never again)")
    done = subprocess.run(cmd, capture_output=True, text=True)
    if done.returncode != 0:
        tail = "\n".join((done.stderr or done.stdout).splitlines()[-20:])
        raise ResolveError(f"`{' '.join(cmd)}` failed (exit {done.returncode}):\n{tail}")
    found = shutil.which(MCP_BIN)
    if not found:
        raise ResolveError(
            f"`{' '.join(cmd)}` reported success but the binary is still not on "
            "$PATH. Is cargo's bin directory (usually ~/.cargo/bin) on it?"
        )
    print(f"installed  : {found}")
    return found


def check_mcp_version(binary: str, want: str | None, install_cmd: str) -> None:
    """Refuse a client older than the teksilo the app resolves.

    Silent when either version is unknown — an unanswerable question is not a
    failure. The rule itself belongs to `teksilo_automation::client`; it is
    enforced here because this is what spawns the client.
    """
    if not want:
        return
    have = mcp_version(binary)
    if not have or version_tuple(have) >= version_tuple(want):
        return
    problem = (
        f"`{binary}` is {MCP_BIN} {have}, but this app resolves teksilo {want}, "
        "whose bridge protocol it predates. It would fail at connect time, with "
        "a symptom naming neither version."
    )
    if _wants_install(f"\n{problem}\n  Update it now with `{install_cmd}`? [y/N] "):
        install_mcp(want)
        return
    raise ResolveError(
        f"{problem}\nUpdate it with:\n  {install_cmd}\n"
        f"or point ${MCP_BIN_ENV} at a build of {want} or newer."
    )


def mcp_binary(*, project: str | os.PathLike | None = None,
               check_version: bool = True) -> str:
    """Path to `teksilo-automation-mcp`, installing it if it is missing.

    **An app needs nothing installed to be automatable** — the bridge is
    compiled into its debug build. This binary is the *client* half, which turns
    the MCP-over-stdio an agent speaks into the app's framed protocol. That
    asymmetry is easy to get wrong from the outside, which is why the app's own
    startup announce says it too.

    Resolution order, `$PATH` first:

    1. ``$TEKSILO_MCP_BIN`` — an explicit override, which must exist.
    2. ``$PATH`` — where `cargo install` puts it, and the supported route.
    3. Nothing found — offer to install, or raise naming the command.
    """
    want = teksilo_version(project)
    command = " ".join(mcp_install_command(want))

    override = os.environ.get(MCP_BIN_ENV)
    if override:
        if not os.path.exists(override):
            raise ResolveError(f"${MCP_BIN_ENV} points at {override}, which does not exist")
        if check_version:
            check_mcp_version(override, want, command)
        return override

    found = shutil.which(MCP_BIN)
    if not found:
        if _wants_install(
            f"\n`{MCP_BIN}` is not installed.\n  Install it now with `{command}`? [y/N] "
        ):
            found = install_mcp(want)
        else:
            raise ResolveError(
                f"cannot find `{MCP_BIN}`, the MCP client these probes drive the "
                f"app through.\nInstall it with:\n  {command}\nthen re-run. "
                f"(Set ${MCP_AUTOINSTALL_ENV}=1 to have this do it for you, or "
                f"${MCP_BIN_ENV} to point at an existing build.)"
            )

    if check_version:
        check_mcp_version(found, want, command)
    return found
