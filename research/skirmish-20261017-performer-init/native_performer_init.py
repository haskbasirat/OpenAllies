"""Actual57E000 ->57DCF0(NULL), actual4BE4D0/4BE4F0 and4B2830.

No successful seams. Changed enabled bytes stop BEFORE required4D1350 body;
all prior byte stores remain observable. Synthetic local definitions explicitly
supply their +60/+64 signed bounds. Actual constructor vtable assignment and
4C023D source GetInteger-to-field sequence are separately inspected/pinned,
not advertised as constructor/TGI-loader execution. Runtime binding uses the
same source Bank and existing actual Ability allocations, not these fixtures.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE,UC_MEM_READ,UC_MEM_WRITE
ORIGINAL=h.P;V14=h.V
class Required(Exception):pass

def run(case,image,pc):
 h.P=ORIGINAL if image=='original'else V14;h.VISITED={};m=h.Machine(pc)
 if case['kind']=='after':
  m.call(m.r32(0xa3df40+0x14),0xdead1000)
  return dict(outcome='ok',reads=[],writes=[]),h.VISITED.copy()
 p=m.alloc(0x70);owner=m.alloc(0x180);m.w32(p,0xa3df40);m.w32(p+4,owner)
 abilities=[];reads=[];writes=[]
 for spec in case['abilities']:
  ability=m.alloc(0x30);local=m.alloc(0x68)if spec['local']else 0
  m.w32(ability+4,0xdead2000) #Shared definition is NOT read by this function.
  m.w32(ability+8,local);m.w8(ability+0x18,int(spec['enabled']))
  if local:m.w32(local+0x60,spec['minimum']);m.w32(local+0x64,spec['maximum'])
  abilities.append((ability,local))
 array=m.alloc(max(4,4*len(abilities)))
 for i,(ability,_)in enumerate(abilities):m.w32(array+4*i,ability)
 m.w32(p+0x14,array);m.w32(p+0x18,len(abilities))
 def access(u,kind,address,size,value,_):
  if address==p+0x18 and kind==UC_MEM_READ:reads.append(['count',size])
  for i,(ability,local)in enumerate(abilities):
   if address==ability+4:raise AssertionError('unreached shared definition read')
   if address in [ability+8,ability+0x18]:
    if kind==UC_MEM_READ:reads.append([i,'local'if address==ability+8 else'enabled',size])
    else:writes.append([i,'enabled',value])
   if local and address in [local+0x60,local+0x64] and kind==UC_MEM_READ:
    reads.append([i,'minimum'if address==local+0x60 else'maximum',size])
 m.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,access)
 def stop(mm):
  assert mm.ecx()==owner
  raise Required('4D1350 actual held Actor actions required')
 m.seams[0x4d1350]=stop;returned=None;stopped=None
 try:returned=m.call(m.r32(0xa3df40+8),p)&0xff
 except Required as e:stopped=str(e)
 out=dict(outcome='required'if stopped else'ok',stopped=stopped,returned_al=returned,
  enabled=[m.r8(ability+0x18)for ability,_ in abilities],reads=reads,writes=writes)
 assert reads.count(['count',4])==1
 return out,h.VISITED.copy()

def cases():
 yield dict(kind='init',abilities=[])
 for local in [False,True]:
  for minimum,maximum in [(-1,-1),(-1,0),(0,-1),(1,-1),(-1,-2),(-2,-1),(0,0),(1,2),(-2,-2),(-2147483648,2147483647),(2147483647,-2147483648)]:
   for enabled in [False,True]:yield dict(kind='init',abilities=[dict(local=local,minimum=minimum,maximum=maximum,enabled=enabled)])
 yield dict(kind='init',abilities=[dict(local=True,minimum=1,maximum=-1,enabled=True),dict(local=True,minimum=0,maximum=0,enabled=False),dict(local=False,minimum=2147483647,maximum=-2147483648,enabled=True)])
 yield dict(kind='init',abilities=[dict(local=True,minimum=-1,maximum=-1,enabled=False),dict(local=True,minimum=-1,maximum=0,enabled=True)])
 yield dict(kind='after')

def main():
 rows=[];pins={}
 for pc in h.PRECISION:
  for case in cases():
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   rows.append(dict(input=case,precision=pc,output=a));pins.update(pa);pins.update(pb)
 executed=[]
 for va,raw in sorted(pins.items()):
  if va==0x4d1350:continue
  assert ORIGINAL.range(va,len(bytes.fromhex(raw)))==V14.range(va,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
  executed.append(dict(va=hex(va),bytes=raw))
 inspected=[]
 for va,size,scope in [(0x57f503,6,'Performer ctor actual vtable store'),(0xa3df48,4,'Performer virtual+8'),(0xa3df54,4,'Performer virtual+14'),(0x4c023d,0x25,'GetInteger required_experience_min/max tolocal60/64'),(0xa220d0,48,'minimum field literal'),(0xa220a0,48,'maximum field literal')]:
  raw=ORIGINAL.range(va,size);assert raw==V14.range(va,size);inspected.append(dict(va=hex(va),bytes=raw.hex(),scope=scope))
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,cases=rows,separate_machine_runs=2*len(rows),
  executed_instructions=executed,successful_seams=[],required_boundary=dict(va='0x4d1350',bytes=ORIGINAL.range(0x4d1350,6).hex()),inspected=inspected)
 path=HERE/'native-performer-init.json'
 if '--write'in sys.argv:
  with path.open('x',encoding='utf-8')as f:f.write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(rows=len(rows),separate_machines=2*len(rows),executed_pins=len(executed),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
