#!/usr/bin/env python3
"""FX-07: 2x dense contrast/saturation, off-haze identity and crop locality."""
from decimal import Decimal, localcontext
from pathlib import Path
import math
import struct
import subprocess
import tempfile
import zlib

ROOT = Path(__file__).resolve().parents[3]
SAMPLES = ROOT / "tools/research/looks/samples"
LOOK = ROOT / "looks/master-ice.look"
FD = ROOT / "target/release/fd"
if not FD.is_file():
    FD = FD.with_suffix(".exe")
DENSITY = 0.0446683592150963  # master-ice density (checked below)


def align8(n):
    return (n + 7) & ~7


def fine_band_mask(file):
    data = file.read_bytes()
    assert data[:8] == b"FDSAMPLE", file
    major, minor, cols, nx, ny, ss, reserved, max_iter, radius, rot = struct.unpack_from(
        "<HHIIIIIQdd", data, 8)
    assert major == 1 and cols & 15 == 15 and ss == 1, "Expected ss=1 full-column .fds"
    off = 56
    for _ in range(4):
        size, = struct.unpack_from("<H", data, off)
        off += 2 + size
    off = align8(off)
    n = nx * ny
    classes = data[off:off + n]
    off += align8(n) + align8(n * 8)
    de = struct.unpack_from(f"<{n}f", data, off)
    mask = [i for i, (c, d) in enumerate(zip(classes, de))
            if c & 3 == 0 and math.isfinite(d) and d > 0
            and (DENSITY * 2.0 / (max(d, 1e-30) * math.log(2))) / ss >= 1.0]
    return (nx, ny), mask


