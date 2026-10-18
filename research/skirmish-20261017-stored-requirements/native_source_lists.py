"""4C51E0 and actual head insertion4C54B0/4C5720, plus shared vf18.

Controlled canonical lookup/allocator transports only. Does not execute the
entire source loader/deferred linker or reinterpret byte5C as virtual18.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
P,V=h.P,h.V
SEAMS={0x463fa0:'canonical Property manager lookup transport',0x4bdba0:'canonical Actor manager lookup transport',0x6f7810:'heap allocation transport'}
def run(image,pc,count,technology):
 h.P=image;m=h.Machine(pc);requirements=m.alloc(0x20);m.u.mem_write(requirements,b'\xa5'*0x20)
 m.call(0x4c51e0,requirements);assert m.r32(requirements,6)==[0,0,0xa5a5a500,0,0,0xa5a5a500]
 game=m.alloc(0x500);m.w32(0xb36530,game);m.w32(game+0xc,m.alloc(0x50));m.w32(game+0x464,m.alloc(0x50))
 definitions=[m.alloc(0x200)for _ in range(count)];labels={value:i+1 for i,value in enumerate(definitions)}
 for value in definitions:m.w32(value+0x174,1<<13)
 current=[0];m.seams[0x463fa0]=lambda x:x.ret(4,current[0]);m.seams[0x4bdba0]=lambda x:x.ret(8,current[0]);m.seams[0x6f7810]=lambda x:x.ret(0,x.alloc(x.arg(0)))
 string=m.alloc(4);m.w32(string,m.alloc(2));owner=m.alloc(4);m.w32(owner,m.alloc(2))
 for current[0]in definitions:
  m.call(0x4c54b0,requirements,[string,owner]);assert m.final_esp==h.STACK+12
  m.call(0x4c5720,requirements,[string,owner]);assert m.final_esp==h.STACK+12
 def order(offset):
  head=m.r32(requirements+offset);values=[];last=0
  while head:values.append(labels[m.r32(head)]);last=head;head=m.r32(head+4)
  assert last==m.r32(requirements+offset+4)
  return values
 #Actual shared PropertyData ctor's vtable store and descriptor are pinned;
 #only its exact virtual leaf executes here, byte5C is deliberately varied.
 assert image.range(0x47d934,6)==bytes.fromhex('c70634c8a000')
 assert image.u32(0xa0c834+0x18)==0x6a71d0
 prop=m.alloc(0x80);m.w32(prop,0xa0c834);m.u.mem_write(prop+0x5c,bytes([technology]));reads=[]
 m.u.hook_add(UC_HOOK_MEM_READ,lambda u,a,address,size,value,data:reads.append(address)if address==prop+0x5c else None)
 shared=bool(m.call(image.u32(0xa0c834+0x18),prop)&255);assert shared and reads==[]
 return dict(property_order=order(0),structure_order=order(0xc),any_property=False,any_actor=False,shared_type=shared,byte5c_reads=0)
def main():
 h.VISITED.clear();rows=[]
 for pc in h.PRECISION:
  for count in range(4):
   for technology in [0,1,255]:
    a=run(P,pc,count,technology);b=run(V,pc,count,technology);assert a==b
    rows.append(dict(precision=pc,count=count,byte5c=technology,output=a))
 h.P=P
 pins=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())if a not in SEAMS]
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,pins=pins,
  boundary_pins=[dict(address=hex(a),bytes=h.VISITED[a],seam=name)for a,name in SEAMS.items()],
  descriptor_pins=[dict(address='0x47d934',bytes=P.range(0x47d934,6).hex()),dict(address=hex(0xa0c834+0x18),bytes=P.range(0xa0c834+0x18,4).hex())])
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-source-lists.json'
 if '--write'in sys.argv:
  with p.open('xb')as f:f.write(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),executed_pins=len(pins),child_seams=len(SEAMS),sha256=hashlib.sha256(data).hexdigest(),result='PASS')))
if __name__=='__main__':main()
