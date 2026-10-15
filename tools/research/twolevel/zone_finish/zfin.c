// Zone-finish probe for GPU-09: two-level float32 finish with an f32-only fallback.
//
// Input: real CPU-zone finish checkpoints (capture/ tool): per pixel the landing z, dz/dc and
// iteration n; all pixels share the finish parameter C (fixed-C tail). The finish is
// z' = z^2 + C, D' = 2 z D + 1 until |z|^2 > 1e20 or n = 20000.
//
// Arms (all scored against the binary64 finish, i.e. the production zone result):
//   ff   float-float (2 x binary32, ~48-bit) z with binary32-mantissa+exponent dz/dc.
//        This is the f32-only fallback arithmetic (DEC-25/GPU-05: no native binary64).
//   tl   two-level: per T x T tile a float-float reference orbit W from one pixel's landing
//        state; every pixel iterates e = z - W in binary32 mantissa+exponent:
//        e' = 2 W e + e^2  (no c term: C is shared). Rebase (|z| < |e| or W ended) onto the
//        critical orbit of C (z_0 = 0), shared by all tiles, with z formed in float-float.
//        Variance guard as in tile32.c; flagged pixels fall back to ff (or optionally f64).
//
// usage: zfin FILE.zfin TILE [csv_out]
#include <math.h>
#include <omp.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAXIT 20000
static double H, CR, CI, ESC2;
static int NX, NY;

typedef struct { float hi, lo; } ff;

static inline ff two_sum(float a, float b) {
    float s = a + b, bb = s - a;
    return (ff){s, (a - (s - bb)) + (b - bb)};
}
static inline ff fast_two_sum(float a, float b) {
    float s = a + b;
    return (ff){s, b - (s - a)};
}
static inline ff ff_add(ff x, ff y) {
    ff s = two_sum(x.hi, y.hi), t = two_sum(x.lo, y.lo);
    s.lo += t.hi;
    s = fast_two_sum(s.hi, s.lo);
    s.lo += t.lo;
    return fast_two_sum(s.hi, s.lo);
}
static inline ff ff_neg(ff x) { return (ff){-x.hi, -x.lo}; }
static inline ff ff_mul(ff x, ff y) {
    float p = x.hi * y.hi;
    float e = fmaf(x.hi, y.hi, -p);
    e = fmaf(x.hi, y.lo, e);
    e = fmaf(x.lo, y.hi, e);
    return fast_two_sum(p, e);
}
static inline ff ff_from(double d) {
    float hi = (float)d;
    return (ff){hi, (float)(d - (double)hi)};
}
static inline double ff_d(ff x) { return (double)x.hi + (double)x.lo; }

typedef struct {
    int32_t n;       // escape iteration or -1
    double nu, de, ang;
    double sig;      // float-float arm only: error-model sigma of the position, px
} Out;

static void encode(double zr, double zi, double dr, double di, int n, Out *o) {
    double a = zr * zr + zi * zi;
    double log2z = 0.5 * log2(a);
    o->n = n;
    o->nu = n + 1.0 - log2(log2z);
    o->de = 2.0 * sqrt(a) * (log2z * M_LN2) / sqrt(dr * dr + di * di) / H;
    o->ang = atan2(zi * dr - zr * di, zr * dr + zi * di);
}

// ---- binary64 finish (production arithmetic) ----
static void fin64(double zr, double zi, double dr, double di, int n, Out *o, long *steps) {
    while (n < MAXIT) {
        double ndr = 2 * (zr * dr - zi * di) + 1, ndi = 2 * (zr * di + zi * dr);
        double nzr = zr * zr - zi * zi + CR;
        zi = 2 * zr * zi + CI; zr = nzr; dr = ndr; di = ndi;
        n++; (*steps)++;
        if (zr * zr + zi * zi > ESC2) { encode(zr, zi, dr, di, n, o); return; }
    }
    o->n = -1;
}

