# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 FernTech

"""Live pictures: aim at what a `LiveImage` shows, and read back what it drew.

A `LiveImage` fits, letterboxes and turns the picture its producer writes, so a
point of the window and a pixel of the source are two different things, and
the mapping between them changes with the window's size, its scale and the
picture's orientation. A probe written in window points is a probe that breaks
when the window is resized. These helpers speak in source pixels instead::

    from teksilo_probe import live_image

    live_image.click_source(session, vm, 10, 20)     # press on source pixel (10, 20)
    live_image.source_of(session, vm, (x, y))        # the source pixel at a window point
    picture = live_image.capture(session, vm)        # the picture a screenshot drew
    picture.pixel(0, 0), picture.generation          # its pixels, and which commit

A picture is cut by the screenshot's own record of where it drew each live
picture and which commit it drew (`live_images` in its metadata), not by
`live_image_stats` or `live_image_map`: a producer keeps committing while the
probe runs, so asking afterwards races with it, and a screenshot of one node
does not say where in the window its image starts. :func:`crop_visible` does
the cutting on a PNG already saved; :func:`capture` takes the screenshot too.

A screenshot needs a GPU adapter. Without one :func:`capture` raises
:class:`~teksilo_probe.session.ToolError` with `GPU_UNAVAILABLE`, as
:func:`teksilo_probe.shot.save` does.

PNG reading and writing are here, standard library only, for the one format the
toolkit writes: 8-bit RGBA (or RGB), not interlaced.
"""

from __future__ import annotations

import struct
import zlib
from pathlib import Path
from typing import Any, Mapping

from . import tools
from .session import ToolError
from .shot import _image_bytes

__all__ = [
    "Picture",
    "click_source",
    "point_of",
    "rect_of",
    "source_of",
    "crop_visible",
    "capture",
    "decode_png",
    "encode_png",
]


class Picture:
    """The part of a screenshot a live picture covers.

    `rgba` holds `width × height` pixels, four bytes each, row by row, in the
    screenshot's **physical** pixels; `scale` relates them to logical points.
    `generation` is the commit of the source the screenshot drew, and
    `deferred` says it drew an older picture or only the background (a size
    newer than the window's last layout, or nothing uploaded yet).
    """

    __slots__ = ("width", "height", "rgba", "generation", "deferred", "scale")

    def __init__(self, width: int, height: int, rgba: bytes, generation: int,
                 deferred: bool, scale: float) -> None:
        if len(rgba) != width * height * 4:
            raise ValueError(f"{len(rgba)} bytes for a {width}x{height} picture")
        self.width = width
        self.height = height
        self.rgba = rgba
        self.generation = generation
        self.deferred = deferred
        self.scale = scale

    def pixel(self, x: int, y: int) -> tuple[int, int, int, int]:
        """The `(r, g, b, a)` of the pixel at column `x`, row `y`."""
        if not (0 <= x < self.width and 0 <= y < self.height):
            raise IndexError(f"({x}, {y}) is outside a {self.width}x{self.height} picture")
        i = (y * self.width + x) * 4
        r, g, b, a = self.rgba[i:i + 4]
        return (r, g, b, a)

    def png(self) -> bytes:
        """The picture as a PNG file's bytes."""
        return encode_png(self.width, self.height, self.rgba)

    def save(self, path: str | Path) -> Path:
        """Write the picture to `path` as a PNG, creating its directory."""
        target = Path(path)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(self.png())
        return target

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        state = " deferred" if self.deferred else ""
        return f"<Picture {self.width}x{self.height} generation {self.generation}{state}>"


def click_source(session: Any, node: int | Mapping, x: int, y: int, *,
                 action: str = "click", window_id: int | None = None,
                 **pointer: Any) -> Any:
    """Press on the centre of source pixel `(x, y)` of the `LiveImage` `node`.

    `action` and the other keyword arguments are `inject_pointer`'s (`kind`,
    `button`, `ctrl`, `command`, ...). The tool refuses a pixel outside the
    source, or one a press would not reach (cropped out of view by the fit,
    hidden by an ancestor's box or clip, or under another widget), with
    `BAD_ARGUMENT`.
    """
    return tools.inject_pointer(session, node=_node_id(node), source=[x, y],
                                action=action, window_id=window_id, **pointer)


def point_of(session: Any, node: int | Mapping, x: int, y: int, *,
             window_id: int | None = None) -> tuple[float, float]:
    """The window point (logical) where source pixel `(x, y)`'s centre is shown."""
    reply = tools.live_image_map(session, _node_id(node), source=[x, y],
                                 window_id=window_id)
    px, py = reply["source_point"]
    return (px, py)


def rect_of(session: Any, node: int | Mapping, rect: tuple[int, int, int, int], *,
            window_id: int | None = None) -> dict:
    """Where the source rect `(x, y, width, height)` is shown: logical window
    bounds, `{"x", "y", "width", "height"}`."""
    reply = tools.live_image_map(session, _node_id(node), source_rect=list(rect),
                                 window_id=window_id)
    return dict(reply["source_window_rect"])


def source_of(session: Any, node: int | Mapping, window_point: tuple[float, float], *,
              window_id: int | None = None) -> tuple[int, int] | None:
    """The source pixel drawn at `window_point`, an `(x, y)` in logical window
    points, or `None` on the letterbox and outside the picture."""
    x, y = window_point
    reply = tools.live_image_map(session, _node_id(node), window=[x, y],
                                 window_id=window_id)
    pixel = reply.get("pixel")
    if pixel is None:
        return None
    sx, sy = pixel
    return (sx, sy)


