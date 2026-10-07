"""BENC-08: the koenigs_bench pixel pipeline as an OpenCL kernel (runs on NVIDIA and AMD).

Same stages as src/main.rs `pixel` (loops, 23 approach steps, phi, Koenigs jump, tail-patch
lookup, finish), in float64; PATCH32=1 evaluates the 16-term tail patch in float32 (the one
stage single_precision.py found float32-safe). Writes the bench's .bin format (class u8 then
nu f64) so koenigs_bench/compare.py can score it, and prints best-of-RUNS kernel seconds.
Usage: python gpu_bench.py CONSTS OUT_DIR NXxNY RUNS WIDTH...   (env: PATCH32, DEVICE substring)
"""
import os, sys, time, math
import numpy as np
import pyopencl as cl
import pyopencl.cltypes
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
import single_precision as sp

SRC = r"""
#pragma OPENCL EXTENSION cl_khr_fp64 : enable
typedef double2 cx;
inline cx cmul(cx a, cx b) { return (cx)(a.x*b.x - a.y*b.y, a.x*b.y + a.y*b.x); }
#if PATCH32
typedef float2 px; inline px pmul(px a, px b) { return (px)(a.x*b.x - a.y*b.y, a.x*b.y + a.y*b.x); }
#else
typedef double2 px; inline px pmul(px a, px b) { return cmul(a, b); }
#endif
__kernel void render(__global const cx *bis, int deg, __global const cx *orbit, __global const cx *phi, int nphi,
                     double scale, double guard, double r0, int period, cx C, cx alpha, cx z24ma,
                     double lnrho, double argrho, int nroot,
                     __global const cx *ncen, __global const int *nkid, __global const int *nleaf,
                     __global const cx *lcen, __global const double *lr, __global const int *ln, __global const px *lcf,
                     int nx, int ny, double h, int maxit, __global uchar *cls, __global double *nu)
{
    int gid = get_global_id(0); if (gid >= nx*ny) return;
    int i = gid % nx, j = gid / nx;
    cx v = (cx)((i + 0.5 - nx/2.0)*h, -(j + 0.5 - ny/2.0)*h) / scale;
    cx b[8];
    for (int a = 0; a <= deg; a++) {
        cx s = (cx)(0, 0);
        for (int q = deg - a; q >= 0; q--) s = cmul(s, v) + bis[a*(deg + 1) + q];
        b[a] = s;
    }
    cx u = v; long n = 1;
    while (length(u)*scale <= guard) {
        if (n + period > maxit) { cls[gid] = 1; nu[gid] = 0; return; }
        cx s = b[deg];
        for (int a = deg - 1; a >= 0; a--) s = cmul(s, u) + b[a];
        u = s; n += period;
    }
    cx d = u*scale;
    for (int k = 0; k < 23; k++) d = cmul(2.0*orbit[k], d) + cmul(d, d);
    n += 23;
    cx h0 = z24ma + d, z = alpha + h0;
    if (length(h0) < r0) {
        cx w0 = (cx)(0, 0);
        for (int k = nphi - 1; k >= 0; k--) w0 = cmul(w0 + phi[k], h0);
        double jj = floor(log(r0/length(w0))/lnrho);
        if (jj >= 0) {
            double re = log(length(w0)) + jj*lnrho;
            double im = atan2(w0.y, w0.x) + jj*argrho;
            im -= floor(im/(2*M_PI))*(2*M_PI);
            int node = min((int)(im/(2*M_PI/nroot)), nroot - 1);
            while (nkid[node] >= 0) { cx c = ncen[node]; node = nkid[node] + 2*(re >= c.x) + (im >= c.y); }
            int L = nleaf[node];
            if (ln[L] >= 0) {
                cx c = lcen[L];
                px t = (px)((re - c.x)/lr[L], (im - c.y)/lr[L]);
                px s = (px)(0, 0);
                for (int q = 15; q >= 0; q--) s = pmul(s, t) + lcf[L*16 + q];
                z = (cx)(s.x, s.y); n += 2*(long)jj + ln[L];
            }
        }
    }
    while (1) {
        if (n >= maxit) { cls[gid] = 1; nu[gid] = 0; return; }
        z = cmul(z, z) + C; n++;
        double a = dot(z, z);
        if (a > 1e20) { cls[gid] = 0; nu[gid] = n + 1 - log2(0.5*log2(a)); return; }
    }
}
"""


