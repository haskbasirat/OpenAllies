"""Read-only replay of accepted native-runtime rows on both PE images.
The base image is selected BEFORE fixture initialization. The old fixture and
producer are preserved; page0 has the current strict FS-only access guard.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-animation-runtime'));import native_runtime as n
import original_rng as rng
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
path=ROOT/'research/phase7-animation-runtime/native-runtime.json'
f=json.loads(path.read_text());before=hashlib.sha256(path.read_bytes()).hexdigest();images={"original":rng.original,"v14":rng.modified};count=0
for image,pe in images.items():
 rng.original=pe
 for index,row in enumerate(f['cases']):
  m=n.Machine(row['input']);m.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,lambda u,access,a,size,value,user:h.page_zero_access(u,a,size),begin=0,end=0xfff)
  assert m.run()==row['output'],(image,index);count+=1
assert hashlib.sha256(path.read_bytes()).hexdigest()==before
out=dict(schema=1,scope=__doc__,executions=count,fixture_sha256=before,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,all_rows_match=True)
(HERE/'existing-animation-replay.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out))