// ---- double-double finish (~106 bits): ground truth for the shading proof ----
typedef struct { double hi, lo; } dd;
static inline dd dd_two_sum(double a, double b) { double s = a + b, bb = s - a; return (dd){s, (a - (s - bb)) + (b - bb)}; }
static inline dd dd_fast(double a, double b) { double s = a + b; return (dd){s, b - (s - a)}; }
static inline dd dd_add(dd x, dd y) { dd s = dd_two_sum(x.hi, y.hi), t = dd_two_sum(x.lo, y.lo); s.lo += t.hi; s = dd_fast(s.hi, s.lo); s.lo += t.lo; return dd_fast(s.hi, s.lo); }
static inline dd dd_mul(dd x, dd y) { double p = x.hi * y.hi, e = fma(x.hi, y.hi, -p); e = fma(x.hi, y.lo, e); e = fma(x.lo, y.hi, e); return dd_fast(p, e); }
static void findd(double zr0, double zi0, double dr, double di, int n, Out *o) {
    dd zr = {zr0, 0}, zi = {zi0, 0}, cr = {CR, 0}, ci = {CI, 0};
    while (n < MAXIT) {
        double t = 2 * (zr.hi * dr - zi.hi * di) + 1; di = 2 * (zr.hi * di + zi.hi * dr); dr = t;
        dd r2 = dd_mul(zr, zr), i2 = dd_mul(zi, zi), ri = dd_mul(zr, zi);
        dd nzr = dd_add(dd_add(r2, (dd){-i2.hi, -i2.lo}), cr);
        zi = dd_add((dd){2 * ri.hi, 2 * ri.lo}, ci); zr = nzr;
        n++;
        if (zr.hi * zr.hi + zi.hi * zi.hi > ESC2) { encode(zr.hi + zr.lo, zi.hi + zi.lo, dr, di, n, o); return; }
    }
    o->n = -1;
}

// binary32 mantissa x 2^k complex for dz/dc
static inline void fe_norm(float *a, float *b, int *k) {
    float M = fmaxf(fabsf(*a), fabsf(*b));
    if (M == 0) return;
    int e; frexpf(M, &e);
    *a = ldexpf(*a, -e); *b = ldexpf(*b, -e); *k += e;
}
static inline int imax(int a, int b) { return a > b ? a : b; }
#define ZERO (INT32_MIN / 4)

// dz/dc' = 2 z D + 1 with z as float (hi part), D float-exp
static inline void dstep(float zr, float zi, float *Dr, float *Di, int *kd) {
    float pr = 2 * (zr * *Dr - zi * *Di), pi = 2 * (zr * *Di + zi * *Dr);
    int kn = imax(*kd, 1);
    *Dr = ldexpf(pr, *kd - kn) + ldexpf(1.0f, -kn);
    *Di = ldexpf(pi, *kd - kn);
    *kd = kn; fe_norm(Dr, Di, kd);
}

// ---- float-float finish: the f32-only fallback ----
static void finff(double zr0, double zi0, double dr0, double di0, int n, Out *o, long *steps) {
    ff zr = ff_from(zr0), zi = ff_from(zi0);
    ff cr = ff_from(CR), ci = ff_from(CI);
    int kd = 0; float Dr = (float)dr0, Di = (float)di0;
    { int e; frexp(fmax(fabs(dr0), fabs(di0)), &e); Dr = (float)ldexp(dr0, -e); Di = (float)ldexp(di0, -e); kd = e; }
    double Dd_r = dr0, Dd_i = di0; int d64 = getenv("FFD64") != NULL;
    // variance of the absolute error on z: landing rounding to float-float, then per-step
    // rounding (~2^-47 relative on z^2 and C) propagated with |2z|^2
    const double UF = ldexp(1.0, -47);
    double s2 = UF * UF * (zr0 * zr0 + zi0 * zi0);
    while (n < MAXIT) {
        {   double a = ff_d(zr), b = ff_d(zi), z2 = a * a + b * b;
            s2 = 4 * z2 * s2 + 2 * UF * UF * (z2 * z2 + CR * CR + CI * CI); }
        if (d64) { double zR = ff_d(zr), zI = ff_d(zi); double t = 2 * (zR * Dd_r - zI * Dd_i) + 1; Dd_i = 2 * (zR * Dd_i + zI * Dd_r); Dd_r = t; }
        else dstep(zr.hi, zi.hi, &Dr, &Di, &kd);
        ff r2 = ff_mul(zr, zr), i2 = ff_mul(zi, zi), ri = ff_mul(zr, zi);
        ff nzr = ff_add(ff_add(r2, ff_neg(i2)), cr);
        zi = ff_add((ff){2 * ri.hi, 2 * ri.lo}, ci);
        zr = nzr;
        n++; (*steps)++;
        if (zr.hi * zr.hi + zi.hi * zi.hi > (float)ESC2) {
            if (d64) encode(ff_d(zr), ff_d(zi), Dd_r, Dd_i, n, o); else encode(ff_d(zr), ff_d(zi), ldexp((double)Dr, kd), ldexp((double)Di, kd), n, o);
            o->sig = sqrt(s2) / hypot(ldexp((double)Dr, kd), ldexp((double)Di, kd)) / H;
            return;
        }
    }
    o->n = -1;
}

