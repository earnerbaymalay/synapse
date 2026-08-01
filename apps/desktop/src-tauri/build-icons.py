#!/usr/bin/env python3
"""
build-icons.py — Generate all required Tauri icon formats from the source icon.

Reads icons/icon.png (512x512 RGBA) and produces:
  - icons/32x32.png       (32x32)
  - icons/128x128.png     (128x128)
  - icons/128x128@2x.png  (256x256)
  - icons/icon.icns       (placeholder — real ICNS requires png2icns or ImageMagick)
  - icons/icon.ico        (placeholder — real ICO requires png2ico or ImageMagick)

Uses only the Python standard library (zlib + struct) to write valid PNG files
with nearest-neighbour downscaling.
"""

import os
import struct
import zlib

# ── Constants ────────────────────────────────────────────────────────────────

SOURCE_REL = "icons/icon.png"
OUTPUTS = {
    "icons/32x32.png": (32, 32),
    "icons/128x128.png": (128, 128),
    "icons/128x128@2x.png": (256, 256),
}
PLACEHOLDER_FILES = [
    "icons/icon.icns",
    "icons/icon.ico",
]

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"

# Color type 6 = RGBA
COLOR_TYPE = 6
BIT_DEPTH = 8


# ── PNG chunk helpers ─────────────────────────────────────────────────────────

def _make_chunk(chunk_type: bytes, data: bytes) -> bytes:
    """Build a single PNG chunk: length + type + data + CRC."""
    length = struct.pack(">I", len(data))
    crc = struct.pack(">I", zlib.crc32(chunk_type + data) & 0xFFFFFFFF)
    return length + chunk_type + data + crc


def _make_ihdr(width: int, height: int) -> bytes:
    """Build the IHDR chunk."""
    data = struct.pack(">IIBBBBB", width, height, BIT_DEPTH, COLOR_TYPE, 0, 0, 0)
    return _make_chunk(b"IHDR", data)


def _make_idat(raw_pixels: bytes, width: int, height: int) -> bytes:
    """
    Build the IDAT chunk.

    Each row is prefixed with a filter byte (0 = None), then the RGBA pixel
    data.  The whole thing is zlib-compressed.
    """
    row_size = width * 4  # 4 bytes per pixel (RGBA)
    raw_data = bytearray()
    for row in range(height):
        start = row * row_size
        raw_data.append(0)  # filter type: None
        raw_data.extend(raw_pixels[start : start + row_size])
    compressed = zlib.compress(bytes(raw_data))
    return _make_chunk(b"IDAT", compressed)


def _make_iend() -> bytes:
    """Build the IEND chunk (empty data)."""
    return _make_chunk(b"IEND", b"")


# ── PNG reader (minimal) ────────────────────────────────────────────────────

def _read_png_rgba(path: str) -> tuple[bytes, int, int]:
    """
    Read a PNG file and return (raw_pixel_bytes, width, height).

    Only handles 8-bit RGBA (color type 6) non-interlaced PNGs.
    """
    with open(path, "rb") as f:
        data = f.read()

    # Verify signature
    if data[:8] != PNG_SIGNATURE:
        raise ValueError(f"Not a valid PNG: {path}")

    # Walk chunks to find IHDR and IDAT
    pos = 8
    width = height = 0
    idat_chunks = []

    while pos < len(data):
        length = struct.unpack(">I", data[pos : pos + 4])[0]
        chunk_type = data[pos + 4 : pos + 8]
        chunk_data = data[pos + 8 : pos + 8 + length]

        if chunk_type == b"IHDR":
            width, height = struct.unpack(">II", chunk_data[:8])
            bit_depth = chunk_data[8]
            color_type = chunk_data[9]
            if bit_depth != 8 or color_type != 6:
                raise ValueError(
                    f"Unsupported PNG format: bit_depth={bit_depth}, "
                    f"color_type={color_type}.  Expected 8-bit RGBA."
                )
        elif chunk_type == b"IDAT":
            idat_chunks.append(chunk_data)

        pos += 12 + length  # length field + type + data + CRC

    if not idat_chunks:
        raise ValueError("No IDAT chunks found in PNG")

    # Decompress
    compressed = b"".join(idat_chunks)
    raw = zlib.decompress(compressed)

    # Strip filter bytes (one per row, filter type 0 = None)
    row_size = width * 4
    pixels = bytearray()
    for row in range(height):
        start = row * (row_size + 1)
        # We only support filter type 0 (None)
        pixels.extend(raw[start + 1 : start + 1 + row_size])

    return bytes(pixels), width, height


