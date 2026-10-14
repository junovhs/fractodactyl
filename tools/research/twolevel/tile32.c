// Two-level reference probe (frontier question 4).
//
// Truth: binary64 perturbation off the global reference with rebasing (Zhuoran), every
// value carried as double mantissa x 2^k (explicit exponent), exactly the fd scaled tier.
// Candidate: per tile, a binary64 secondary reference (itself a rebasing perturbation off
// the global reference) stores its full orbit W_n. Each pixel then iterates its offset
// e = z - W in binary32 mantissa x 2^k (GPU float32 with an integer exponent; rescaling is
// by exact powers of two), rebases onto the tile orbit's start when |z| < |e| or when the
// tile orbit ends, and carries a running first-order absolute error bound rho. The guard
// sends a pixel to binary64 when rho/|dz/dc|/h exceeds the tolerance.
//
// usage: tile32 truth REF W H NMAX OUT.truth
//        tile32 f32   REF W H NMAX TILE TRUTH [refsel=centre|maxit] [tol=1e-3] [bla=0|1]
// Pixel spacing h = 2^K (K from REF, computed for a 1920-wide frame; W x H pixels are
// taken around the centre at that spacing, so a 960x540 run is the central quarter).
#include <math.h>
#include <omp.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct { double re, im; } cd;
typedef struct { float re, im; } cf;

static cd *Z;
static long NZ;
static int K;
static const double R2 = 1e8;  // bailout radius 1e4

typedef struct {
    int32_t n;    // escape iteration, or -1 if not escaped by NMAX
    float pad;
    double nu;    // smooth iteration count
    double de;    // distance estimate in pixels
} Truth;

static inline void renorm64(double *a, double *b, int *k) {
    double M = fmax(fabs(*a), fabs(*b));
    if (M == 0) return;
    int e;
    frexp(M, &e);
    if (e > 64 || e < -64) { *a = ldexp(*a, -e); *b = ldexp(*b, -e); *k += e; }
}

// binary64 perturbation off the global reference. If zs != NULL, store z_0..z_n.
// Returns escape iteration or -1; *len = number of stored entries.
static int run64(double px, double py, int nmax, cd *zs, long *len, double *nu, double *de) {
    double dr = 0, di = 0; int k = 0;     // delta = d * 2^k
    double Dr = 0, Di = 0; int kd = 0;    // dz/dc = D * 2^kd
    long m = 0;
    double zr = 0, zi = 0;
    if (zs) zs[0] = (cd){0, 0};
    for (int n = 0; n < nmax; n++) {
        double t = 2 * (zr * Dr - zi * Di) + ldexp(1.0, -kd);
        Di = 2 * (zr * Di + zi * Dr); Dr = t;
        double Zr = Z[m].re, Zi = Z[m].im;
        double ar = 2 * (Zr * dr - Zi * di), ai = 2 * (Zr * di + Zi * dr);
        double sr = dr * dr - di * di, si = 2 * dr * di;
        dr = ar + ldexp(sr, k) + ldexp(px, K - k);
        di = ai + ldexp(si, k) + ldexp(py, K - k);
        m++;
        renorm64(&dr, &di, &k); renorm64(&Dr, &Di, &kd);
        zr = Z[m].re + ldexp(dr, k); zi = Z[m].im + ldexp(di, k);
        if (zs) zs[n + 1] = (cd){zr, zi};
        double z2 = zr * zr + zi * zi;
        if (z2 > R2) {
            if (len) *len = n + 2;
            double lz = 0.5 * log(z2), az = sqrt(z2);
            *nu = n + 2 - log2(lz);
            *de = ldexp(az * lz / hypot(Dr, Di), -(kd + K));
            return n + 1;
        }
        double d2 = ldexp(dr * dr + di * di, 2 * k);
        if (z2 < d2 || m == NZ - 1) { dr = zr; di = zi; k = 0; renorm64(&dr, &di, &k); m = 0; }
    }
    if (len) *len = nmax + 1;
    *nu = 0; *de = 0;
    return -1;
}

