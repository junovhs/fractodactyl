import sys, array, math
def pad(x): return (x+7)//8*8
def load(f, n=480*270):
    b=open(f,'rb').read(); cols=pad(n)+pad(8*n)+pad(4*n)+pad(2*n); h=len(b)-cols
    cls=b[h:h+n]; o=h+pad(n)
    nu=array.array('d'); nu.frombytes(b[o:o+8*n]); o+=pad(8*n)
    de=array.array('f'); de.frombytes(b[o:o+4*n]); o+=pad(4*n)
    nm=array.array('H'); nm.frombytes(b[o:o+2*n])
    return cls,nu,de,nm
A=load(sys.argv[1]); B=load(sys.argv[2]); n=480*270
mism=sum(1 for k in range(n) if (A[0][k]&3)!=(B[0][k]&3))
d=[B[1][k]-A[1][k] for k in range(n) if A[0][k]&3==0 and B[0][k]&3==0]
d.sort(); med=d[len(d)//2]
dev=[abs(x-med) for x in d]; dev.sort()
der=[abs(B[2][k]/A[2][k]-1) for k in range(n) if A[0][k]&3==0 and B[0][k]&3==0 and A[2][k]>0]; der.sort()
dn=[((B[3][k]-A[3][k])%65536)*360/65536 for k in range(n) if A[0][k]&3==0 and B[0][k]&3==0]; dn.sort()
print(f"{sys.argv[2]}: class mismatches {mism}/{n}; nu offset median {med:.9f}, |nu - offset| p50 {dev[len(dev)//2]:.2e} p99 {dev[int(.99*len(dev))]:.2e} max {dev[-1]:.2e}; de rel diff p50 {der[len(der)//2]:.2e} max {der[-1]:.2e}; normal shift median {dn[len(dn)//2]:.4f} deg")