// ---- shared critical orbit of C (float-float, periodic so effectively endless) ----
static ff *CRI, *CII;  // critical orbit 0, C, C^2+C, ...
static long NCRIT;

typedef struct { float re, im, lre, lim; int k; } Wfe;  // hi/lo mantissas at exponent k
static Wfe to_wfe(ff r, ff i) {
    Wfe w = {0, 0, 0, 0, ZERO};
    double dr = ff_d(r), di = ff_d(i);
    if (dr == 0 && di == 0) return w;
    int e; frexp(fmax(fabs(dr), fabs(di)), &e);
    w.re = ldexpf(r.hi, -e); w.im = ldexpf(i.hi, -e); w.lre = ldexpf(r.lo, -e); w.lim = ldexpf(i.lo, -e); w.k = e;
    return w;
}

typedef struct { Out o; float sigma; float zrat; float dsig; int steps; int reb; } TL;

// two-level pixel. W: tile orbit (NW entries, W[0] = tile landing state). e0 = z_p - z_T.
static void fintl(double e0r, double e0i, double dr0, double di0, int n, const Wfe *W, const ff *Wr, const ff *Wi,
                  long NW, const Wfe *Cw, TL *t) {
    const float U = 5.9604645e-8f, C2 = 4 * U * U;
    float mr, mi; int k;
    {   int e; if (e0r == 0 && e0i == 0) { mr = mi = 0; k = ZERO; }
        else { frexp(fmax(fabs(e0r), fabs(e0i)), &e); mr = (float)ldexp(e0r, -e); mi = (float)ldexp(e0i, -e); k = e; } }
    float Dr, Di; int kd;
    { int e; frexp(fmax(fabs(dr0), fabs(di0)), &e); Dr = (float)ldexp(dr0, -e); Di = (float)ldexp(di0, -e); kd = e; }
    // rounding of e0 itself
    float s2 = (k == ZERO) ? 0 : C2 * (mr * mr + mi * mi);
    const Wfe *orb = W; const ff *orr = Wr, *ori = Wi; long norb = NW; long j = 0;
    float zr, zi; int kz;
    // z_0 = W_0 + e_0
    #define FORM_Z() do { const Wfe *w = &orb[j]; kz = imax(w->k, k); \
        if (kz == ZERO) { zr = zi = 0; } else { \
        zr = (w->k == ZERO ? 0 : ldexpf(w->re, w->k - kz)) + (k == ZERO ? 0 : ldexpf(mr, k - kz)); \
        zi = (w->k == ZERO ? 0 : ldexpf(w->im, w->k - kz)) + (k == ZERO ? 0 : ldexpf(mi, k - kz)); \
        fe_norm(&zr, &zi, &kz); if (zr == 0 && zi == 0) kz = ZERO; } } while (0)
    FORM_Z();
    float zrat = 3e38f, vD = 0;
    int steps = 0, reb = 0;
    while (n < MAXIT) {
        // dz/dc' = 2 z D + 1 (z float-exp); vD: variance model of the relative error of D
        {   float pr = 2 * (zr * Dr - zi * Di), pi = 2 * (zr * Di + zi * Dr);
            int kp = (kz == ZERO) ? ZERO : kz + kd;
            int kn = imax(kp, 1);
            float p2 = pr * pr + pi * pi;
            Dr = (kp == ZERO ? 0 : ldexpf(pr, kp - kn)) + ldexpf(1.0f, -kn);
            Di = (kp == ZERO ? 0 : ldexpf(pi, kp - kn));
            float dn2 = Dr * Dr + Di * Di;
            float amp = (kp == ZERO || dn2 == 0) ? 0 : ldexpf(p2 / dn2, 2 * (kp - kn));
            vD = amp * (vD + 3 * C2) + C2;
            kd = kn; fe_norm(&Dr, &Di, &kd); }
        // e' = 2 W e + e^2 (W hi+lo)
        const Wfe *w = &orb[j];
        float z2m = zr * zr + zi * zi;
        float ar = 0, ai = 0, sr = 0, si = 0; int ka = ZERO, ks = ZERO;
        if (k != ZERO && w->k != ZERO) {
            ar = 2 * (w->re * mr - w->im * mi) + 2 * (w->lre * mr - w->lim * mi);
            ai = 2 * (w->re * mi + w->im * mr) + 2 * (w->lre * mi + w->lim * mr);
            ka = w->k + k;
        }
        if (k != ZERO) { sr = mr * mr - mi * mi; si = 2 * mr * mi; ks = 2 * k; }
        int ke = imax(ka, ks);
        float prop = (kz == ZERO || k == ZERO || ke == ZERO) ? 0 : ldexpf(4 * z2m * s2, 2 * (kz + k - ke));
        float inj = 0, nr = 0, ni = 0;
        if (ka != ZERO) { nr += ldexpf(ar, ka - ke); ni += ldexpf(ai, ka - ke); inj += ldexpf(ar * ar + ai * ai, 2 * (ka - ke)); }
        if (ks != ZERO) { nr += ldexpf(sr, ks - ke); ni += ldexpf(si, ks - ke); inj += ldexpf(sr * sr + si * si, 2 * (ks - ke)); }
        s2 = prop + C2 * inj;
        mr = nr; mi = ni; k = ke;
        if (k != ZERO) { int k0 = k; fe_norm(&mr, &mi, &k); s2 = ldexpf(s2, 2 * (k0 - k)); if (mr == 0 && mi == 0) k = ZERO; }
        j++; n++; steps++;
        FORM_Z();
        float z2 = (kz == ZERO) ? 0 : ldexpf(zr * zr + zi * zi, 2 * kz);
        if (s2 > 0 && kz != ZERO && k != ZERO) {
            float rr = ldexpf((zr * zr + zi * zi) / s2, 2 * (kz - k));
            if (rr < zrat) zrat = rr;
        }
        if (z2 > (float)ESC2) {
            double ZR = ff_d(orr[j]) + ldexp((double)mr, k), ZI = ff_d(ori[j]) + ldexp((double)mi, k);
            double dR = ldexp((double)Dr, kd), dI = ldexp((double)Di, kd);
            encode(ZR, ZI, dR, dI, n, &t->o);
            t->sigma = (float)(ldexp(sqrt((double)s2), k) / hypot(dR, dI) / H);  // px, same units as fd compare displacement
            t->zrat = zrat; t->dsig = sqrtf(vD); t->steps = steps; t->reb = reb;
            return;
        }
        int ended = (j >= norb - 1);
        int smaller = (kz == ZERO) || (k != ZERO && ldexpf(zr * zr + zi * zi, 2 * (kz - k)) < mr * mr + mi * mi);
        if (smaller || ended) {
            // rebase onto the critical orbit of C; z formed in float-float
            ff er = ff_from(ldexp((double)mr, k)), ei = ff_from(ldexp((double)mi, k));
            ff Zr = ff_add(orr[j], er), Zi = ff_add(ori[j], ei);
            double zR = ff_d(Zr), zI = ff_d(Zi);
            int kn; frexp(fmax(fabs(zR), fabs(zI)), &kn);
            float nmr = (float)ldexp(zR, -kn), nmi = (float)ldexp(zI, -kn);
            s2 = ldexpf(s2, 2 * (k - kn)) + C2 * (nmr * nmr + nmi * nmi);
            mr = nmr; mi = nmi; k = kn;
            if (zR == 0 && zI == 0) k = ZERO;
            orb = Cw; orr = CRI; ori = CII; norb = NCRIT; j = 0;
            reb++;
            FORM_Z();
        }
    }
    t->o.n = -1; t->sigma = 0; t->zrat = zrat; t->steps = steps; t->reb = reb;
}

