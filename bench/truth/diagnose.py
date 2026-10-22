"""Independently validate exact fixed-corpus failures; no exclusions."""
import generate
from mpmath import mp
for idx in [318,988]:
 c=next(c for c in generate.cases() if c['name']=='filament')
 for guard in [160,416,672]:
  row=generate.point((c,idx,guard))
  print('point',idx,'guard',guard,'oracle',row,flush=True)
 mp.prec=800
 h=mp.mpf(c['width'])/c['nx'];x,y=idx%c['nx'],idx//c['nx']
 re=mp.mpf(c['re'])+h*(mp.mpf(x)+mp.mpf('.5')-mp.mpf(c['nx'])/2)
 im=mp.mpf(c['im'])-h*(mp.mpf(y)+mp.mpf('.5')-mp.mpf(c['ny'])/2)
 print('exact_camera',idx,mp.nstr(re,100),mp.nstr(im,100),flush=True)

# Actual Plane F64 construction: reference center and delta are binary64 inputs.
# Extract exact binary values rather than rounding their sum to binary64 again.
for idx in [318,988]:
 mp.prec=800
 c=next(c for c in generate.cases() if c['name']=='filament')
 h=mp.mpf(c['width'])/32;x,y=idx%32,idx//32
 exact_r=mp.mpf(c['re'])+h*(x+.5-16);exact_i=mp.mpf(c['im'])-h*(y+.5-16)
 # width's leading64-bit fixed representation rounds to the same binary64 as this
 # width. The Rust Plane bit report independently checks these actual inputs.
 hf=float(c['width'])/32
 cr,ci=float(c['re']),float(c['im']);ar,ai=(x+.5-16)*hf,-(y+.5-16)*hf
 rounded_r=mp.mpf(cr)+mp.mpf(ar);rounded_i=mp.mpf(ci)+mp.mpf(ai)
 shift=mp.sqrt((rounded_r-exact_r)**2+(rounded_i-exact_i)**2)/h
 res=generate.oracle.direct(rounded_r,rounded_i,c['cap'],mp.mpf('1e20'))
 import struct
 bits=lambda v:struct.unpack('<Q',struct.pack('<d',v))[0]
 print('rounded_input',idx,'bits',[bits(v) for v in [cr,ci,ar,ai]],'shift_px',mp.nstr(shift,30),'oracle_escape',None if res is None else res[0],flush=True)
