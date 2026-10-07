import sys, array, math
def pad(x): return (x+7)//8*8
def load(f, n=480*270):
    b=open(f,'rb').read(); cols=pad(n)+pad(8*n)+pad(4*n)+pad(2*n); h=len(b)-cols
    cls=b[h:h+n]; o=h+pad(n)
    nu=array.array('d'); nu.frombytes(b[o:o+8*n]); o+=pad(8*n)
    de=array.array('f'); de.frombytes(b[o:o+4*n])
    return cls,nu,de
A=load(sys.argv[1]); B=load(sys.argv[2]); n=480*270; P=float(sys.argv[3])
mism=sum(1 for k in range(n) if (A[0][k]&3)!=(B[0][k]&3))
# displacement in output px: |dnu| * ln2 * de  (|grad nu| ~ 1/(ln2*de) px^-1)
disp=sorted(abs(B[1][k]-A[1][k]-P)*math.log(2)*max(A[2][k],1e-9) for k in range(n) if A[0][k]&3==0 and B[0][k]&3==0)
m=len(disp)
print(f"class mismatch {mism}; nu-shift-{P:g} as px displacement: p50 {disp[m//2]:.1e}  p99 {disp[int(.99*m)]:.1e}  p99.99 {disp[int(.9999*m)]:.1e}  max {disp[-1]:.1e}")