static inline void renorm32(float *a, float *b, int *k, float *rho /* rho, s2 */) {
    float M = fmaxf(fabsf(*a), fabsf(*b));
    if (M == 0) return;
    int e;
    frexpf(M, &e);
    if (e > 32 || e < -32) {
        *a = ldexpf(*a, -e); *b = ldexpf(*b, -e); *k += e;
        if (rho) { rho[0] = ldexpf(rho[0], -e); rho[1] = ldexpf(rho[1], -2 * e); }
    }
}

typedef struct {
    int32_t n;        // escape iteration or -1
    int32_t rebases;  // pixel rebases (|z|<|e|)
    int32_t endrebases;
    int32_t steps;    // executed steps (iterations, or BLA steps + iterations)
    int32_t nl;
    int32_t jmax;     // deepest tile-orbit index this pixel read
    float zrat;       // min over steps of |z_n|^2 / sigma_n^2 (nonlinearity margin)
    double nu, de, bound, sigma, dsig;
} Res;

static const float U = 5.9604645e-8f;  // 2^-24
static int WLO = 0;            // carry the tile orbit as float32 hi+lo (removes coherent W rounding)

// BLA on the tile orbit (one level table per tile: lengths 2^l starting at multiples of 2^l)
typedef struct { float Ar, Ai; int kA; float Br, Bi; int kB; double r; } Bla;  // A = (Ar,Ai) 2^kA

// Tile orbit entry in binary32 mantissa x 2^k (hi) plus a binary32 lo mantissa at the same
// exponent (used only when WLO). Near a nucleus the orbit passes far below the binary32 range,
// so W must carry its own exponent (a plain float W underflows to 0 there).
typedef struct { float re, im, lre, lim; int k; } Wfe;

static inline int fe_exp(float a, float b) {
    float M = fmaxf(fabsf(a), fabsf(b));
    if (M == 0) return INT32_MIN / 4;
    int e; frexpf(M, &e); return e;
}
// normalise mantissa to [0.5,1) by an exact power of two
static inline void fe_norm(float *a, float *b, int *k) {
    int e = fe_exp(*a, *b);
    if (e == INT32_MIN / 4) return;
    *a = ldexpf(*a, -e); *b = ldexpf(*b, -e); *k += e;
}
static inline int imax(int a, int b) { return a > b ? a : b; }
static Wfe to_wfe(cd w) {
    Wfe r = {0, 0, 0, 0, INT32_MIN / 4};
    if (w.re == 0 && w.im == 0) return r;
    int e; frexp(fmax(fabs(w.re), fabs(w.im)), &e);
    double hr = ldexp(w.re, -e), hi = ldexp(w.im, -e);
    r.re = (float)hr; r.im = (float)hi; r.lre = (float)(hr - r.re); r.lim = (float)(hi - r.im); r.k = e;
    return r;
}

