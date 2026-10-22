#!/usr/bin/env bash
# Observe actual native platform jobs at the frozen SHA, including asynchronous startup.
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 - <<'PY'
import json,os,subprocess,time
from pathlib import Path
expected=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
path=Path(os.environ['TOOL03_CI_RUN_FILE'])
deadline=time.monotonic()+600
while time.monotonic()<deadline:
 if path.exists():
  run=path.read_text().strip()
  result=subprocess.run(['gh','run','view',run,'--repo','junovhs/fractodactyl','--json','headSha,status,conclusion,jobs,workflowName'],capture_output=True,text=True)
  if result.returncode==0:
   r=json.loads(result.stdout)
   assert r['headSha']==expected,('wrong tree',r['headSha'],expected)
   assert r['workflowName']=='fd-dev',r
   if r['status']=='completed':
    assert r['conclusion']=='success',r
    jobs={j['name']:j for j in r['jobs']}
    for name in ['report (ubuntu-latest)','report (windows-latest)']:
     j=jobs[name]
     assert j['conclusion']=='success',j
     assert any(s['name']=='Native report command parity and clean error behavior' and s['conclusion']=='success' for s in j['steps']),j
    print('TOOL-03 native Linux and Windows CLI parity/error/Clippy passed at',expected)
    break
 print('Waiting for actual native CI at',expected,flush=True)
 time.sleep(5)
else:raise SystemExit('Native CI did not complete successfully within10minutes')
PY