def main():
    consts, out, size, runs, widths = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5:]
    nx, ny = map(int, size.split('x')); N = nx*ny
    p32 = os.environ.get('PATCH32') == '1'
    k = sp.load(consts); deg = k['deg']
    want = os.environ.get('DEVICE', 'NVIDIA').lower()
    dev = next(d for p in cl.get_platforms() for d in p.get_devices() if want in (p.name + d.name).lower())
    ctx = cl.Context([dev]); q = cl.CommandQueue(ctx)
    prg = cl.Program(ctx, SRC).build(options=[f'-DPATCH32={int(p32)}'])
    mf = cl.mem_flags
    d2 = lambda z: cl.cltypes.make_double2(z.real, z.imag)
    c2 = lambda arr: np.array([[x.real, x.imag] for x in arr], np.float64)
    bis = np.zeros((deg + 1)*(deg + 1), complex)
    for a, bb, cf in k['bis']: bis[a*(deg + 1) + bb] = cf
    buf = lambda a: cl.Buffer(ctx, mf.READ_ONLY | mf.COPY_HOST_PTR, hostbuf=np.ascontiguousarray(a))
    lcf = k['lcf']
    lcfa = (np.stack([lcf.real, lcf.imag], -1).astype(np.float32 if p32 else np.float64)).reshape(-1, 2)
    args = [buf(c2(bis)), np.int32(deg), buf(c2(k['orbit'])), buf(c2(k['phi'])), np.int32(len(k['phi'])),
            np.float64(k['scale']), np.float64(k['guard']), np.float64(k['r0']), np.int32(int(k['period'])),
            d2(k["c"]), d2(k["alpha"]),
            d2(k["z24_minus_alpha"]),
            np.float64(math.log(abs(k['rho']))), np.float64(math.atan2(k['rho'].imag, k['rho'].real)), np.int32(k['nx']),
            buf(c2(k['ncen'])), buf(k['nkid'].astype(np.int32)), buf(k['nleaf'].astype(np.int32)),
            buf(c2(k['lcen'])), buf(k['lr'].astype(np.float64)), buf(k['ln'].astype(np.int32)), buf(lcfa)]
    cls_g = cl.Buffer(ctx, mf.WRITE_ONLY, N); nu_g = cl.Buffer(ctx, mf.WRITE_ONLY, 8*N)
    os.makedirs(out, exist_ok=True)
    print(f"device: {dev.name}  ({'f32' if p32 else 'f64'} patch)  {size}")
    for w in widths:
        h = float(w)/nx; times = []
        for _ in range(runs):
            t = time.perf_counter()
            prg.render(q, ((N + 255)//256*256,), (256,), *args, np.int32(nx), np.int32(ny), np.float64(h),
                       np.int32(20000), cls_g, nu_g)
            q.finish(); times.append(time.perf_counter() - t)
        cls = np.empty(N, np.uint8); nu = np.empty(N, np.float64)
        cl.enqueue_copy(q, cls, cls_g); cl.enqueue_copy(q, nu, nu_g); q.finish()
        open(os.path.join(out, f'w{w}.bin'), 'wb').write(cls.tobytes() + nu.tobytes())
        print(f"  width {w}: best {min(times)*1000:.1f} ms  (runs {', '.join(f'{x*1000:.0f}' for x in times)})")


if __name__ == '__main__':
    main()
