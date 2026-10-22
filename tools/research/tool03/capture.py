"""Capture a real tiny report-compatibility run, never a performance benchmark."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time

p=argparse.ArgumentParser();p.add_argument('--fd',required=True);p.add_argument('--legacy-reporter',required=True);p.add_argument('--out',required=True,type=Path);a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
root=Path.cwd();path=a.out/'path.txt'
# 76 tiny frames exercise both full and partial 75-frame reporting bins.
r=subprocess.run([a.fd,'path','zoom','--re','-0.75','--im','0.1','--from','4','--to','1e-20','--seconds','76','--fps','1'],capture_output=True,check=True)
path.write_bytes(r.stdout)
commands=[]
def run(name,args,timed=False):
 cmd=[a.fd,*map(str,args)];commands.append(cmd)
 with (a.out/'phases.log').open('a') as f:f.write(f'{time.time():.6f} {name}\n')
 with (a.out/'load.log').open('a') as f:f.write(f'{time.time():.6f} {os.getloadavg()[0]}\n')
 stdout=a.out/(name+'.jsonl' if name in ['A','B','C'] or name.startswith('cmp') else name+'.log')
 actual=['/usr/bin/time','-v','-o',str(a.out/(name+'.time')),*cmd] if timed else cmd
 result=subprocess.run(actual,capture_output=True);stdout.write_bytes(result.stdout);(a.out/(name+'.stderr')).write_bytes(result.stderr)
 if timed:
  with (a.out/(name+'.time')).open('a') as f:f.write(f'exit {result.returncode}\n')
 if result.returncode:raise RuntimeError((name,result.returncode,result.stderr.decode()[-2000:]))
common=['--size','8x8','--iter','32','--columns','nu,de,normal']
for arm,extra in [('A',[]),('B',['--bla','per-frame'])]:
 run(arm,['control',path,*common,'--threads','1','--runs','1','--oracle',root/'tools/oracle.py','--every','75','--k','2','-o',a.out/arm,*extra],True)
run('C.compile',['compile',path,'--store',a.out/'store',*common,'--workers','1'],True)
run('C',['play',a.out/'C.compile.log','--store',a.out/'store','--threads','1','--look','umber','--oracle',root/'tools/oracle.py','--every','75','--k','2','-o',a.out/'C'],True)
for pair in ['AB','AC','BC']:run('cmp'+pair,['compare',a.out/pair[0],a.out/pair[1]])
r=subprocess.run(['python3',a.legacy_reporter,str(a.out)],capture_output=True,check=True);(a.out/'expected.json').write_bytes(r.stdout)
(a.out/'capture.json').write_text(json.dumps(dict(kind='fresh tiny real compatibility capture; NOT BENC01 rerun or performance evidence',frames=76,size='8x8',max_iter=32,threads=1,commands=commands),indent=2)+'\n')
print('Captured actual A/B/compile/play/compare and old Python expected report:',a.out,flush=True)
