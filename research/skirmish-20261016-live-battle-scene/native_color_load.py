"""Unchanged47E0F0 KingdomColor LoadData RGB word stores.
The inherited name is empty. Localized-name lookup/assignment and GetPoint3
are explicit typed-field seams; full TGI parsing, constructor seed, manager
registration, default replacement color and renderer routing are not claimed.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
HASH=lambda b:hashlib.sha256(b).hexdigest()

def run(bits,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 obj=m.alloc(0x30);name=m.alloc(0x30)+0x10;node=m.alloc(0x40)
 m.w32(obj+0x10,name);m.w32(obj+0x18,0xcccccccc,0xdddddddd,0xeeeeeeee)
 calls=[]
 def getname(_):calls.append('GetLocalizedName');m.ret(4,name)
 def setname(_):calls.append('AssignLocalizedName');m.w32(m.ecx(),m.arg());m.ret(4,m.ecx())
 def point(_):calls.append('GetPoint3(color)');m.w32(m.arg(),*bits);m.ret(8,m.arg())
 m.seams.update({0x5b5620:getname,0x405b80:setname,0x5b4c30:point})
 m.call(0x47e0f0,obj,[node]);assert m.final_esp==h.STACK+8;assert m.fpu_top()==0
 return dict(bits=m.r32(obj+0x18,3),loaded=m.r8(obj+0x14),calls=calls)
def main():
 rows=[]
 values=[[0,0,0],[0x3f800000,0x3f800000,0x3f800000],[0x80000000,1,0x7f7fffff],
   [0,0x3e99999a,0x3f800000],[0x3f59999a,0,0x3dcccccd],
   [0x7fc12345,0x7f800001,0xffc54321],[0x3e99999a,0x3f3a9fbe,0x3f800000]]
 for pc in h.PRECISION:
  for bits in values:
   a=run(bits,'original',pc);b=run(bits,'v14',pc);assert a==b
   rows.append(dict(precision=pc,input=bits,output=a))
 result=dict(schema=1,scope=__doc__,original_sha256=HASH(ORIGINAL.b),v14_sha256=HASH(h.V.b),cases=rows,
  native_executions=len(rows)*2,instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 p=HERE/'native-color-load.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=1)+'\n')
 else:assert json.loads(p.read_text())==result
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=HASH(p.read_bytes()))))
if __name__=='__main__':main()
