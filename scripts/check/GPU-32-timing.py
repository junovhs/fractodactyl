#!/usr/bin/env python3
"""Bounded actual GPU32 timings; same frozen cameras, explicit resolution/indices."""
import argparse,ast,json,re,subprocess,time
from decimal import Decimal,localcontext
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--fd',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--size',required=True);p.add_argument('--indices',default='5');p.add_argument('--only-film',action='store_true');a=p.parse_args();a.fd=a.fd.resolve();a.out.mkdir(parents=True,exist_ok=True)
module=ast.parse(Path('scripts/check/GPU-32.py').read_text());views=next(ast.literal_eval(n.value) for n in module.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='views' for t in n.targets))
def run(args,stem):
 t=time.perf_counter();r=subprocess.run([str(a.fd),*map(str,args)],capture_output=True,text=True);wall=time.perf_counter()-t
 (a.out/f'{stem}.stdout').write_text(r.stdout);(a.out/f'{stem}.stderr').write_text(r.stderr)
 assert r.returncode==0,(stem,r.returncode,r.stderr[-3000:]);return r,wall
summary={'size':a.size,'workers':8,'scope':'single cold actual process per arm; complete process wall includes encoding FDS/setup/fallback; film compute interval separately includes reference/BLA/series but excludes shade/encode; no GPU resident runtime implemented','views':[]}
for index in map(int,a.indices.split(',')):
 name,re_,im,width=views[index];path=a.out/f'{name}.path';path.write_text(f'{re_} {im} {width} 0\n');arms={}
 common=['control',path,'--size',a.size,'--ss','1','--iter','10000','--columns','nu,de,normal','--threads','8']
 for arm,extra in ([] if a.only_film else [('plain',[]),('bla',['--bla','per-frame']),('gpu',['--gpu-perturb','on'])]):
  r,wall=run([*common,*extra,'-o',a.out/f'{name}-{arm}'],f'{name}-{arm}')
  records=[json.loads(l) for l in r.stdout.splitlines() if l.startswith('{')];armdata={'process_wall_seconds':wall,'report':records}
  if arm=='gpu':armdata['gpu_reports']=[{k:float(v) for k,v in re.findall(r'(accepted|fallback|guard|unfinished|invalid|gpu_seconds|cpu_fallback_seconds|total_seconds)=([0-9.]+)',line)} for line in r.stderr.splitlines() if 'adapter=' in line]
  arms[arm]=armdata;print(name,arm,wall,flush=True)
 for arm in ([] if a.only_film else ['bla','gpu']):
  r,wall=run(['compare',a.out/f'{name}-plain',a.out/f'{name}-{arm}'],f'{name}-{arm}-compare');arms[arm]['compare']=[json.loads(l) for l in r.stdout.splitlines() if l.startswith('{')]
 with localcontext() as ctx:ctx.prec=1000;to=str(Decimal(width)/2)
 film=['film',re_,im,'--from',width,'--to',to,'--seconds','1','--fps','2','--frames','0','--size',a.size,'--ss','1','--iter','10000','--threads','8','--keyframes','off','--aa','off','--unresolved','mark','--gpu','off','--x264','ultrafast','--json','--mp4',a.out/f'{name}-film.mp4']
 r,wall=run(film,f'{name}-film');arms['landed-film-auto']={'process_wall_seconds':wall,'report':json.loads(r.stdout)};print(name,'film',wall,flush=True)
 summary['views'].append(dict(name=name,camera=[re_,im,width],arms=arms));(a.out/('summary-film.json' if a.only_film else 'summary.json')).write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary),flush=True)
