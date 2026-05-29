#!/usr/bin/env python3
import math
import struct
import zlib
from pathlib import Path


SIZE = 1024


def clamp(value, lower=0, upper=255):
    return max(lower, min(upper, int(round(value))))


def mix(a, b, t):
    return a + (b - a) * t


def smoothstep(edge0, edge1, value):
    if edge0 == edge1:
        return 1.0 if value >= edge1 else 0.0
    t = max(0.0, min(1.0, (value - edge0) / (edge1 - edge0)))
    return t * t * (3.0 - 2.0 * t)


def rounded_rect_alpha(x, y, left, top, right, bottom, radius, feather=3.0):
    px = x + 0.5
    py = y + 0.5
    cx = min(max(px, left + radius), right - radius)
    cy = min(max(py, top + radius), bottom - radius)
    dist = math.hypot(px - cx, py - cy) - radius
    return 1.0 - smoothstep(-feather, feather, dist)


def capsule_alpha(x, y, left, top, right, bottom, feather=3.0):
    radius = (bottom - top) / 2.0
    return rounded_rect_alpha(x, y, left, top, right, bottom, radius, feather)


def blend_pixel(dst, src):
    sr, sg, sb, sa = src
    if sa <= 0:
        return dst
    dr, dg, db, da = dst
    a = sa / 255.0
    out_a = a + da / 255.0 * (1.0 - a)
    if out_a <= 0:
        return (0, 0, 0, 0)
    r = (sr * a + dr * (da / 255.0) * (1.0 - a)) / out_a
    g = (sg * a + dg * (da / 255.0) * (1.0 - a)) / out_a
    b = (sb * a + db * (da / 255.0) * (1.0 - a)) / out_a
    return (clamp(r), clamp(g), clamp(b), clamp(out_a * 255))


def apply_shape(pixels, alpha_fn, color_fn):
    for y in range(SIZE):
        row = y * SIZE
        for x in range(SIZE):
            alpha = alpha_fn(x, y)
            if alpha <= 0.001:
                continue
            color = color_fn(x, y, alpha)
            pixels[row + x] = blend_pixel(pixels[row + x], color)


