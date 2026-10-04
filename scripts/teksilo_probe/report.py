# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Recording what a probe observed, and turning it into an exit code.

Three outcomes, not two, and the distinction is the point:

===== ==================================================================
  0   every check passed.
  1   **the probe could not run** — the app did not launch, the bridge
      never appeared, a tool call failed, a fixture was missing. Nothing
      was learned about the app's behaviour.
  2   **the probe ran cleanly and the behaviour was absent** — every call
      succeeded and a check came back false.
===== ==================================================================

Collapsing 1 and 2 into "failed" is what makes a probe suite untrustworthy: a
run that exits 1 because the binary was not built looks identical to one that
exits 1 because the feature regressed, so the first teaches the reader to
distrust the second. Keeping them apart means a `2` is always a real finding.
"""

from __future__ import annotations

import sys
from typing import Any

#: Exit codes, named so a caller never writes the integers.
EXIT_PASS = 0
EXIT_ERROR = 1
EXIT_ABSENT = 2


class Report:
    """Checks, notes and errors for one probe run.

    ::

        report = Report("rail action")
        report.check(cog["role"] == "Button", "the cog is a Button, not a Tab")
        raise SystemExit(report.finish())
    """

    def __init__(self, title: str | None = None, *, stream=None) -> None:
        self.title = title
        self.stream = stream if stream is not None else sys.stdout
        self.passed: list[str] = []
        self.failed: list[str] = []
        self.errors: list[str] = []
        self.notes: list[str] = []
        if title:
            self._write(f"== {title} ==")

    def _write(self, text: str) -> None:
        print(text, file=self.stream)

    # -- Recording --------------------------------------------------------

    def check(self, ok: Any, msg: str) -> bool:
        """Record one observation about the app's behaviour.

        A false check means the behaviour is **absent**, not that the probe
        broke — so it steers the run towards exit 2. Use :meth:`error` for the
        latter.
        """
        ok = bool(ok)
        (self.passed if ok else self.failed).append(msg)
        self._write(("  ok    " if ok else "  FAIL  ") + msg)
        return ok

    def error(self, msg: str) -> None:
        """Record that the probe could not observe what it came to observe.

        Exit 1 — nothing was learned. A missing binary, a bridge that never
        appeared, a tool that returned `NOT_FOUND` for a node the probe needs to
        exist before it can start.
        """
        self.errors.append(msg)
        self._write("  ERROR " + msg)

    def note(self, msg: str) -> None:
        """Context for a reader. Never affects the exit code."""
        self.notes.append(msg)
        self._write("  note  " + msg)

    # -- Finishing --------------------------------------------------------

    @property
    def exit_code(self) -> int:
        if self.errors:
            return EXIT_ERROR
        return EXIT_ABSENT if self.failed else EXIT_PASS

    def summary(self) -> str:
        if self.errors:
            head = f"ERROR ({len(self.errors)}) — the probe could not run"
            body = self.errors + self.failed
        elif self.failed:
            head = f"ABSENT ({len(self.failed)}) — the probe ran; the behaviour is not there"
            body = self.failed
        else:
            head = f"PASS ({len(self.passed)} checks)"
            body = []
        lines = [head] + [f"  - {item}" for item in body]
        return "\n".join(lines)

    def finish(self) -> int:
        """Print the summary and return the exit code."""
        self._write("")
        self._write(self.summary())
        return self.exit_code

    def exit(self) -> "None":  # pragma: no cover - trivially `SystemExit`
        """`finish()` and leave."""
        raise SystemExit(self.finish())
