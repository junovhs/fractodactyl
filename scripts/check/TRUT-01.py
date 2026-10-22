"""Real comparator checks: timing, frozen provenance and fail-closed corruptions."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time

p=argparse.ArgumentParser()
p.add_argument('--binary',required=True)
p.add_argument('--out',required=True,type=Path)
a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
pack=Path('bench/truth/v1')
provenance=json.loads((pack/'provenance.json').read_text())
for field,path in [('oracle_sha256',Path('tools/oracle.py')),('generator_sha256',Path('bench/truth/generate.py'))]:
    assert hashlib.sha256(path.read_bytes()).hexdigest()==provenance[field],field
for name,digest in json.loads((pack/'data-sha256.json').read_text()).items():
    assert hashlib.sha256((pack/name).read_bytes()).hexdigest()==digest,name

def run(name,path,good):
    started=time.perf_counter()
    r=subprocess.run([a.binary,str(path),'8'],capture_output=True)
    seconds=time.perf_counter()-started
    (a.out/f'{name}.stdout').write_bytes(r.stdout);(a.out/f'{name}.stderr').write_bytes(r.stderr)
    assert (r.returncode==0)==good,(name,r.returncode,r.stderr.decode()[-2000:])
    return r,seconds

r,wall=run('positive',pack,True)
rows=[json.loads(l) for l in r.stdout.splitlines()]
summary=rows[-1]
assert summary['cases']==5 and summary['points']==5120
assert wall<10 and summary['seconds']<10,(wall,summary)
assert summary['raw_class_mismatches']==summary['known_precision_edges']==4
assert summary['unexpected_class_mismatches']==summary['scalar_failures']==0
assert summary['bla_blocks']>0 and summary['bla_unavailable_cases']==1
fixed=[row for row in rows[:-1] if row['route'].startswith('fixed-recheck')]
assert len(fixed)==2 and all(row['raw_class_mismatches']==row['scalar_failures']==0 for row in fixed)
assert any(row['case']=='extreme' and row['route']=='bla-unavailable-plain-fallback' for row in rows[:-1])

def fixture(name):
    path=a.out/name
    shutil.rmtree(path,ignore_errors=True);shutil.copytree(pack,path)
    return path

def mutate_point(path,file,index,change):
    f=path/file;lines=f.read_text().splitlines()
    for n,line in enumerate(lines):
        if line.startswith(str(index)+'\t'):
            fields=line.split('\t');change(fields);lines[n]='\t'.join(fields);break
    else:raise AssertionError('missing point')
    f.write_text('\n'.join(lines)+'\n')

# These go through the compiled loader and real renderer, not just checksum checks.
path=fixture('bad-class')
def wrong_class(f):f[1]='U';f[2]='200'
mutate_point(path,'exterior.tsv',0,wrong_class);run('bad-class',path,False)
path=fixture('bad-scalar')
mutate_point(path,'exterior.tsv',0,lambda f:f.__setitem__(3,str(float(f[3])+1000)));run('bad-scalar',path,False)
# A known F64 edge cannot hide a wrong value: mandatory Fixed rechecks must fail.
path=fixture('bad-fixed-scalar')
mutate_point(path,'filament.tsv',318,lambda f:f.__setitem__(3,str(float(f[3])+1e15)));run('bad-fixed-scalar',path,False)
for name,column in [('bad-input-bits',9),('bad-edge-index',1)]:
    path=fixture(name);f=path/'precision-edges.txt';lines=f.read_text().splitlines();fields=lines[2].split('\t');fields[column]=str(int(fields[column])+1);lines[2]='\t'.join(fields);f.write_text('\n'.join(lines)+'\n');run(name,path,False)
path=fixture('bad-edge-pattern')
mutate_point(path,'filament.tsv',318,lambda f:(f.__setitem__(1,'U'),f.__setitem__(2,'2000')));run('bad-edge-pattern',path,False)
summary['subprocess_wall_seconds']=wall
summary['negative_checks']=6
(a.out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary))
