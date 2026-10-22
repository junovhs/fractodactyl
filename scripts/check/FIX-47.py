#!/usr/bin/env python3
"""Real elephant-valley compiler command and three unchanged AUTO-01 controls."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd',required=True)
p.add_argument('--native',required=True)
p.add_argument('--out',type=Path,required=True)
a=p.parse_args();root=Path(__file__).resolve().parents[2];here=root/'tools/research/misiurewicz';a.out.mkdir(parents=True,exist_ok=True)
rows=[]
for file in ['auto02_minibrot_pool.txt','auto02_controls.txt']:
    for line in (here/file).read_text().splitlines():
        if not line.strip() or line.startswith('#'):continue
        f=line.split('|')
        if file.endswith('controls.txt') or f[0]=='mb-elephant-valley-p112':rows.append(f)
reports=[]
for name,re,im,width,origin,iterations in rows:
    out=a.out/name
    cmd=[sys.executable,str(here/'auto_discover.py'),'--re='+re,'--im='+im,'--width='+width,
         '--fd',a.fd,'--native',a.native,'--size','960x540','--iter',iterations,'--threads','8','--out',str(out)]
    forced=name.startswith("calibration-")
    if forced: cmd.append("--force")  # existing research-only numerical/jump proof
    r=subprocess.run(cmd,cwd=root,text=True,capture_output=True,timeout=300)
    (a.out/(name+'.stdout')).write_text(r.stdout);(a.out/(name+'.stderr')).write_text(r.stderr)
    assert r.returncode==0,(name,r.returncode,r.stderr,r.stdout)
    report=json.loads((out/'report.json').read_text())
    assert report['pre_render_decision'] in ('accelerate','decline')
    if report['pre_render_decision']=='accelerate':
        assert report['jumps']>0,(name,report)
        assert report['retrospective_pass'],(name,report)
    if name.startswith('calibration-'):
        assert report['pre_render_decision']=='accelerate' and report['jumps']>0,(name,report)
    reports.append(dict(name=name,research_force=forced,**report))
(a.out/'summary.json').write_text(json.dumps(reports,indent=2)+'\n')
print(json.dumps([dict(name=r['name'],decision=r['pre_render_decision'],reason=r['reason'],jumps=r.get('jumps'),accepted=r.get('retrospective_pass')) for r in reports]))