def png_pixels(file):
    raw = file.read_bytes()
    assert raw.startswith(b"\x89PNG\r\n\x1a\n")
    pos, chunks = 8, []
    while pos < len(raw):
        length, = struct.unpack_from(">I", raw, pos)
        kind = raw[pos + 4:pos + 8]
        value = raw[pos + 8:pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            width, height, depth, color, _, _, interlace = struct.unpack(">IIBBBBB", value)
            assert depth == 8 and color == 2 and interlace == 0
        if kind == b"IDAT":
            chunks.append(value)
    decoded = zlib.decompress(b"".join(chunks))
    stride = width * 3
    pixels = []
    for y in range(height):
        start = y * (stride + 1)
        assert decoded[start] == 0, "fd-shade PNG encoder must use filter 0"
        row = decoded[start + 1:start + 1 + stride]
        pixels.extend(zip(row[::3], row[1::3], row[2::3]))
    return (width, height), pixels


def metrics(pixels, mask):
    # RMS contrast is the coefficient of variation of display-space luminance.
    lum = [sum(a * b for a, b in zip((0.2126, 0.7152, 0.0722), pixels[i])) / 255
           for i in mask]
    avg = sum(lum) / len(lum)
    rms = (sum((v - avg) ** 2 for v in lum) / len(lum)) ** 0.5 / max(avg, 1e-9)
    sat = [0 if max(pixels[i]) == 0 else
           (max(pixels[i]) - min(pixels[i])) / max(pixels[i]) for i in mask]
    return rms, sum(sat) / len(sat)


def shade(fds, look, out, aa="on", dither="on"):
    subprocess.run([str(FD), "shade", str(fds), "--preset", str(look),
                    "--aa", aa, "--dither", dither, "--unresolved", "interior",
                    "-o", str(out)], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    return out / (fds.stem + ".studio.png")


def sparse_crop(dims, mask):
    # Choose an 8x8 tile with fine pixels but fewer than a quarter of them.
    # The former whole-frame gate would shade these pixels differently on crop.
    width, height = dims
    fine = set(mask)
    tiles = []
    for y in range(0, height - 7, 8):
        for x in range(0, width - 7, 8):
            count = sum(yy * width + xx in fine
                        for yy in range(y, y + 8) for xx in range(x, x + 8))
            if 0 < count < 16:
                tiles.append((count, x, y))
    if not tiles:
        raise SystemExit("No sparse mixed crop found in the dense fixture")
    count, x, y = min(tiles)
    return (x, y, 8, 8), count


def crop_fds(source, target, rect):
    # Crop the on-disk columns without rounding nu, DE, or the deep camera.
    data = source.read_bytes()
    magic, major, minor, cols, nx, ny, ss, reserved, maxit, radius, rotation = (
        struct.unpack_from("<8sHHIIIIIQdd", data))
    assert magic == b"FDSAMPLE" and major == 1 and cols & ~31 == 0
    x, y, w, h = rect
    assert ss == 1 and rotation == 0.0 and x + w <= nx and y + h <= ny
    off, strings = 56, []
    for _ in range(4):
        n, = struct.unpack_from("<H", data, off)
        off += 2
        strings.append(data[off:off + n].decode("utf-8"))
        off += n
    off = align8(off)

    # The width is horizontal and the pixel pitch is width/nx. Decimal is
    # essential here (DEC-21): the VIDE-01 centre must never pass through f64.
    with localcontext() as ctx:
        ctx.prec = 200
        re, im, width = map(Decimal, strings[:3])
        pitch = width / Decimal(nx)
        re += (Decimal(x) + Decimal(w) / 2 - Decimal(nx) / 2) * pitch
        im -= (Decimal(y) + Decimal(h) / 2 - Decimal(ny) / 2) * pitch
        strings[:3] = [str(re), str(im), str(pitch * w)]

    buf = bytearray(struct.pack("<8sHHIIIIIQdd", magic, major, minor, cols, w, h,
                                ss, reserved, maxit, radius, rotation))
    for item in strings:
        raw = item.encode("utf-8")
        buf.extend(struct.pack("<H", len(raw)))
        buf.extend(raw)
    buf.extend(b"\0" * (align8(len(buf)) - len(buf)))
    for bit, step in enumerate((1, 8, 4, 2, 4)):
        if cols & (1 << bit):
            for row in range(y, y + h):
                start = off + (row * nx + x) * step
                buf.extend(data[start:start + w * step])
            buf.extend(b"\0" * (align8(w * h * step) - w * h * step))
            off += align8(nx * ny * step)
    target.write_bytes(buf)


def assert_locality(fds, look, tmp, dims, mask):
    rect, fine_count = sparse_crop(dims, mask)
    x, y, w, h = rect
    cropped = tmp / "dense-crop.fds"
    crop_fds(fds, cropped, rect)
    for aa in ("off", "on"):
        full = shade(fds, look, tmp / ("full-" + aa), aa=aa, dither="off")
        small = shade(cropped, look, tmp / ("crop-" + aa), aa=aa, dither="off")
        _, pixels = png_pixels(full)
        small_dims, crop_pixels = png_pixels(small)
        assert small_dims == (w, h)
        for cy in range(h):
            for cx in range(w):
                assert crop_pixels[cy * w + cx] == pixels[(y + cy) * dims[0] + x + cx], (
                    f"crop locality failed aa={aa} at ({cx},{cy}) in {rect}")
    print(f"locality: {w}x{h} crop at ({x},{y}), {fine_count}/{w*h} fine pixels; "
          "byte-identical with AA on and off (dither off)")

def main():
    if not FD.is_file():
        raise SystemExit("Missing target/release/fd; run cargo build --release")
    if not all((SAMPLES / (n + ".fds")).is_file() for n in ("dense", "ordinary")):
        raise SystemExit("Missing samples; run python3 tools/research/looks/make_haze_samples.py")
    lines = LOOK.read_text().splitlines(keepends=True)
    assert any(x.strip() == f"density {DENSITY}" for x in lines), "Master-ice density changed"
    assert sum(line.startswith("haze ") for line in lines) == 1, "Master-ice haze missing"
    with tempfile.TemporaryDirectory(prefix="fx07-") as td:
        tmp = Path(td)
        old = tmp / "previous.look"
        old.write_text("".join(line for line in lines if not line.startswith("haze ")))
        results = {}
        for name in ("dense", "ordinary"):
            fds = SAMPLES / (name + ".fds")
            dims, mask = fine_band_mask(fds)
            baseline = shade(fds, old, tmp / (name + "-old"))
            current = shade(fds, LOOK, tmp / (name + "-new"))
            old_dims, old_pixels = png_pixels(baseline)
            new_dims, new_pixels = png_pixels(current)
            assert dims == old_dims == new_dims
            fine = set(mask)
            outside_changes = sum(a != b for i, (a, b) in
                                  enumerate(zip(old_pixels, new_pixels)) if i not in fine)
            print(f"{name}: outside-haze pixels {len(old_pixels) - len(mask)}, "
                  f"changed {outside_changes}")
            results[name + "-outside"] = outside_changes == 0
            if name == "ordinary":
                changed = sum(old_pixels[i] != new_pixels[i] for i in mask)
                print(f"ordinary: fine-band pixels {len(mask)}, changed {changed}")
                results["ordinary-active"] = changed > 0
                continue
            if len(mask) < len(old_pixels) // 4:
                raise SystemExit(f"Dense sample has only {len(mask)}/{len(old_pixels)} fine-band pixels")
            before = metrics(old_pixels, mask)
            after = metrics(new_pixels, mask)
            print("dense: fine-band pixels", len(mask), "/", len(old_pixels))
            print("dense FX-03 baseline: RMS contrast %.6f mean saturation %.6f" % before)
            print("dense FX-07 updated:  RMS contrast %.6f mean saturation %.6f" % after)
            print("dense ratios: contrast %.3fx saturation %.3fx" %
                  (after[0] / max(before[0], 1e-9), after[1] / max(before[1], 1e-9)))
            results["dense"] = after[0] >= 2 * before[0] and after[1] >= 2 * before[1]
            assert_locality(fds, LOOK, tmp, dims, mask)
            results["crop-locality"] = True
        if not all(results.values()):
            raise SystemExit("FAIL: FX-07 dense 2x, outside-haze identity and crop locality required")
        print("PASS: FX-07", flush=True)


if __name__ == "__main__":
    main()
