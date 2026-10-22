#!/usr/bin/env python3
"""GPU-32 actual CLI frames: fixed corpus, every-pixel comparison, cold timings."""
import argparse
import json
from pathlib import Path
import re
import struct
import subprocess
import time

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd',type=Path,required=True)
p.add_argument('--out',type=Path,required=True)
a=p.parse_args();fd=a.fd.resolve();a.out.mkdir(parents=True,exist_ok=True)
root=Path(__file__).resolve().parents[2]

def run(args,stem):
    start=time.perf_counter()
    r=subprocess.run([str(fd),*map(str,args)],text=True,capture_output=True,cwd=root)
    wall=time.perf_counter()-start
    (a.out/f'{stem}.stdout').write_text(r.stdout)
    (a.out/f'{stem}.stderr').write_text(r.stderr)
    assert r.returncode==0,(stem,r.stderr,r.stdout)
    return r,wall

help_result,_=run(['--help'],'help')
assert all(t in help_result.stdout for t in ['experimental Heuristic','default off','currently slower'])

# Fixed before results: mixed shallow seahorse, cusp, exact deep valley, exterior
# control at 1e-8, and PROB-22 film frame 23. No zone files or tuning.
views=[
 ('seahorse','-0.7453','0.1127','0.0065'),
 ('cusp','0.25','0','1e-8'),
 ('seahorse-deep','-0.7453','0.1127','1e-14'),
 ('valley','-0.743643887037158704752191506114774','0.131825904205311970493132056385139','1e-28'),
 ('exterior','1','0.5','1e-8'),
 ('prob22-23','-0.8296874999999996981089858669738646984522311568651036619209','0.1859374999999989417838556112859064221737945709381783514212','2.3713737e-29'),
]
path=a.out/'fixed.path';path.write_text('\n'.join(f'{re_} {im} {width} 0' for _,re_,im,width in views)+'\n')
common=['control',path,'--size','128x72','--ss','1','--iter','10000','--columns','nu,de,normal','--threads','8']
base,bwall=run([*common,'-o',a.out/'cpu'],'cpu')
gpu,gwall=run([*common,'--gpu-perturb','on','-o',a.out/'gpu'],'gpu')
compare,_=run(['compare',a.out/'cpu',a.out/'gpu'],'compare')
records=[json.loads(l) for l in compare.stdout.splitlines() if l.startswith('{')]
assert records and all(r['ok'] for r in records),records
reports=[]
for line in gpu.stderr.splitlines():
    if 'fd gpu-perturb: adapter=' in line:
        reports.append({k:(float(v) if '.' in v else int(v)) for k,v in re.findall(r'(accepted|fallback|guard|unfinished|invalid|gpu_seconds|cpu_fallback_seconds|total_seconds)=([0-9.]+)',line)})
assert len(reports)==len(views),(gpu.stderr,reports)
assert sum(r['accepted'] for r in reports)>0,'No useful GPU escaped-sample coverage'
assert all(r['accepted']+r['fallback']==128*72 for r in reports)
# Discoverable on/off on direct render; unsupported bounded output returns CPU bytes.
render=['render','--re','1','--im','0.5','--width','1e-8','--size','32x18','--iter','256','--columns','nu,de,normal,bound','--threads','8']
run([*render,'-o',a.out/'bound-cpu.fds'],'bound-cpu')
bound,_=run([*render,'--gpu-perturb','on','-o',a.out/'bound-gpu.fds'],'bound-gpu')
assert 'frame declined (bound/scaled/range/grid); CPU restart from zero' in bound.stderr
assert (a.out/'bound-cpu.fds').read_bytes()==(a.out/'bound-gpu.fds').read_bytes()
run([*render[:-4],'--columns','nu,de,normal','--threads','8','--gpu-perturb','on','-o',a.out/'render.fds'],'render')
# Exterior admits every sample: actual stored evidence must remain Heuristic.
assert reports[4]['accepted']==128*72,reports[4]
files=sorted((a.out/'gpu').glob('*.fds'));assert len(files)==len(views),files
data=files[4].read_bytes();pos=56
for _ in range(4):
 n=struct.unpack_from('<H',data,pos)[0];pos+=2+n
body=(pos+7)&~7;classes=data[body:body+128*72]
assert len(classes)==128*72 and all((b&3)==0 and ((b>>2)&3)==0 for b in classes),'GPU output must be Escaped/Heuristic'
summary={'schema':'gpu32-evidence/1','views':[dict(name=v[0],re=v[1],im=v[2],width=v[3],**r) for v,r in zip(views,reports)],'cpu_process_wall_seconds':bwall,'gpu_process_wall_seconds':gwall,'compare':records,'limits':'one cold process per arm; 128x72 ss1 iter10000 8 CPU workers; all setup/GPU/CPU fallback included; no film or speed target claim'}
(a.out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary),flush=True)
