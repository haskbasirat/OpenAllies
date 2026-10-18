"""Actual Object/Area/Camera/Doodad vtable exclusion, B4 and owner/parent leaves.
The direct source50 link is null; source self/other and real classification
are explicit probes. No positive generation link, wall, callback or shape is
replaced with a permissive answer.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P;H=lambda b:hashlib.sha256(b).hexdigest();VT={2:0xa2a2a0,6:0xa28a00,7:0xa28b88,9:0xa28e90}
def run(c,image):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(c['precision']);o=m.alloc(0x200);other=m.alloc(0x200);vt=VT[c['class']]
 m.w32(o,vt);m.w32(other,vt);m.w32(o+0x50,0)
 slot=c['slot'];target=h.P.u32(vt+slot);args=[o if c['self']else other]if slot==0x30 else[]
 result=m.call(target,o,args)
 if slot in (0x30,0xb4): result&=255
 assert result==(int(c['self'])if slot==0x30 else 0)
 assert m.final_esp==h.STACK+4+len(args)*4;assert m.fpu_top()==0
 return result
def main():
 rows=[];data=[]
 for cls,vt in VT.items():
  for slot in (0x30,0x100,0x108,0xb4):
   raw=ORIGINAL.range(vt+slot,4);assert raw==h.V.range(vt+slot,4);data.append(dict(address=hex(vt+slot),bytes=raw.hex()))
 for pc in h.PRECISION:
  for cls in VT:
   for slot,self_ in ((0x100,False),(0x108,False),(0xb4,False),(0x30,False),(0x30,True)):
    c=dict(precision=pc,**{'class':cls,'self':self_},slot=slot);a=run(c,'original');b=run(c,'v14');assert a==b;rows.append(dict(input=c,output=a))
 result=dict(schema=1,scope=__doc__,original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),cases=rows,native_executions=len(rows)*2,data_pins=data,instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 p=HERE/'native-object-dispatch.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=1)+'\n')
 else:assert json.loads(p.read_text())==result
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=H(p.read_bytes()))))
if __name__=='__main__':main()
