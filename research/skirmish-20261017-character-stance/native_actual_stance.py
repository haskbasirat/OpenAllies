"""Native stance over actual same-owner Rust source/constructor captures.
Only explicit Character/animation/scalar inputs are projected into the raw
native owner proof; this is not a full InitCreate/mission execution claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));from native_stance_owner import run,h

def main():
 capture=HERE/'actual-stance-inputs.json';inputs=json.loads(capture.read_text());rows=[];visited={};seams=set()
 for c in inputs:
  pc=c['precision'];a,pa,sa=run('original',pc,c['input']);b,pb,sb=run('v14',pc,c['input']);assert a==b
  assert a['state']==c['state'],(c['actor'],pc,a['state'],c['state'])
  rows.append(dict(actor=c['actor'],precision=pc,output=a));visited.update(pa);visited.update(pb);seams|=sa|sb
 pins=[]
 for va,n in sorted(visited.items()):
  if va in seams:continue
  raw=h.P.range(va,n);assert raw==h.V.range(va,n);pins.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,capture_sha256=hashlib.sha256(capture.read_bytes()).hexdigest(),producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2,instructions=pins,seams=[hex(a)for a in sorted(seams)])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-actual-stance.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