// binary32-with-exponent pixel off a tile orbit W (len NT entries, W[0]=0).
// (ex,ey): pixel offset from the tile reference in pixels (exact small integers).
static void run32(float ex, float ey, int nmax, const cd *W64, const Wfe *W, long NT,
                  Bla **bla, int nlev, Res *o, int wlo) {
    const int ZERO = INT32_MIN / 4;
    float mr = 0, mi = 0; int k = ZERO;   // e = m * 2^k (0)
    float Dr = 0, Di = 0; int kd = ZERO;  // dz/dc
    float zr = 0, zi = 0; int kz = ZERO;  // z = W + e
    float s2 = 0;                         // variance model of the error on e, units 2^2k
    float vD = 0;
    const float C2 = 4 * U * U;
    long j = 0;
    int steps = 0, reb = 0, ereb = 0;
    float zrat = 3e38f;
    int jmx = 0, n = 0;
    float epsm_r = ex, epsm_i = ey; int keps = K;
    fe_norm(&epsm_r, &epsm_i, &keps);
    while (n < nmax) {
        int did_bla = 0;
        if (bla && j > 0 && k != ZERO) {
            double e_abs = ldexp((double)hypotf(mr, mi), k);
            for (int l = nlev - 1; l >= 1; l--) {
                long L = 1L << l;
                if (j % L) continue;
                if (j + L >= NT - 1 || n + L >= nmax) continue;
                const Bla *bb = &bla[l][j >> l];
                if (!(e_abs < bb->r)) continue;
                // dz/dc' = A D + B
                float pr = bb->Ar * Dr - bb->Ai * Di, pi = bb->Ar * Di + bb->Ai * Dr;
                int kp = (kd == ZERO) ? ZERO : bb->kA + kd;
                int kn = imax(kp, bb->kB);
                float Do2 = Dr * Dr + Di * Di;
                Dr = (kp == ZERO ? 0 : ldexpf(pr, kp - kn)) + ldexpf(bb->Br, bb->kB - kn);
                Di = (kp == ZERO ? 0 : ldexpf(pi, kp - kn)) + ldexpf(bb->Bi, bb->kB - kn);
                {   float A2 = bb->Ar * bb->Ar + bb->Ai * bb->Ai, Dn2 = Dr * Dr + Di * Di;
                    float rat = (kp == ZERO || Dn2 == 0) ? 0 : ldexpf(A2 * Do2 / Dn2, 2 * (kp - kn));
                    vD = rat * (vD + C2); }
                kd = kn; fe_norm(&Dr, &Di, &kd);
                // e' = A e + B eps
                float ar = bb->Ar * mr - bb->Ai * mi, ai = bb->Ar * mi + bb->Ai * mr;
                float br = bb->Br * epsm_r - bb->Bi * epsm_i, bi = bb->Br * epsm_i + bb->Bi * epsm_r;
                int ka = bb->kA + k, kb = bb->kB + keps;
                int ke = imax(ka, kb);
                float A2 = bb->Ar * bb->Ar + bb->Ai * bb->Ai;
                float nr = ldexpf(ar, ka - ke) + ldexpf(br, kb - ke), ni = ldexpf(ai, ka - ke) + ldexpf(bi, kb - ke);
                // variance: propagate |A|^2 s2, inject rounding + BLA truncation of each term
                float aa = (ar * ar + ai * ai), bb2 = (br * br + bi * bi);
                s2 = ldexpf(A2 * s2, 2 * (ka - ke)) + 2 * C2 * (ldexpf(aa, 2 * (ka - ke)) + ldexpf(bb2, 2 * (kb - ke)));
                mr = nr; mi = ni; k = ke;
                { int k0 = k; fe_norm(&mr, &mi, &k); s2 = ldexpf(s2, 2 * (k0 - k)); }
                j += L; n += (int)L; steps++;
                if (j > jmx) jmx = (int)j;
                did_bla = 1;
                break;
            }
        }
        if (!did_bla) {
            // dz/dc' = 2 z D + 1
            float pr = 2 * (zr * Dr - zi * Di), pi = 2 * (zr * Di + zi * Dr);
            int kp = (kz == ZERO || kd == ZERO) ? ZERO : kz + kd;
            int kn = imax(kp, 1);
            float D2o = Dr * Dr + Di * Di, z2m = zr * zr + zi * zi;
            Dr = (kp == ZERO ? 0 : ldexpf(pr, kp - kn)) + ldexpf(1.0f, -kn);
            Di = (kp == ZERO ? 0 : ldexpf(pi, kp - kn));
            {   float Dn2 = Dr * Dr + Di * Di;
                float rat = (kp == ZERO || Dn2 == 0) ? 0 : ldexpf(4 * z2m * D2o / Dn2, 2 * (kp - kn));
                vD = rat * (vD + C2); }
            kd = kn; fe_norm(&Dr, &Di, &kd);
            // e' = 2 W e + e^2 + eps
            const Wfe *w = &W[j];
            float ar = 0, ai = 0, sr = 0, si = 0;
            int ka = ZERO, ks = ZERO;
            if (k != ZERO && w->k != ZERO) {
                ar = 2 * (w->re * mr - w->im * mi); ai = 2 * (w->re * mi + w->im * mr);
                if (wlo) { ar += 2 * (w->lre * mr - w->lim * mi); ai += 2 * (w->lre * mi + w->lim * mr); }
                ka = w->k + k;
            }
            if (k != ZERO) { sr = mr * mr - mi * mi; si = 2 * mr * mi; ks = 2 * k; }
            int ke = imax(imax(ka, ks), keps);
            float nr = ldexpf(epsm_r, keps - ke), ni = ldexpf(epsm_i, keps - ke);
            if (ka != ZERO) { nr += ldexpf(ar, ka - ke); ni += ldexpf(ai, ka - ke); }
            if (ks != ZERO) { nr += ldexpf(sr, ks - ke); ni += ldexpf(si, ks - ke); }
            // variance: s' = 4|z|^2 s + C2 (|2We|^2 + |e^2|^2 + |eps|^2), all in units 2^2ke
            float prop = (kz == ZERO || k == ZERO) ? 0 : ldexpf(4 * z2m * s2, 2 * (kz + k - ke));
            float inj = ldexpf(epsm_r * epsm_r + epsm_i * epsm_i, 2 * (keps - ke));
            if (ka != ZERO) inj += ldexpf(ar * ar + ai * ai, 2 * (ka - ke));
            if (ks != ZERO) inj += ldexpf(sr * sr + si * si, 2 * (ks - ke));
            s2 = prop + C2 * inj;
            mr = nr; mi = ni; k = ke;
            { int k0 = k; fe_norm(&mr, &mi, &k); s2 = ldexpf(s2, 2 * (k0 - k)); }
            j++; n++; steps++;
            if (j > jmx) jmx = (int)j;
        }
        // z = W + e (aligned add)
        {
            const Wfe *w = &W[j];
            int kw = w->k;
            kz = imax(kw, k);
            if (kz == ZERO) { zr = zi = 0; }
            else {
                zr = (kw == ZERO ? 0 : ldexpf(w->re, kw - kz)) + (k == ZERO ? 0 : ldexpf(mr, k - kz));
                zi = (kw == ZERO ? 0 : ldexpf(w->im, kw - kz)) + (k == ZERO ? 0 : ldexpf(mi, k - kz));
                fe_norm(&zr, &zi, &kz);
                if (zr == 0 && zi == 0) kz = ZERO;
            }
        }
        float z2m = zr * zr + zi * zi;
        if (s2 > 0 && kz != ZERO) {
            float rr = ldexpf(z2m / s2, 2 * (kz - k));
            if (rr < zrat) zrat = rr;
        }
        float z2 = (kz == ZERO) ? 0 : ldexpf(z2m, 2 * kz);
        if (z2 > (float)R2) {
            double zR = W64[j].re + ldexp((double)mr, k), zI = W64[j].im + ldexp((double)mi, k);
            double zz = zR * zR + zI * zI, lz = 0.5 * log(zz), az = sqrt(zz);
            double Dabs = hypot((double)Dr, (double)Di);
            o->n = n; o->nu = n + 1 - log2(lz);
            o->de = ldexp(az * lz / Dabs, -(kd + K));
            o->bound = 0;
            o->sigma = ldexp(sqrt((double)s2) / Dabs, k - kd - K);
            o->dsig = sqrt((double)vD);
            o->rebases = reb; o->endrebases = ereb; o->steps = steps; o->nl = 0; o->zrat = zrat; o->jmax = jmx;
            return;
        }
        int endref = (j >= NT - 1);
        int smaller = (kz == ZERO) || (k != ZERO && ldexpf(z2m, 2 * (kz - k)) < mr * mr + mi * mi);
        if (smaller || endref) {
            // rebase onto the start of the tile orbit; z formed with the binary64 tile orbit
            double zR = W64[j].re + ldexp((double)mr, k), zI = W64[j].im + ldexp((double)mi, k);
            int kn;
            frexp(fmax(fabs(zR), fabs(zI)), &kn);
            if (zR == 0 && zI == 0) { kn = k; }
            float nmr = (float)ldexp(zR, -kn), nmi = (float)ldexp(zI, -kn);
            s2 = ldexpf(s2, 2 * (k - kn)) + C2 * (nmr * nmr + nmi * nmi);
            mr = nmr; mi = nmi; k = kn;
            j = 0;
            if (endref) ereb++; else reb++;
        }
    }
    o->n = -1; o->nu = 0; o->de = 0;
    o->bound = 0; o->sigma = 0; o->dsig = 0; o->rebases = reb; o->endrebases = ereb; o->steps = steps; o->nl = 0; o->zrat = zrat; o->jmax = jmx;
}

