import sys, struct, array, os
def pad(x): return (x+7)//8*8
print(f"{'view':10} {'esc%':>5} | share of escaped PIXELS with de >= 32/64/128/256 px | share of ITERATIONS (nu-weighted) with de >= 32/64/128/256")
for f in sys.argv[1:]:
    b=open(f,'rb').read(); n=480*270
    cols = pad(n) + pad(8*n) + pad(4*n)
    h=len(b)-cols
    cls=b[h:h+n]
    nu=array.array('d'); nu.frombytes(b[h+pad(n):h+pad(n)+8*n])
    de=array.array('f'); de.frombytes(b[h+pad(n)+pad(8*n):h+pad(n)+pad(8*n)+4*n])
    esc=[k for k in range(n) if cls[k]&3==0]
    tot=sum(nu[k] for k in esc) or 1
    px=[sum(1 for k in esc if de[k]>=t)/len(esc) for t in (32,64,128,256)]
    it=[sum(nu[k] for k in esc if de[k]>=t)/tot for t in (32,64,128,256)]
    print(f"{os.path.basename(f)[:-4]:10} {100*len(esc)/n:5.1f} | " + " ".join(f"{100*x:5.1f}%" for x in px) + " | " + " ".join(f"{100*x:5.1f}%" for x in it) + f"   mean nu {tot/len(esc):.0f}")