def crop_visible(png: bytes, meta: Mapping, node: int | Mapping) -> Picture | None:
    """Cut the screenshot `png`, whose metadata is `meta`, to the picture of
    the `LiveImage` `node`.

    `meta` is what the screenshot answered with — :func:`teksilo_probe.shot.save`
    returns it. `None` when the screenshot drew no picture of the node: no
    source attached, a picture the fit put out of view, a widget culled off
    screen, or a screenshot cropped to somewhere else.
    """
    node_id = _node_id(node)
    shot = next((s for s in meta.get("live_images") or [] if s.get("node") == node_id),
                None)
    if shot is None:
        return None
    width, height, rgba = decode_png(png)
    x, y, w, h = shot["rect"]
    if x + w > width or y + h > height:
        raise ValueError(f"the picture's rect {shot['rect']} exceeds the "
                         f"{width}x{height} image")
    row = w * 4
    cut = b"".join(rgba[((y + r) * width + x) * 4:((y + r) * width + x) * 4 + row]
                   for r in range(h))
    return Picture(w, h, cut, int(shot["generation"]), bool(shot.get("deferred")),
                   float(meta.get("scale") or 1.0))


def capture(session: Any, node: int | Mapping, *,
            window_id: int | None = None) -> Picture | None:
    """Screenshot the `LiveImage` `node` and cut it to its picture: see
    :func:`crop_visible`.

    Raises :class:`~teksilo_probe.session.ToolError` when the screenshot fails,
    `GPU_UNAVAILABLE` included, and with `BAD_REPLY` when its picture does not
    fit its image.
    """
    args: dict[str, Any] = {"node": _node_id(node)}
    if window_id is not None:
        args["window_id"] = window_id
    result = session.call_result("screenshot", **args)
    meta = result.payload if isinstance(result.payload, Mapping) else {}
    png = _image_bytes(result.content)
    if png is None:
        raise ToolError("screenshot", "NO_IMAGE",
                        "the reply carried no image block — the tool answered, "
                        "but with nothing to cut")
    try:
        return crop_visible(png, meta, node)
    except ValueError as error:
        raise ToolError("screenshot", "BAD_REPLY", str(error)) from error


# ---------------------------------------------------------------------------
# PNG, standard library only
# ---------------------------------------------------------------------------

_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def decode_png(data: bytes) -> tuple[int, int, bytes]:
    """`(width, height, rgba)` out of a PNG file's bytes.

    Reads 8-bit RGBA and RGB images that are not interlaced, every filter type
    included; RGB comes back with an opaque alpha. Anything else raises
    :class:`ValueError` rather than decoding wrongly.
    """
    if not data.startswith(_SIGNATURE):
        raise ValueError("not a PNG file")
    pos = len(_SIGNATURE)
    header = None
    idat = []
    while pos + 8 <= len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if len(body) != length:
            raise ValueError("a PNG chunk runs past the end of the file")
        crc = struct.unpack(">I", data[pos + 8 + length:pos + 12 + length])[0]
        if zlib.crc32(kind + body) & 0xFFFFFFFF != crc:
            raise ValueError(f"the PNG chunk {kind!r} fails its checksum")
        pos += 12 + length
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat.append(body)
        elif kind == b"IEND":
            break
    if header is None:
        raise ValueError("a PNG file with no header")
    width, height, depth, colour, _, _, interlace = header
    if depth != 8 or colour not in (2, 6) or interlace != 0:
        raise ValueError(f"unsupported PNG: depth {depth}, colour type {colour}, "
                         f"interlace {interlace}")
    bpp = 4 if colour == 6 else 3
    raw = zlib.decompress(b"".join(idat))
    stride = width * bpp
    if len(raw) != height * (stride + 1):
        raise ValueError("the PNG's pixel data does not match its size")
    out = bytearray()
    previous = bytearray(stride)
    for r in range(height):
        start = r * (stride + 1)
        kind = raw[start]
        line = bytearray(raw[start + 1:start + 1 + stride])
        _unfilter(kind, line, previous, bpp)
        if bpp == 4:
            out += line
        else:
            for i in range(0, stride, 3):
                out += line[i:i + 3] + b"\xff"
        previous = line
    return width, height, bytes(out)


def encode_png(width: int, height: int, rgba: bytes) -> bytes:
    """An 8-bit RGBA PNG of `width × height` pixels from `rgba`."""
    if len(rgba) != width * height * 4:
        raise ValueError(f"{len(rgba)} bytes for a {width}x{height} image")
    stride = width * 4
    raw = b"".join(b"\x00" + rgba[r * stride:(r + 1) * stride] for r in range(height))

    def chunk(kind: bytes, body: bytes) -> bytes:
        crc = zlib.crc32(kind + body) & 0xFFFFFFFF
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", crc)

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (_SIGNATURE + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw))
            + chunk(b"IEND", b""))


def _unfilter(kind: int, line: bytearray, previous: bytearray, bpp: int) -> None:
    """Undo one row's PNG filter in place (PNG specification, section 9)."""
    if kind == 0:
        return
    for i in range(len(line)):
        left = line[i - bpp] if i >= bpp else 0
        up = previous[i]
        if kind == 1:
            predicted = left
        elif kind == 2:
            predicted = up
        elif kind == 3:
            predicted = (left + up) // 2
        elif kind == 4:
            corner = previous[i - bpp] if i >= bpp else 0
            estimate = left + up - corner
            pa, pb, pc = abs(estimate - left), abs(estimate - up), abs(estimate - corner)
            predicted = left if pa <= pb and pa <= pc else up if pb <= pc else corner
        else:
            raise ValueError(f"unknown PNG filter type {kind}")
        line[i] = (line[i] + predicted) & 0xFF


def _node_id(node: int | Mapping) -> int:
    return node if isinstance(node, int) else int(node["id"])
