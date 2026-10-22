#!/usr/bin/env python3
"""GPU33 fixed whole-frame/usage probe; cameras unchanged from GPU32."""
import argparse,ast,json,re,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--fd',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--size',default='128x72');p.add_argument('--indices',default='0,1,2,3,4,5');a=p.parse_args();a.fd=a.fd.resolve();a.out.mkdir(parents=True,exist_ok=True)
module=ast.parse(Path('scripts/check/GPU-32.py').read_text());views=next(ast.literal_eval(n.value) for n in module.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='views' for t in n.targets))
selected=[views[int(i)] for i in a.indices.split(',')];path=a.out/'fixed.path';path.write_text(''.join(f'{v[1]} {v[2]} {v[3]} 0\n' for v in selected))
def run(args,stem):
 t=time.perf_counter();r=subprocess.run([str(a.fd),*map(str,args)],text=True,capture_output=True);wall=time.perf_counter()-t
 (a.out/f'{stem}.stdout').write_text(r.stdout);(a.out/f'{stem}.stderr').write_text(r.stderr);assert r.returncode==0,(stem,r.stderr,r.stdout);return r,wall
help_result,_=run(['--help'],'help')
assert all(t in help_result.stdout for t in ['on|bla|off','experimental Heuristic','default off','currently slower','original plain CPU'])
render=['render','--re','1','--im','0.5','--width','1e-8','--size','32x18','--iter','256','--columns','nu,de,normal,bound','--threads','8']
run([*render,'-o',a.out/'bound-cpu.fds'],'bound-cpu')
bound,_=run([*render,'--gpu-perturb','bla','-o',a.out/'bound-gpu.fds'],'bound-gpu')
assert 'frame declined (bound/scaled/range/grid); CPU restart from zero' in bound.stderr
assert (a.out/'bound-cpu.fds').read_bytes()==(a.out/'bound-gpu.fds').read_bytes()
common=['control',path,'--size',a.size,'--ss','1','--iter','10000','--columns','nu,de,normal','--threads','8'];summary={'schema':'gpu33-probe/1','size':a.size,'views':selected,'scope':'complete cold processes, same camera/AA/iterations/columns; all table/setup/upload/fallback included','arms':{}}
for name,extra in [('plain',[]),('cpu-bla',['--bla','per-frame']),('gpu-on',['--gpu-perturb','on']),('gpu-bla',['--gpu-perturb','bla'])]:
 r,wall=run([*common,*extra,'-o',a.out/name],name);data={'process_seconds':wall}
 if name.startswith('gpu'):
  data['gpu']=[{k:float(v) for k,v in re.findall(r'(accepted|fallback|guard|unfinished|invalid|gpu_seconds|cpu_fallback_seconds|total_seconds)=([0-9.]+)',l)} for l in r.stderr.splitlines() if 'adapter=' in l]
 if name=='gpu-bla':
  data['la']=[{k:float(v) for k,v in re.findall(r'(blocks|skipped|table_blocks|table_bytes|table_seconds)=([0-9.]+)',l)} for l in r.stderr.splitlines() if 'fd gpu-bla:' in l]
  assert len(data['la'])==len(selected),r.stderr
  assert sum(x['blocks'] for x in data['la'])>0,'no actual BLA blocks used'
 summary['arms'][name]=data;print(name,wall,flush=True)
for arm in ['cpu-bla','gpu-on','gpu-bla']:
 r,_=run(['compare',a.out/'plain',a.out/arm],f'compare-{arm}');rows=[json.loads(l) for l in r.stdout.splitlines() if l.startswith('{')];assert rows and all(x['ok'] for x in rows),rows;summary['arms'][arm]['comparison']=rows
# The fixed exterior control admits every sample; stored outputs stay Heuristic.
if len(selected)==6:
 import struct
 files=sorted((a.out/'gpu-bla').glob('*.fds'));data=files[4].read_bytes();pos=56
 for _ in range(4):n=struct.unpack_from('<H',data,pos)[0];pos+=2+n
 body=(pos+7)&~7;count=128*72
 assert summary['arms']['gpu-bla']['gpu'][4]['accepted']==count
 assert all((b&3)==0 and ((b>>2)&3)==0 for b in data[body:body+count])
(a.out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary),flush=True)