def draw_icon(output):
    pixels = [(0, 0, 0, 0)] * (SIZE * SIZE)

    # Soft outer shadow.
    for spread, opacity in [(38, 22), (24, 32), (12, 28)]:
        def shadow_alpha(x, y, spread=spread):
            return rounded_rect_alpha(x, y, 86 - spread, 86 - spread, 938 + spread, 938 + spread, 210 + spread, 16)

        def shadow_color(_x, _y, a, opacity=opacity):
            return (18, 36, 64, clamp(opacity * a))

        apply_shape(pixels, shadow_alpha, shadow_color)

    # Main rounded-square body.
    def body_alpha(x, y):
        return rounded_rect_alpha(x, y, 96, 80, 928, 936, 214, 3)

    def body_color(x, y, a):
        nx = x / (SIZE - 1)
        ny = y / (SIZE - 1)
        top_left = (49, 127, 239)
        bottom_right = (9, 157, 150)
        deep = (28, 91, 218)
        r = mix(top_left[0], bottom_right[0], 0.55 * nx + 0.45 * ny)
        g = mix(top_left[1], bottom_right[1], 0.45 * nx + 0.55 * ny)
        b = mix(top_left[2], bottom_right[2], 0.40 * nx + 0.60 * ny)
        vignette = math.hypot(nx - 0.5, ny - 0.52)
        r = mix(r, deep[0], max(0, vignette - 0.42) * 1.25)
        g = mix(g, deep[1], max(0, vignette - 0.42) * 1.25)
        b = mix(b, deep[2], max(0, vignette - 0.42) * 1.25)
        highlight = max(0.0, 1.0 - math.hypot(nx - 0.28, ny - 0.18) / 0.52)
        r = mix(r, 120, highlight * 0.18)
        g = mix(g, 190, highlight * 0.18)
        b = mix(b, 255, highlight * 0.18)
        return (clamp(r), clamp(g), clamp(b), clamp(255 * a))

    apply_shape(pixels, body_alpha, body_color)

    # Inner top glow.
    def glow_alpha(x, y):
        nx = x / SIZE
        ny = y / SIZE
        glow = max(0.0, 1.0 - math.hypot(nx - 0.38, ny - 0.20) / 0.42)
        return glow * body_alpha(x, y)

    def glow_color(_x, _y, a):
        return (255, 255, 255, clamp(56 * a))

    apply_shape(pixels, glow_alpha, glow_color)

    # Glass border.
    def border_alpha(x, y):
        outer = rounded_rect_alpha(x, y, 96, 80, 928, 936, 214, 2)
        inner = rounded_rect_alpha(x, y, 118, 104, 906, 914, 190, 2)
        return max(0.0, outer - inner)

    def border_color(_x, y, a):
        return (255, 255, 255, clamp((95 if y < 430 else 42) * a))

    apply_shape(pixels, border_alpha, border_color)

    # Refill mark shadow.
    def mark_shadow_alpha(x, y):
        top = capsule_alpha(x, y, 286, 334, 738, 458, 4)
        bottom = capsule_alpha(x, y, 286, 552, 738, 676, 4)
        bridge = capsule_alpha(x, y, 472, 404, 552, 606, 4)
        return max(top, bottom, bridge)

    def mark_shadow_color(_x, _y, a):
        return (12, 62, 128, clamp(62 * a))

    apply_shape(pixels, lambda x, y: mark_shadow_alpha(x - 10, y - 18), mark_shadow_color)

    # White switch/refill mark.
    def mark_alpha(x, y):
        top = capsule_alpha(x, y, 278, 322, 730, 446, 3)
        bottom = capsule_alpha(x, y, 294, 542, 746, 666, 3)
        bridge = capsule_alpha(x, y, 474, 398, 556, 590, 3)
        return max(top, bottom, bridge)

    def mark_color(_x, _y, a):
        return (255, 255, 255, clamp(238 * a))

    apply_shape(pixels, mark_alpha, mark_color)

    # Cut colored channels inside tracks.
    def cut_alpha(x, y):
        top = capsule_alpha(x, y, 370, 368, 622, 400, 2)
        bottom = capsule_alpha(x, y, 410, 588, 662, 620, 2)
        return max(top, bottom)

    def cut_color(x, y, a):
        nx = x / SIZE
        return (clamp(mix(29, 7, nx)), clamp(mix(112, 160, nx)), clamp(mix(235, 165, nx)), clamp(255 * a))

    apply_shape(pixels, cut_alpha, cut_color)

    # Two end nodes.
    for cx, cy, radius, color in [
        (326, 384, 38, (255, 255, 255)),
        (704, 604, 38, (255, 255, 255)),
    ]:
        def node_alpha(x, y, cx=cx, cy=cy, radius=radius):
            dist = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
            return 1.0 - smoothstep(radius - 3, radius + 3, dist)

        def node_color(_x, _y, a, color=color):
            return (*color, clamp(245 * a))

        apply_shape(pixels, node_alpha, node_color)

    # Small refill sparkle.
    def sparkle_alpha(x, y):
        cx, cy = 690, 300
        dx = abs(x + 0.5 - cx)
        dy = abs(y + 0.5 - cy)
        diamond = max(0.0, 1.0 - (dx + dy) / 82.0)
        cross = max(0.0, 1.0 - min(dx / 13.0, dy / 13.0)) * max(0.0, 1.0 - max(dx, dy) / 86.0)
        return max(diamond * 0.78, cross)

    def sparkle_color(_x, _y, a):
        return (255, 255, 255, clamp(205 * a))

    apply_shape(pixels, sparkle_alpha, sparkle_color)

    rows = []
    for y in range(SIZE):
        row = bytearray([0])
        for x in range(SIZE):
            row.extend(pixels[y * SIZE + x])
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    Path(output).write_bytes(png)


if __name__ == "__main__":
    draw_icon("src-tauri/icons/icon.png")