// Build BLA levels on a tile orbit (binary64 build, stored float). Level l, block i covers
// [i*2^l, (i+1)*2^l). Single-step: A = 2W, B = 1, r = eps_bla * |W| (with |W|>0).
// Merge (x then y): A = Ay Ax, B = Ay Bx + By, r = min(rx, max(0,(ry - |Bx| epsmax)/|Ax|)).
static int build_bla(const cd *W, long NT, double epsmax_c, Bla **out) {
    int nlev = 1;
    while ((1L << nlev) < NT) nlev++;
    const double eb = ldexp(1.0, -24);
    typedef struct { double Ar, Ai, Br, Bi, r; } B64;
    B64 *prev = malloc(sizeof(B64) * (NT + 1));
    long cnt = NT - 1;
    for (long i = 0; i < cnt; i++) {
        double wr = W[i].re, wi = W[i].im;
        prev[i] = (B64){2 * wr, 2 * wi, 1, 0, eb * hypot(wr, wi)};
    }
    out[0] = NULL;
    for (int l = 1; l < nlev; l++) {
        long c2 = cnt / 2;
        B64 *cur = malloc(sizeof(B64) * (c2 + 1));
        out[l] = malloc(sizeof(Bla) * (c2 + 1));
        for (long i = 0; i < c2; i++) {
            B64 x = prev[2 * i], y = prev[2 * i + 1];
            double Ar = y.Ar * x.Ar - y.Ai * x.Ai, Ai = y.Ar * x.Ai + y.Ai * x.Ar;
            double Br = y.Ar * x.Br - y.Ai * x.Bi + y.Br, Bi = y.Ar * x.Bi + y.Ai * x.Br + y.Bi;
            double ax = hypot(x.Ar, x.Ai), bx = hypot(x.Br, x.Bi);
            double r = fmin(x.r, fmax(0, (y.r - bx * epsmax_c) / ax));
            cur[i] = (B64){Ar, Ai, Br, Bi, r};
            int ea, eb2;
            frexp(fmax(fabs(Ar), fabs(Ai)), &ea); frexp(fmax(fabs(Br), fabs(Bi)), &eb2);
            out[l][i] = (Bla){(float)ldexp(Ar, -ea), (float)ldexp(Ai, -ea), ea,
                              (float)ldexp(Br, -eb2), (float)ldexp(Bi, -eb2), eb2, r};
        }
        free(prev); prev = cur; cnt = c2;
        if (c2 == 0) { nlev = l + 1; break; }
    }
    free(prev);
    return nlev;
}