# ── Nearest-neighbour downscaler ───────��─────────────────────────────────────

def _downscale_nearest(
    src_pixels: bytes, src_w: int, src_h: int, dst_w: int, dst_h: int
) -> bytes:
    """
    Nearest-neighbour downscaling.

    For each output pixel at (x, y), sample the source pixel at
    (x * src_w / dst_w, y * src_h / dst_h).
    """
    dst = bytearray(dst_w * dst_h * 4)
    for dy in range(dst_h):
        sy = int(dy * src_h / dst_h)
        for dx in range(dst_w):
            sx = int(dx * src_w / dst_w)
            src_idx = (sy * src_w + sx) * 4
            dst_idx = (dy * dst_w + dx) * 4
            dst[dst_idx : dst_idx + 4] = src_pixels[src_idx : src_idx + 4]
    return bytes(dst)


# ── PNG writer ───────────────────────────────────────────────────────────────

def _write_png(path: str, pixels: bytes, width: int, height: int) -> None:
    """Write a valid PNG file from raw RGBA pixel data."""
    with open(path, "wb") as f:
        f.write(PNG_SIGNATURE)
        f.write(_make_ihdr(width, height))
        f.write(_make_idat(pixels, width, height))
        f.write(_make_iend())


# ── Placeholder writers ─────────────────────────────────────────────────────

def _write_placeholder_icns(path: str) -> None:
    """
    Write a minimal placeholder .icns file.

    A real ICNS file embeds PNG data in an 'ic07' (128x128) or 'ic09' (256x256)
    icon entry.  Since we can't run png2icns or ImageMagick here, we write a
    comment file instead so the build process knows what's needed.
    """
    comment = (
        "PLACEHOLDER — Replace with a real ICNS file.\n"
        "Generate it on the build machine with:\n"
        "  png2icns icons/icon.icns icons/icon.png\n"
        "or with ImageMagick:\n"
        "  convert icons/icon.png icons/icon.icns\n"
    )
    with open(path, "w") as f:
        f.write(comment)
    print(f"  [PLACEHOLDER] {path}")


def _write_placeholder_ico(path: str) -> None:
    """
    Write a minimal placeholder .ico file.

    A real ICO file contains a directory entry + embedded PNG or BMP data.
    Since we can't run png2ico or ImageMagick here, we write a comment file
    instead so the build process knows what's needed.
    """
    comment = (
        "PLACEHOLDER — Replace with a real ICO file.\n"
        "Generate it on the build machine with:\n"
        "  png2ico icons/icon.ico icons/icon.png\n"
        "or with ImageMagick:\n"
        "  convert icons/icon.png icons/icon.ico\n"
    )
    with open(path, "w") as f:
        f.write(comment)
    print(f"  [PLACEHOLDER] {path}")


# ── Main ─────────────────────────────────────────────────────────────────────

def main() -> None:
    script_dir = os.path.dirname(os.path.abspath(__file__))
    source_path = os.path.join(script_dir, SOURCE_REL)

    if not os.path.isfile(source_path):
        print(f"ERROR: Source icon not found at {source_path}")
        raise SystemExit(1)

    print(f"Reading source icon: {source_path}")
    src_pixels, src_w, src_h = _read_png_rgba(source_path)
    print(f"  Source dimensions: {src_w}x{src_h} RGBA")

    # ── Generate resized PNGs ───────────────────────────────────────────────
    for rel_path, (out_w, out_h) in OUTPUTS.items():
        out_path = os.path.join(script_dir, rel_path)
        print(f"Generating {out_w}x{out_h} -> {rel_path}")
        pixels = _downscale_nearest(src_pixels, src_w, src_h, out_w, out_h)
        _write_png(out_path, pixels, out_w, out_h)
        print(f"  Wrote {out_path}")

    # ── Generate placeholder ICNS / ICO ─────────────────────────────────────
    for rel_path in PLACEHOLDER_FILES:
        out_path = os.path.join(script_dir, rel_path)
        if rel_path.endswith(".icns"):
            _write_placeholder_icns(out_path)
        elif rel_path.endswith(".ico"):
            _write_placeholder_ico(out_path)

    print("\nDone.  All icon files generated.")
    print("NOTE: icon.icns and icon.ico are placeholders.  To create real")
    print("      macOS/Windows icon files, install png2icns/png2ico or")
    print("      ImageMagick and run the commands shown in the placeholders.")


if __name__ == "__main__":
    main()
