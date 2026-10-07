"""PROB-08: score koenigs-bench frames against fd per-frame-BLA frames, pixel by pixel.

Error per escaped pixel in px: |nu_candidate - nu_fd| * ln2 * de_px, with fd's de column
(output pixels). Wrong: above 1e-3 px. Class mismatch: escaped in one, not the other.
Usage: python compare.py FD_DIR BENCH_DIR NXxNY WIDTH...   (fd frames in WIDTH order)
"""
import array, json, math, sys

def pad(x): return (x + 7)//8*8

def load_fd(path, n):
    # Columns (class u8, nu f64, de f32), each 8-byte aligned, after the header.
    b = open(path, 'rb').read(); cols = pad(n) + pad(8*n) + pad(4*n); h = len(b) - cols
    cls = b[h:h+n]; o = h + pad(n)
    nu = array.array('d'); nu.frombytes(b[o:o+8*n]); o += pad(8*n)
    de = array.array('f'); de.frombytes(b[o:o+4*n])
    return cls, nu, de

def main():
    fd_dir, bench_dir, size, widths = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
    nx, ny = map(int, size.split('x')); n = nx*ny
    ok = True
    for f, w in enumerate(widths):
        cls, nu, de = load_fd(f'{fd_dir}/frame-{f:05d}.fds', n)
        b = open(f'{bench_dir}/w{w}.bin', 'rb').read()
        kcls = b[:n]; knu = array.array('d'); knu.frombytes(b[n:n+8*n])
        mism = 0; errs = []
        for i in range(n):
            fesc = cls[i] & 3 == 0; kesc = kcls[i] == 0
            if fesc != kesc: mism += 1; continue
            if fesc: errs.append(abs(knu[i] - nu[i])*math.log(2)*de[i])
        errs.sort(); wrong = sum(e > 1e-3 for e in errs)
        ok &= wrong == 0 and mism == 0
        print(json.dumps(dict(width=w, pixels=n, compared=len(errs), class_mismatches=mism, wrong_px_gt_1e3=wrong,
                              max_px=errs[-1], p99_px=errs[int(.99*len(errs))], p50_px=errs[len(errs)//2])))
    print('CORRECTNESS', 'PASS' if ok else 'FAIL')


if __name__ == '__main__':
    main()