static cd *load_ref(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) { perror(path); exit(1); }
    int64_t n; int32_t k;
    if (fread(&n, 8, 1, f) != 1 || fread(&k, 4, 1, f) != 1) exit(1);
    NZ = n; K = k;
    cd *z = malloc(sizeof(cd) * n);
    if (fread(z, sizeof(cd), n, f) != (size_t)n) exit(1);
    fclose(f);
    return z;
}

static int cmpd(const void *a, const void *b) {
    double x = *(const double *)a, y = *(const double *)b;
    return (x > y) - (x < y);
}

#define FW 1920
#define FH 1080
#define BS 64
static int selected(int bx, int by, int S) {
    unsigned h = (unsigned)(bx * 73856093u) ^ (unsigned)(by * 19349663u);
    h ^= h >> 13; h *= 0x5bd1e995u; h ^= h >> 15;
    return (h % (unsigned)S) == 0;
}

int main(int argc, char **argv) {
    if (argc < 6) { fprintf(stderr, "see header\n"); return 2; }
    const char *mode = argv[1];
    Z = load_ref(argv[2]);
    int NMAX = atoi(argv[3]), S = atoi(argv[4]);
    const int Wd = FW, Ht = FH;
    size_t NP = (size_t)Wd * Ht;
    int nbx = (Wd + BS - 1) / BS, nby = (Ht + BS - 1) / BS;
    if (!strcmp(mode, "truth")) {
        Truth *T = calloc(NP, sizeof(Truth));
        for (size_t i = 0; i < NP; i++) T[i].n = -2;
        double t0 = omp_get_wtime();
#pragma omp parallel for schedule(dynamic, 1)
        for (int b = 0; b < nbx * nby; b++) {
            int bx = b % nbx, by = b / nbx;
            if (!selected(bx, by, S)) continue;
            for (int y = by * BS; y < (by + 1) * BS && y < Ht; y++)
                for (int x = bx * BS; x < (bx + 1) * BS && x < Wd; x++) {
                    Truth *t = &T[(size_t)y * Wd + x];
                    t->n = run64(x - Wd / 2 + 0.5, y - Ht / 2 + 0.5, NMAX, NULL, NULL, &t->nu, &t->de);
                }
        }
        FILE *f = fopen(argv[5], "wb");
        fwrite(T, sizeof(Truth), NP, f); fclose(f);
        long esc = 0, tot = 0; double sumn = 0;
        for (size_t i = 0; i < NP; i++) {
            if (T[i].n == -2) continue;
            tot++;
            if (T[i].n >= 0) { esc++; sumn += T[i].n; }
        }
        printf("truth px %ld escaped %.4f mean_n %.1f time %.1fs\n", tot,
               (double)esc / tot, sumn / (esc ? esc : 1), omp_get_wtime() - t0);
        return 0;
    }
    int TS = atoi(argv[5]);
    WLO = getenv("WLO") != NULL;
    const char *truthp = argv[6];
    int maxit = argc > 7 && !strcmp(argv[7], "maxit");
    double tol = argc > 8 ? atof(argv[8]) : 1e-3;
    int usebla = argc > 9 ? atoi(argv[9]) : 0;
    Truth *T = malloc(sizeof(Truth) * NP);
    FILE *f = fopen(truthp, "rb");
    if (!f || fread(T, sizeof(Truth), NP, f) != NP) { fprintf(stderr, "truth read\n"); return 1; }
    fclose(f);
    Res *Rz = calloc(NP, sizeof(Res));
    Res *Ra = getenv("ALT") ? calloc(NP, sizeof(Res)) : NULL;
    int tx = (Wd + TS - 1) / TS, ty = (Ht + TS - 1) / TS;
    long tile_ref_iters = 0, tile_ref_need = 0;
    double t0 = omp_get_wtime();
#pragma omp parallel reduction(+ : tile_ref_iters, tile_ref_need)
    {
        cd *W64 = malloc(sizeof(cd) * (NMAX + 2));
        Wfe *W32 = malloc(sizeof(Wfe) * (NMAX + 2));
        cd *A64 = malloc(sizeof(cd) * (NMAX + 2));
        Wfe *A32 = malloc(sizeof(Wfe) * (NMAX + 2));
#pragma omp for schedule(dynamic, 1)
        for (int t = 0; t < tx * ty; t++) {
            int x0 = (t % tx) * TS, y0 = (t / tx) * TS;
            int x1 = x0 + TS < Wd ? x0 + TS : Wd, y1 = y0 + TS < Ht ? y0 + TS : Ht;
            // any selected pixel in this tile?
            int any = 0;
            for (int by = y0 / BS; by <= (y1 - 1) / BS && !any; by++)
                for (int bx = x0 / BS; bx <= (x1 - 1) / BS && !any; bx++)
                    any = selected(bx, by, S);
            if (!any) continue;
            int rx = (x0 + x1) / 2, ry = (y0 + y1) / 2;
            if (maxit) {  // oracle proxy for "deepest pixel from the previous keyframe"
                int best = -3;
                for (int y = y0; y < y1; y++)
                    for (int x = x0; x < x1; x++) {
                        int n = T[(size_t)y * Wd + x].n;
                        if (n == -2) continue;
                        if (n < 0) n = NMAX + 1;
                        if (n > best) { best = n; rx = x; ry = y; }
                    }
            }
            long NT; double nu, de;
            run64(rx - Wd / 2 + 0.5, ry - Ht / 2 + 0.5, NMAX, W64, &NT, &nu, &de);
            tile_ref_iters += NT - 1;
            for (long i = 0; i < NT; i++) W32[i] = to_wfe(W64[i]);
            Bla *bla[64] = {0}; int nlev = 0;
            if (usebla) nlev = build_bla(W64, NT, ldexp(TS * 0.75, K), bla);
            for (int y = y0; y < y1; y++)
                for (int x = x0; x < x1; x++) {
                    if (T[(size_t)y * Wd + x].n == -2) { Rz[(size_t)y * Wd + x].n = -2; continue; }
                    run32((float)(x - rx), (float)(y - ry), NMAX, W64, W32, NT,
                          usebla ? bla : NULL, nlev, &Rz[(size_t)y * Wd + x], WLO);
                }
            { int need = 0;
              for (int y = y0; y < y1; y++) for (int x = x0; x < x1; x++)
                  if (T[(size_t)y * Wd + x].n != -2 && Rz[(size_t)y * Wd + x].jmax > need) need = Rz[(size_t)y * Wd + x].jmax;
              tile_ref_need += need; }
            for (int l = 1; l < nlev; l++) free(bla[l]);
            if (Ra) {
                int ax = rx + TS / 2 + 1, ay = ry + TS / 2 + 1;   // second reference: different orbit, different roundings
                long NA;
                run64(ax - Wd / 2 + 0.5, ay - Ht / 2 + 0.5, NMAX, A64, &NA, &nu, &de);
                for (long i = 0; i < NA; i++) A32[i] = to_wfe(A64[i]);
                for (int y = y0; y < y1; y++)
                    for (int x = x0; x < x1; x++) {
                        if (T[(size_t)y * Wd + x].n == -2) continue;
                        run32((float)(x - ax), (float)(y - ay), NMAX, A64, A32, NA, NULL, 0, &Ra[(size_t)y * Wd + x], 0);
                    }
            }
        }
        free(W64); free(W32); free(A64); free(A32);
    }
    double el = omp_get_wtime() - t0;
    // ---- statistics ----
    long esc = 0, clsmis = 0, fb = 0, over = 0, over_kept = 0, de_bad = 0, de_bad_kept = 0;
    long nonfin = 0;
    // guards: [lambda 4,6,8] x [contract nu-only, full nu+DE]
    const double lams[3] = {4, 6, 8};
    long fb_tol = 0, fb_nl = 0, fb_de = 0;
    long clsk[2][3] = {{0}};
    long fbn[2][3] = {{0}}, overk[2][3] = {{0}}, debadk[2][3] = {{0}};
    double maxk[2][3] = {{0}}, cal = 0;
    double tolde = 2e-3;
    double maxd = 0, maxd_kept = 0, maxde = 0, sumsteps = 0, sumreb = 0, sumereb = 0;
    double *ds = malloc(sizeof(double) * NP);
    long nd = 0;
    double worst_ratio = 1e300;  // min bound/actual among pixels with actual > 1e-6
    long tightn = 0; double tightlog = 0;
    long NS = 0;
    for (size_t i = 0; i < NP; i++) {
        Res *r = &Rz[i]; Truth *t = &T[i];
        if (t->n == -2) continue;
        NS++;
        sumsteps += r->steps; sumreb += r->rebases; sumereb += r->endrebases;
        int fallback = (r->n >= 0 && !(r->bound <= tol));
        int fbl[2][3];
        for (int q = 0; q < 3; q++) {
            double ls = lams[q] * r->sigma;
            int nl = getenv("DERULE") ? (r->n >= 0 && !(ls <= 0.1 * r->de)) : r->nl;
            int nu_ok = !nl && (r->n < 0 || ls <= tol);
            int de_ok = r->n < 0 || !(r->de > 1e-3) || ls <= tolde * r->de;
            fbl[0][q] = !nu_ok;
            fbl[1][q] = !(nu_ok && de_ok);
            fbn[0][q] += fbl[0][q]; fbn[1][q] += fbl[1][q];
            if (q == 0) { fb_tol += r->n >= 0 && !(ls <= tol); fb_nl += nl; fb_de += !de_ok; }
        }
        if ((r->n < 0) != (t->n < 0)) {
            clsmis++;
            if (getenv("DUMP")) fprintf(stderr, "cls px %zu,%zu n32 %d n64 %d\n", i % Wd, i / Wd, r->n, t->n);
            for (int c2 = 0; c2 < 2; c2++) for (int q = 0; q < 3; q++) if (!fbl[c2][q]) clsk[c2][q]++;
            continue;
        }
        if (t->n < 0) continue;
        esc++;
        if (!isfinite(r->nu) || !isfinite(r->de)) { nonfin++; continue; }
        double d = fabs(r->nu - t->nu) * log(2.0) * t->de;
        ds[nd++] = d;
        if (d > maxd) maxd = d;
        if (d > 1e-3) over++;
        if (!fallback) { if (d > maxd_kept) maxd_kept = d; if (d > 1e-3) over_kept++; }
        for (int c2 = 0; c2 < 2; c2++) for (int q = 0; q < 3; q++)
            if (!fbl[c2][q]) { if (d > maxk[c2][q]) maxk[c2][q] = d; if (d > 1e-3) overk[c2][q]++; }
        if (r->sigma > 0 && d / r->sigma > cal && d > 1e-9 && t->de > 1e-2) cal = d / r->sigma;
        if (getenv("DUMP") && (d > 1e-3 || (d > 1e-7 && r->bound < d))) {
            fprintf(stderr, "px %zu,%zu d %.3e bound %.3e n %d/%d reb %d ereb %d de %.3e de32 %.3e nu %.6f/%.6f\n",
                    i % Wd, i / Wd, d, r->bound, r->n, t->n, r->rebases, r->endrebases, t->de, r->de, r->nu, t->nu);
        }
        if (d > 1e-7) {
            double ratio = r->bound / d;
            if (ratio < worst_ratio) worst_ratio = ratio;
            tightlog += log10(ratio); tightn++;
        }
        if (t->de > 1e-3) {
            double e = fabs(r->de / t->de - 1);
            if (e > maxde) maxde = e;
            if (e > 2e-3) { de_bad++; if (!fallback) de_bad_kept++; }
            if (getenv("DUMPDE") && e > 2e-3)
                fprintf(stderr, "DE px %zu,%zu rel %.3e dsig %.3e de %.3e n %d/%d d %.2e sigma %.2e\n", i % Wd, i / Wd, e, r->dsig, t->de, r->n, t->n, d, r->sigma);
            if (e > 2e-3) for (int c2 = 0; c2 < 2; c2++) for (int q = 0; q < 3; q++) if (!fbl[c2][q]) debadk[c2][q]++;
        }
    }
    qsort(ds, nd, sizeof(double), cmpd);
#define Q(p) (nd ? ds[(long)((nd - 1) * (p))] : 0)
    printf("tile %d refsel %s bla %d | px %zu esc %ld cls_mis %ld nonfin %ld | disp p50 %.2e p99 %.2e p99.99 %.2e max %.2e | over1e-3 %ld | fallback %.4f%% | kept max %.2e over %ld | DE maxrel %.2e bad %ld kept_bad %ld | bound/actual min %.2g geo %.2g | steps/px %.1f reb/px %.2f endreb/px %.3f | tileref iters/px %.2f need/px %.3f | %.1fs\n",
           TS, maxit ? "maxit" : "centre", usebla, (size_t)NS, esc, clsmis, nonfin, Q(0.5), Q(0.99), Q(0.9999),
           maxd, over, 100.0 * fb / NS, maxd_kept, over_kept, maxde, de_bad, de_bad_kept,
           worst_ratio, tightn ? pow(10, tightlog / tightn) : 0, sumsteps / NS, sumreb / NS,
           sumereb / NS, (double)tile_ref_iters / NS, (double)tile_ref_need * TS * TS / NS / ((double)TS * TS), el);
    if (getenv("REC")) {
        FILE *fr = fopen(getenv("REC"), "wb");
        for (size_t i = 0; i < NP; i++) {
            if (T[i].n == -2) continue;
            int nrec = Ra ? 13 : 10;
            double rec[13] = {T[i].n, T[i].nu, T[i].de, Rz[i].n, Rz[i].nu, Rz[i].de, Rz[i].sigma, Rz[i].zrat, Rz[i].steps, Rz[i].bound,
                              Ra ? Ra[i].n : 0, Ra ? Ra[i].nu : 0, Ra ? Ra[i].de : 0};
            fwrite(rec, sizeof(double), nrec, fr);
        }
        fclose(fr);
    }
    printf("  sigma calib max(actual/sigma, de>1e-2px) %.2f | lambda4 causes: tol %.4f%% nonlin %.4f%% de %.4f%%\n", cal, 100.0*fb_tol/NS, 100.0*fb_nl/NS, 100.0*fb_de/NS);
    for (int c2 = 0; c2 < 2; c2++) for (int q = 0; q < 3; q++)
        printf("  %s lambda %.0f: fallback %.4f%% | kept max %.2e over1e-3 %ld | kept DE bad %ld | kept cls_mis %ld\n",
               c2 ? "full" : "nu  ", lams[q], 100.0 * fbn[c2][q] / NS, maxk[c2][q], overk[c2][q], debadk[c2][q], clsk[c2][q]);
    return 0;
}
