# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Scratch space, and one rule: a live-app probe never opens a checked-in fixture.

Driving the real app means Ctrl+S, autosave and format migration all happen for
real, and any of them rewrites whatever file was passed on the command line —
silently corrupting the repository's fixture for the next run. The probe that
did it still passes, so nothing catches it; the next probe fails for a reason
that has nothing to do with the change being tested.

:func:`working_copy` hands back a throwaway in the scratch directory, and probes
open *that*. Cheap enough (a few MB) that there is no reason to skip it.
"""

from __future__ import annotations

import os
import shutil
import tempfile
from pathlib import Path

#: Point this at a session scratchpad to keep probe litter out of the system
#: temp dir. Never set it to anything inside the repository — :func:`working_copy`
#: refuses to hand back a path under the project root precisely because that is
#: the mistake this module exists to prevent.
SCRATCH_ENV = "TEKSILO_PROBE_SCRATCH"


def scratch_dir() -> Path:
    """The directory throwaway copies live in. Created if missing."""
    base = os.environ.get(SCRATCH_ENV) or tempfile.gettempdir()
    path = Path(base)
    path.mkdir(parents=True, exist_ok=True)
    return path


def scratch_path(name: str) -> Path:
    """A per-process path in the scratch directory.

    The pid is in the name so parallel probes — and successive runs of one
    probe — never collide on the same file.
    """
    return scratch_dir() / f"probe-{os.getpid()}-{name}"


def make_writable(path: str | os.PathLike) -> None:
    """Give the owner write permission, for a file or a whole tree.

    `shutil.copy2` preserves the source's mode, and a checked-in fixture is
    often kept read-only precisely so a stray write to it fails loudly. Without
    this the protection follows the copy, and the app then fails to save into
    its own scratch file — a confusing failure a long way from its cause.
    """
    def allow(target: str) -> None:
        try:
            os.chmod(target, os.stat(target).st_mode | 0o200)
        except OSError:
            pass

    path = os.fspath(path)
    allow(path)
    if os.path.isdir(path):
        for root, dirs, files in os.walk(path):
            for entry in dirs + files:
                allow(os.path.join(root, entry))


def working_copy(src: str | os.PathLike, *, label: str = "fixture",
                 guard_root: str | os.PathLike | None = None) -> str:
    """Copy `src` into the scratch directory and return the copy's path.

    Works for a file and for a directory alike, so a document that is a zip in
    one shape and an exploded folder in another needs no branch at the call
    site. The name carries `label` and the pid, so parallel probes never share.

    `guard_root` is the tree the copy must **not** land inside; it defaults to
    the Cargo project these probes belong to. The assertion is belt and braces:
    if this ever returned a path inside the repository, the probe is about to do
    the exact thing this module exists to prevent, and it should stop here
    rather than three minutes later with a dirty working tree.
    """
    source = Path(src).resolve()
    if not source.exists():
        raise FileNotFoundError(f"no fixture at {source}")

    stem = source.name or "fixture"
    destination = scratch_path(f"{label}-{stem}")
    if destination.is_dir():
        shutil.rmtree(destination)
    elif destination.exists():
        destination.unlink()

    if source.is_dir():
        shutil.copytree(source, destination)
    else:
        shutil.copy2(source, destination)
    make_writable(destination)

    if guard_root is None:
        from .resolve import project_dir

        guard_root = project_dir()
    # Both sides realpath'd, or the guard silently never fires: on macOS the
    # scratch dir is `/var/folders/...` and `resolve()` turns one side into
    # `/private/var/folders/...`, so a plain prefix test compares two spellings
    # of the same directory and always disagrees.
    root = os.path.realpath(os.fspath(guard_root))
    landed = os.path.realpath(destination)
    if landed == root or landed.startswith(root + os.sep):
        raise RuntimeError(
            f"the working copy landed inside the project ({destination}) — "
            f"refusing to hand it back. Set ${SCRATCH_ENV} to a directory "
            "outside the repository."
        )
    return str(destination)
