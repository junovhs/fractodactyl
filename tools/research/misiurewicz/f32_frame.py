# Render one v0 mid-band frame twice with the probe pipeline: all-double and all-float (table jump),
# shade both the same way, and measure how different the pictures are.
import sys, io, os, contextlib, math, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from multiprocessing import Pool
W,H=160,90; WEXP=int(sys.argv[1]) if len(sys.argv)>1 else 15
def init():
    global FS
    a=sys.argv[:]; sys.argv=[a[0],'0']
    with contextlib.redirect_stdout(io.StringIO()):
        import float32_stages as FS_
    FS=FS_; sys.argv=a
def row(y):
    D,F=FS.C128,FS.C64
    pd={1:D,3:D,4:D,5:D,6:D,7:D,'add':D,'jump':'table'}; pf={1:F,3:F,4:F,5:F,6:F,7:F,'add':F,'jump':'table'}
    w=10.0**(-WEXP); out=np.zeros((2,W))
    for x in range(W):
        dc=complex((x+0.5-W/2)*w/W, -(y+0.5-H/2)*w/W)
        a=FS.pixel(dc,pd); b=FS.pixel(dc,pf)
        out[0,x]=-1 if a is None else a; out[1,x]=-1 if b is None else b
    return y,out
if __name__=='__main__':
    with Pool(2,initializer=init) as p:
        nu=np.zeros((2,H,W))
        for y,o in p.imap_unordered(row,range(H),chunksize=4): nu[:,y,:]=o
    np.save(f'nu_{WEXP}.npy',nu)
