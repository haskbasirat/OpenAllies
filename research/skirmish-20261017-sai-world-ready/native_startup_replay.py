"""Fresh original/v14 7A0D40 outer-kernel replay; required child seams named.

This certifies the existing coordinator's direct-field-lease transformation,
not actual AI setup or complete live-world startup.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-readiness'))
import startup_oracle as old
import original_rng as base
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
H=lambda b:hashlib.sha256(b).hexdigest()
LEAVES={0x79f9b0,0x56a270,0x7abc30,0x79e170,0x7b4830,0x7a0c20,0x4ea430,0x7aa3d0,0x7b8ff0,0x7ba0d0}
def run(c,image):
 prior=base.original;base.original=old.original if image=='original' else old.modified
 try:m=old.Machine(c)
 finally:base.original=prior
 def guard(u,access,address,size,value,user):assert address>=0x1000,('null-page access',hex(address),size)
 m.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,guard)
 return m.run(),m.executed
def main():
 frozen=json.loads((ROOT/'research/phase7-readiness/native-startup.json').read_text());rows=[];executed={};leaves={}
 for c,prior in zip(old.cases(),frozen['fixtures'],strict=True):
  a,ap=run(c,'original');b,bp=run(c,'v14');assert a==b==prior['native']
  for va,n in (ap|bp).items():
   raw=old.original.range(va,n);assert raw==old.modified.range(va,n)
   (leaves if va in LEAVES else executed)[va]=raw.hex()
  rows.append(dict(input=c,output=a))
 deps=[ROOT/'research/phase7-readiness/startup_oracle.py',ROOT/'research/phase4-rng/original_rng.py',ROOT/'research/phase7-readiness/native-startup.json']
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),rows=rows,machines=len(rows)*2,
  original_sha256=H(old.original.b),v14_sha256=H(old.modified.b),
  executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(executed.items())],
  intercepted_entries=[dict(va=hex(a),bytes=b)for a,b in sorted(leaves.items())],
  virtual_seams=['Actor+B4/B0','seen-context+38'],dependencies=[dict(path=str(p.relative_to(ROOT)),sha256=H(p.read_bytes()))for p in deps])
 out=HERE/'native-startup.json';raw=(json.dumps(doc,indent=1)+'\n').encode()
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=len(rows)*2,executed_pins=len(executed),intercepted_entries=len(leaves),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