typedef struct { uint32_t idx, it; double zr, zi, dr, di; } Rec;

int main(int argc, char **argv) {
    if (argc < 3) { fprintf(stderr, "usage: zfin FILE.zfin TILE [csv]\n"); return 2; }
    FILE *f = fopen(argv[1], "rb");
    char mag[8]; uint32_t nx, ny, cnt;
    if (!f || fread(mag, 1, 8, f) != 8 || memcmp(mag, "ZFIN0001", 8)) { fprintf(stderr, "bad file\n"); return 1; }
    fread(&nx, 4, 1, f); fread(&ny, 4, 1, f);
    fread(&H, 8, 1, f); fread(&CR, 8, 1, f); fread(&CI, 8, 1, f); fread(&ESC2, 8, 1, f);
    fread(&cnt, 4, 1, f);
    NX = nx; NY = ny;
    Rec *R = malloc(sizeof(Rec) * cnt);
    for (uint32_t i = 0; i < cnt; i++) {
        fread(&R[i].idx, 4, 1, f); fread(&R[i].it, 4, 1, f);
        fread(&R[i].zr, 8, 4, f);
    }
    fclose(f);
    int T = atoi(argv[2]);
    size_t NP = (size_t)nx * ny;
    int32_t *map = malloc(sizeof(int32_t) * NP);
    for (size_t i = 0; i < NP; i++) map[i] = -1;
    for (uint32_t i = 0; i < cnt; i++) map[R[i].idx] = (int32_t)i;

    // critical orbit of C in float-float (period-764 nucleus, so it stays bounded)
    NCRIT = MAXIT + 2;
    CRI = malloc(sizeof(ff) * NCRIT); CII = malloc(sizeof(ff) * NCRIT);
    Wfe *Cw = malloc(sizeof(Wfe) * NCRIT);
    {   ff zr = {0, 0}, zi = {0, 0}, cr = ff_from(CR), ci = ff_from(CI);
        for (long k = 0; k < NCRIT; k++) {
            CRI[k] = zr; CII[k] = zi; Cw[k] = to_wfe(zr, zi);
            ff r2 = ff_mul(zr, zr), i2 = ff_mul(zi, zi), ri = ff_mul(zr, zi);
            ff nzr = ff_add(ff_add(r2, ff_neg(i2)), cr);
            zi = ff_add((ff){2 * ri.hi, 2 * ri.lo}, ci); zr = nzr;
        } }

    Out *t64 = calloc(cnt, sizeof(Out)), *tff = calloc(cnt, sizeof(Out));
    TL *ttl = calloc(cnt, sizeof(TL));
    long st64 = 0, stff = 0, sttl = 0, stref = 0, nrefs = 0;
    double t0 = omp_get_wtime();
#pragma omp parallel for schedule(dynamic, 4096) reduction(+ : st64, stff)
    for (long i = 0; i < (long)cnt; i++) {
        long a = 0, b = 0;
        fin64(R[i].zr, R[i].zi, R[i].dr, R[i].di, R[i].it, &t64[i], &a);
        finff(R[i].zr, R[i].zi, R[i].dr, R[i].di, R[i].it, &tff[i], &b);
        st64 += a; stff += b;
    }
    int tx = (nx + T - 1) / T, ty = (ny + T - 1) / T;
#pragma omp parallel reduction(+ : sttl, stref, nrefs)
    {
        Wfe *W = malloc(sizeof(Wfe) * (MAXIT + 2));
        ff *Wr = malloc(sizeof(ff) * (MAXIT + 2)), *Wi = malloc(sizeof(ff) * (MAXIT + 2));
#pragma omp for schedule(dynamic, 16)
        for (int t = 0; t < tx * ty; t++) {
            int x0 = (t % tx) * T, y0 = (t / tx) * T;
            int x1 = x0 + T < (int)nx ? x0 + T : (int)nx, y1 = y0 + T < (int)ny ? y0 + T : (int)ny;
            // reference: captured pixel nearest the tile centre
            int best = -1; double bd = 1e300;
            for (int y = y0; y < y1; y++) for (int x = x0; x < x1; x++) {
                int m = map[(size_t)y * nx + x]; if (m < 0) continue;
                double d = (x - (x0 + x1) / 2.0) * (x - (x0 + x1) / 2.0) + (y - (y0 + y1) / 2.0) * (y - (y0 + y1) / 2.0);
                if (d < bd) { bd = d; best = m; }
            }
            if (best < 0) continue;
            // tile orbit in float-float from the reference's landing state, as far as any pixel needs
            int minit = MAXIT;
            for (int y = y0; y < y1; y++) for (int x = x0; x < x1; x++) {
                int m = map[(size_t)y * nx + x]; if (m >= 0 && (int)R[m].it < minit) minit = R[m].it;
            }
            long NW = MAXIT - minit + 1;
            ff zr = ff_from(R[best].zr), zi = ff_from(R[best].zi), cr = ff_from(CR), ci = ff_from(CI);
            long kk;
            for (kk = 0; kk < NW; kk++) {
                Wr[kk] = zr; Wi[kk] = zi; W[kk] = to_wfe(zr, zi);
                if (zr.hi * zr.hi + zi.hi * zi.hi > (float)ESC2) { kk++; break; }
                ff r2 = ff_mul(zr, zr), i2 = ff_mul(zi, zi), ri = ff_mul(zr, zi);
                ff nzr = ff_add(ff_add(r2, ff_neg(i2)), cr);
                zi = ff_add((ff){2 * ri.hi, 2 * ri.lo}, ci); zr = nzr;
            }
            NW = kk;
            long need = 0;
            for (int y = y0; y < y1; y++) for (int x = x0; x < x1; x++) {
                int m = map[(size_t)y * nx + x]; if (m < 0) continue;
                fintl(R[m].zr - R[best].zr, R[m].zi - R[best].zi, R[m].dr, R[m].di, R[m].it, W, Wr, Wi, NW, Cw, &ttl[m]);
                sttl += ttl[m].steps;
                long reach = ttl[m].reb ? NW : ttl[m].steps;  // steps on the tile orbit before leaving it
                if (reach > need) need = reach;
            }
            stref += need < NW ? need : NW; nrefs++;
        }
        free(W); free(Wr); free(Wi);
    }
    double el = omp_get_wtime() - t0;

    // ---- scoring ----
    // contract A: DE/normal gated where DE_ref > 1e-3 px (current); B: where DE_ref >= 1 px.
    const double lam = 4;
    long esc = 0;
    long ff_cls = 0, ff_px = 0, ff_deA = 0, ff_deB = 0; double ff_max = 0;
    long tl_cls_raw = 0, tl_px_raw = 0; double tl_max_raw = 0;
    long fbA = 0, fbB = 0, fbN = 0; double fbstA = 0, fbstB = 0;
    long finA_cls = 0, finA_px = 0, finA_de = 0, finA_nm = 0; double finA_max = 0;
    long finB_cls = 0, finB_px = 0, finB_de = 0, finB_nm = 0; double finB_max = 0;
    long keptA_deov = 0, kcls[2] = {0}, kpx[2] = {0}, kde[2] = {0}, knm[2] = {0}; double kmax[2] = {0};
    double ang_lim = 0.2 * M_PI / 180;
    for (uint32_t i = 0; i < cnt; i++) {
        Out *r = &t64[i];
        int re = r->n >= 0;
        esc += re;
        // ff arm alone
        {   Out *c = &tff[i];
            if ((c->n >= 0) != re) ff_cls++;
            else if (re) {
                double d = fabs(c->nu - r->nu) * r->de * M_LN2 / 2;
                if (d > ff_max) ff_max = d;
                ff_px += d > 1e-3;
                double rel = fabs(c->de - r->de) / r->de;
                double da = fabs(remainder(c->ang - r->ang, 2 * M_PI));
                if (r->de > 1e-3 && (rel > 2e-3 || da > ang_lim)) { ff_deA++;
                    if (getenv("DUMPFF")) fprintf(stderr, "ff idx %u de_ref %.4e de_ff %.4e rel %.2e dang %.2e deg n %d/%d d %.2e\n", R[i].idx, r->de, c->de, rel, da*180/M_PI, c->n, r->n, d); }
                if (r->de >= 1 && (rel > 2e-3 || da > ang_lim)) ff_deB++;
            } }
        TL *c = &ttl[i];
        int ce = c->o.n >= 0;
        double d = 0, rel = 0, da = 0;
        if (ce != re) tl_cls_raw++;
        else if (re) {
            d = fabs(c->o.nu - r->nu) * r->de * M_LN2 / 2;
            rel = fabs(c->o.de - r->de) / r->de;
            da = fabs(remainder(c->o.ang - r->ang, 2 * M_PI));
            if (d > tl_max_raw) tl_max_raw = d;
            tl_px_raw += d > 1e-3;
        }
        // guard (uses only the candidate's own outputs)
        double ls = lam * c->sigma;
        double dist = 0.5 * c->o.de;  // Milnor distance in px (production DE carries a factor 2)
        int nu_ok = ce ? (ls <= 1e-3 && ls <= dist) : (c->zrat >= lam * lam);
        double lde = getenv("LAMDE") ? atof(getenv("LAMDE")) : lam;
        int dok = getenv("NODSIG") ? 1 : (lam * c->dsig <= 2e-3);
        int deA_ok = !ce || !(c->o.de > 0.5e-3) || (lde * c->sigma <= 2e-3 * dist && dok);
        int deB_ok = !ce || !(c->o.de >= 0.5) || (lde * c->sigma <= 2e-3 * dist && dok);
        int fA = !(nu_ok && deA_ok), fB = !(nu_ok && deB_ok);
        fbN += !nu_ok; fbA += fA; fbB += fB;
        // fallback cost: rerun from the landing checkpoint in float-float
        long ffst = (tff[i].n >= 0 ? tff[i].n : MAXIT) - R[i].it;
        if (fA) fbstA += ffst;
        if (fB) fbstB += ffst;
        // final outputs: kept tl or ff fallback
        // kept-only errors (equivalently: native binary64 fallback arm, which reproduces the reference)
        for (int ct = 0; ct < 2; ct++) {
            int fb = ct ? fB : fA;
            if (fb) continue;
            int oe = c->o.n >= 0;
            if (oe != re) { kcls[ct]++; continue; }
            if (!re) continue;
            double dd = fabs(c->o.nu - r->nu) * r->de * M_LN2 / 2;
            double rl = fabs(c->o.de - r->de) / r->de;
            double aa = fabs(remainder(c->o.ang - r->ang, 2 * M_PI));
            if (dd > kmax[ct]) kmax[ct] = dd;
            kpx[ct] += dd > 1e-3;
            if (ct ? r->de >= 1.0 : r->de > 1e-3) { kde[ct] += rl > 2e-3; knm[ct] += aa > ang_lim;
                if (getenv("DUMP") && ct == 0 && (rl > 2e-3 || aa > ang_lim))
                    fprintf(stderr, "kept idx %u (%u,%u) de_ref %.3e de_tl %.3e rel %.2e dang %.2e deg | sigma %.2e dsig %.2e | n %d/%d ff_n %d ff_rel %.2e | land_it %u steps %d reb %d d %.2e\n",
                        R[i].idx, R[i].idx % nx, R[i].idx / nx, r->de, c->o.de, rl, aa * 180 / M_PI, c->sigma, c->dsig, c->o.n, r->n, tff[i].n,
                        fabs(tff[i].de - r->de) / r->de, R[i].it, c->steps, c->reb, dd); }
        }
        for (int ct = 0; ct < 2; ct++) {
            int fb = ct ? fB : fA;
            Out *o = fb ? &tff[i] : &c->o;
            int oe = o->n >= 0;
            long *cls = ct ? &finB_cls : &finA_cls, *px = ct ? &finB_px : &finA_px;
            long *de = ct ? &finB_de : &finA_de, *nm = ct ? &finB_nm : &finA_nm;
            double *mx = ct ? &finB_max : &finA_max;
            if (oe != re) { (*cls)++; continue; }
            if (!re) continue;
            double dd = fabs(o->nu - r->nu) * r->de * M_LN2 / 2;
            double rl = fabs(o->de - r->de) / r->de;
            double aa = fabs(remainder(o->ang - r->ang, 2 * M_PI));
            if (dd > *mx) *mx = dd;
            *px += dd > 1e-3;
            double gate = ct ? 1.0 : 1e-3;
            if ((ct ? r->de >= gate : r->de > gate)) { *de += rl > 2e-3; *nm += aa > ang_lim; }
        }
        (void)keptA_deov;
    }
    long ffg_flag = 0;
    if (getenv("DUMPOUT")) {
        // per captured pixel, capture order: 4 arms x {n (as double), nu, de, angle}: binary64, ff, hybrid-B, double-double
        FILE *fo = fopen(getenv("DUMPOUT"), "wb");
        for (uint32_t i = 0; i < cnt; i++) {
            TL *c = &ttl[i];
            double ls = lam * c->sigma, dist = 0.5 * c->o.de;
            int ce = c->o.n >= 0;
            int nu_ok = ce ? (ls <= 1e-3 && ls <= dist) : (c->zrat >= lam * lam);
            int deB_ok = !ce || !(c->o.de >= 0.5) || ls <= 2e-3 * dist;
            Out *h = (nu_ok && deB_ok) ? &c->o : &tff[i];
            if (getenv("FFGUARD")) {
                // rerun if the float-float error could shift nu by more than DNU iterations,
                // for pixels outside the already-exempt <1e-3 px band (production itself is unstable there)
                double lamf = atof(getenv("FFGUARD")), dnu = getenv("DNU") ? atof(getenv("DNU")) : 0.01;
                Out *f = &tff[i];
                int flag = f->n >= 0 && f->de * 0.5 > 1e-3 * 0.5 && lamf * f->sig > dnu * f->de * M_LN2 / 2;
                h = flag ? &t64[i] : f;
                ffg_flag += flag;
            }
            Out tdd; findd(R[i].zr, R[i].zi, R[i].dr, R[i].di, R[i].it, &tdd);
            Out *arms[4] = {&t64[i], &tff[i], h, &tdd};
            for (int a = 0; a < 4; a++) {
                double v[4] = {arms[a]->n, arms[a]->nu, arms[a]->de, arms[a]->ang};
                fwrite(v, sizeof v, 1, fo);
            }
        }
        fclose(fo);
        if (getenv("FFGUARD")) printf("ff guard: rerun share %.4f%%\n", 100.0 * ffg_flag / cnt);
    }
    double mean64 = (double)st64 / cnt;
    // ---- cost model (issue slots per lane per step; see notes) ----
    const double F64_STEP = 12, F64_INT = 4;       // binary64 finish: 12 FP64 ops + 4 int/compare
    const double FF_STEP = 100;                     // float-float z (81) + f32-exp dz/dc (12) + escape/loop (7)
    const double TL_STEP = 41;                      // two-level float32 step incl. guard
    double slots_tl = TL_STEP * sttl + FF_STEP * stref;  // + tile orbits (float-float)
    double s64_rdna = (F64_STEP * 16 + F64_INT) * st64, s64_amp = (F64_STEP * 64 + F64_INT) * st64;
    double sff = FF_STEP * stff;
    printf("file %s T %d | captured %u escaped %.4f | mean finish steps f64 %.1f ff %.1f tl %.1f | tile orbit steps/px %.2f (refs %ld) | %.1fs\n",
           argv[1], T, cnt, (double)esc / cnt, mean64, (double)stff / cnt, (double)sttl / cnt, (double)stref / cnt, nrefs, el);
    printf("  ff alone (f32-only, every pixel): cls %ld px>1e-3 %ld max %.2e | DE/normal over: A %ld B %ld\n", ff_cls, ff_px, ff_max, ff_deA, ff_deB);
    printf("  tl raw (no guard): cls %ld px>1e-3 %ld max %.2e\n", tl_cls_raw, tl_px_raw, tl_max_raw);
    printf("  fallback: nu-only %.4f%% | contract A %.4f%% | contract B %.4f%%\n", 100.0 * fbN / cnt, 100.0 * fbA / cnt, 100.0 * fbB / cnt);
    printf("  final A (tl + ff fallback): cls %ld px>1e-3 %ld max %.2e DE %ld normal %ld\n", finA_cls, finA_px, finA_max, finA_de, finA_nm);
    printf("  final B (tl + ff fallback): cls %ld px>1e-3 %ld max %.2e DE %ld normal %ld\n", finB_cls, finB_px, finB_max, finB_de, finB_nm);
    for (int ct = 0; ct < 2; ct++)
        printf("  kept-only %s (= binary64 fallback arm): cls %ld px>1e-3 %ld max %.2e DE %ld normal %ld\n",
               ct ? "B" : "A", kcls[ct], kpx[ct], kmax[ct], kde[ct], knm[ct]);
    for (int ct = 0; ct < 2; ct++) {
        double hy = slots_tl + FF_STEP * (ct ? fbstB : fbstA);
        printf("  proj %s: hybrid/ff-only %.2fx | hybrid vs f64 RDNA1 %.2fx, vs f64 Ampere-consumer %.2fx | ff-only vs f64: %.2fx / %.2fx | fallback cost share %.1f%%\n",
               ct ? "B" : "A", sff / hy, s64_rdna / hy, s64_amp / hy, s64_rdna / sff, s64_amp / sff,
               100.0 * FF_STEP * (ct ? fbstB : fbstA) / hy);
    }
    return 0;
}
