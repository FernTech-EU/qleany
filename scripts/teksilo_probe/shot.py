# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Screenshots: the MCP image block, decoded onto disk.

`screenshot` answers with two blocks — the PNG as base64 in an image block, and
a small metadata block. Read both. **The pixels are physical and every other
coordinate in the whole toolkit is logical**, so `scale` is the only thing that
lets a caller click what it can see::

    meta = save(session, "before.png")
    logical_x = pixel_x / meta["scale"]

A screenshot needs a GPU adapter. When there is none the tool comes back with
`GPU_UNAVAILABLE`, which is environmental and not a finding — :func:`try_save`
exists so a probe can note it and carry on rather than report a regression.
"""

from __future__ import annotations

import base64
from pathlib import Path
from typing import Any, Mapping

from .session import ToolError

#: The error code a host with no usable GPU adapter answers with.
GPU_UNAVAILABLE = "GPU_UNAVAILABLE"


def save(session: Any, path: str | Path, *, node: int | Mapping | None = None,
         window_id: int | None = None) -> dict:
    """Render the window (or one node's bounds) to `path`. Returns the metadata.

    Raises :class:`~teksilo_probe.session.ToolError` when the render fails —
    including `GPU_UNAVAILABLE`. Use :func:`try_save` where that is expected.
    """
    args: dict[str, Any] = {}
    if node is not None:
        args["node"] = node if isinstance(node, int) else node["id"]
    if window_id is not None:
        args["window_id"] = window_id
    result = session.call_result("screenshot", **args)

    png = _image_bytes(result.content)
    if png is None:
        raise ToolError("screenshot", "NO_IMAGE",
                        "the reply carried no image block — the tool answered, "
                        "but with nothing to save")
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(png)

    meta = result.payload if isinstance(result.payload, Mapping) else {}
    return dict(meta)


def try_save(session: Any, path: str | Path, *, node: int | Mapping | None = None,
             window_id: int | None = None) -> dict | None:
    """:func:`save`, returning `None` instead of raising on a render failure.

    A headless CI runner with no GPU adapter is the normal case here, not an
    exceptional one: the toolkit reports it as `GPU_UNAVAILABLE` precisely so a
    caller can tell it from a real `NOT_FOUND`.
    """
    try:
        return save(session, path, node=node, window_id=window_id)
    except ToolError:
        return None


def _image_bytes(content) -> bytes | None:
    """The first image block's decoded bytes, or `None`."""
    for block in content or []:
        if isinstance(block, Mapping) and block.get("type") == "image" and block.get("data"):
            try:
                return base64.b64decode(block["data"])
            except (ValueError, TypeError):
                return None
    return None
