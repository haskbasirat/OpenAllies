"""Native null-parent branches4CED60 and4CE9A0 on actual Actor/Kingdom vtables.
Structure absence and present Structure+10 null are explicit memory inputs;
Wall is absent. Positive parent/Wall states are not ported by the adapter.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P;H=lambda b:hashlib.sha256(b).hexdigest()
def run(c,image):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(c['precision']);o=m.alloc(0x200);structure=m.alloc(0x60)
 vt={3:0xa273f0,4:0xa299d8}[c['class']];m.w32(o,vt);m.w32(o+0x94,structure if c['structure']else 0);m.w32(o+0x90,0);m.w32(structure+4,o);m.w32(structure+0x10,0)
 target=h.P.u32(vt+c['slot']);result=m.call(target,o,[]);assert result==0;assert m.fpu_top()==0;assert m.final_esp==h.STACK+4
 return result
def main():
 rows=[]
 for pc in h.PRECISION:
  for cls in (3,4):
   for structure in (False,True):
    for slot in (0x100,0x15c):
     c=dict(precision=pc,**{'class':cls},structure=structure,slot=slot);a=run(c,'original');b=run(c,'v14');assert a==b;rows.append(dict(input=c,output=a))
 result=dict(schema=1,scope=__doc__,original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),cases=rows,native_executions=len(rows)*2,instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 p=HERE/'native-null-parents.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=1)+'\n')
 else:assert json.loads(p.read_text())==result
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=H(p.read_bytes()))))
if __name__=='__main__':main()
