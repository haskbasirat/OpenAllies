"""Actual42A590→429080 null head with byte0 used by both fresh callers.
Native79E130 initializes each selected pair; actual position words are read
twice before return, including raw sNaN/sign bits. No World read or child
seam, no actual Actor constructor/positive FX/full caller execution claim.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[3]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
P,V=h.P,h.V;H=lambda b:hashlib.sha256(b).hexdigest()
def run(image,pc,offset,position):
 h.P=image;m=h.Machine(pc);definition=m.alloc(0x200);actor=m.alloc(0x80);head=definition+offset
 m.w32(head,0xaaaaaaaa,0xbbbbbbbb);m.call(0x79e130,head);assert m.r32(head,2)==[0,0]
 m.w32(actor+0x20,*position);m.w32(0xb36534,0);reads=[];world=[]
 def observed(u,access,address,size,value,data):
  if actor+0x20<=address<actor+0x2c:reads.append([address-actor,size])
  if address==0xb36534:world.append(address)
 m.u.hook_add(UC_HOOK_MEM_READ,observed)
 m.call(0x42a590,head,[actor,0]);assert m.final_esp==h.STACK+12 and m.fpu_top()==0
 assert reads==[[0x20,4],[0x24,4],[0x28,4]]*2 and world==[]
 assert m.r32(actor+0x20,3)==position and m.r32(head,2)==[0,0]
 return dict(position_reads=reads,world_reads=0,unchanged_raw_position=m.r32(actor+0x20,3),null_pair=m.r32(head,2))
def main():
 h.VISITED.clear();rows=[]
 for pc,offset,pos in itertools.product(h.PRECISION,[0xbc,0xe4],[[0x3f800000,0x40000000,0x40400000],[0x7f800001,0x80000000,0xffb30012]]):
  a=run(P,pc,offset,pos);b=run(V,pc,offset,pos);assert a==b
  rows.append(dict(precision=pc,list_offset=offset,flag=0,output=a))
 h.P=P;pins={hex(a):v for a,v in sorted(h.VISITED.items())}
 for a,v in pins.items():assert P.range(int(a,16),len(bytes.fromhex(v)))==V.range(int(a,16),len(bytes.fromhex(v)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),machine_sha256=H(Path(h.__file__).read_bytes()),rows=rows,pins=pins,seams=[])
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-null-flag-zero.json'
 if '--write'in sys.argv:
  with p.open('xb')as f:f.write(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=H(data))))
if __name__=='__main__':main()
