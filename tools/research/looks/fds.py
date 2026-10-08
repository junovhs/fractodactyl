"""Minimal .fds reader for look prototyping (FIX/look research): class kind, nu, de, normal.
Assumes the columns nu,de,normal were written (fd render/control default)."""
import numpy as np, struct

def pad(x): return (x + 7) // 8 * 8

def load(path, nx, ny):
    n = nx * ny
    b = open(path, 'rb').read()
    cols = pad(n) + pad(8 * n) + pad(4 * n) + pad(2 * n)
    o = len(b) - cols
    kind = (np.frombuffer(b, np.uint8, n, o) & 3).reshape(ny, nx); o += pad(n)
    nu = np.frombuffer(b, np.float64, n, o).reshape(ny, nx); o += pad(8 * n)
    de = np.frombuffer(b, np.float32, n, o).astype(np.float64).reshape(ny, nx); o += pad(4 * n)
    nm = np.frombuffer(b, np.uint16, n, o).reshape(ny, nx)
    ang = nm / 65536.0 * 2 * np.pi
    return kind, nu, de, np.cos(ang), np.sin(ang)
