"""Actual50ED15..50ED38 queued-Kingdom loop.4F53D0 is an explicit required
destruction boundary and is NOT executed here. Private vector allocation and
pointer identities are fixture inputs; loop order/zero argument/error prefix
are proven, not class4 destruction or full ConfigureGWorld completion.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
class Stop(Exception):pass
def run(image,pc,c):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);events=[];completed=0;final=False;failure=False
 world=m.alloc(0x200);m.w32(0xb36534,world);array=m.alloc(max(1,len(c['ids']))*4)
 ids={}
 for i,id in enumerate(c['ids']):
  p=next((p for p,v in ids.items()if v==id),None)or m.alloc(0x400);ids[p]=id;m.w32(array+i*4,p)
 def remove(_):
  nonlocal completed,failure
  assert m.ecx()==world and m.arg(1)==0
  events.append(ids[m.arg()])
  if len(events)==c['fail_call']:failure=True;raise Stop()
  completed+=1;m.ret(8)
 def tail(_):
  nonlocal final
  final=True;raise Stop()
 m.seams[0x4f53d0]=remove;m.seams[0x50ed38]=tail
 shim=m.stub_next;m.stub_next+=64
 code=b'\x31\xdb\x83\xec\x50\xc7\x44\x24\x38'+array.to_bytes(4,'little')
 code+=b'\xc7\x44\x24\x3c'+len(c['ids']).to_bytes(4,'little')+b'\xb8'+(0x50ed15).to_bytes(4,'little')+b'\xff\xe0'
 m.u.mem_write(shim,code)
 try:m.call(shim)
 except Stop:pass
 return {'attempted':events,'completed':completed,'final_iterator_reached':final,'failed':failure}
def main():
 rows=[]
 for pc,ids,fail in itertools.product(h.PRECISION,[[],[1677],[1677,1678],[17,17,19],[1677,1678,1679,1680,1681]],[None,1,2]):
  c={'ids':ids,'fail_call':fail};a=run('original',pc,c);b=run('v14',pc,c);assert a==b
  rows.append({'precision':pc,'input':c,'output':a})
 pins=[]
 for a,b in sorted(h.VISITED.items()):
  if a in (0x4f53d0,0x50ed38):continue
  raw=bytes.fromhex(b);assert raw==ORIGINAL.range(a,len(raw))==h.V.range(a,len(raw));pins.append({'va':hex(a),'bytes':b})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'executed_instructions':pins,
  'not_executed_boundaries':['0x4f53d0','0x50ed38']}
 p=HERE/'native-configure-removal-order.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
