"""Unchanged original/v14 numeric callbacks over captured actual saved fields.

Runtime probes alter the same live modifier scalars explicitly. Source fields,
component presence and prior values are captured; no unconsumed missing field is
filled with a gameplay default. Native-only guard probes are labeled separately.
"""
from pathlib import Path
import sys,json,hashlib,copy,importlib.util
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
spec=importlib.util.spec_from_file_location('property_instances_v1',ROOT/'research/skirmish-20261016-load-property-instances/native_property_instances.py')
v1=importlib.util.module_from_spec(spec);spec.loader.exec_module(v1)

def image(m,name):
 if name=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])

def provided(row,name,pc):
 """Reuse frozen constructor capture; replace only its unavailable clamp seam.

 The real 50252F instruction and cache-pointer dereference execute. The current
 cell value comes from the same captured runtime owner, not the declaration.
 """
 factory=nm.Machine
 def machine(p):
  m=factory(p);observe=m.observe
  def register(address,callback):
   if address==0x50252f:
    cache=m.alloc(0x200);cell=m.alloc(4);m.w(cell,row['input']['minimum_multiply'])
    m.w(cache+0x1a8,cell);m.w(0xb39ab0,cache)
   else:observe(address,callback)
  m.observe=register
  return m
 nm.Machine=machine
 try:return v1.run(row,name,pc)
 finally:nm.Machine=factory

def run(c,name,pc):
 m=nm.Machine(pc);image(m,name)
 actor=m.alloc(0x400);data=m.alloc(0x600);descriptor=m.alloc(0x20);modifier=m.alloc(0xa0)
 m.w(actor+4,data);m.w(actor+0x14,c.get('owner',0));m.w(descriptor+0x10,c['index'])
 for i,bits in enumerate(c['modifier']):m.w(modifier+4+4*i,bits)
 cache=m.alloc(0x200);add=m.alloc(4);m.w(add,c['clamp_add']);m.w(cache+0x1a4,add);m.w(0xb39ab0,cache)
 if c.get('missing_callback_sources'):
  m.w(actor+4,0xdeadf000);m.w(0xb39ab0,0xdeadf000)
 before=c['before'];out=copy.deepcopy(before);parts={};fields=[]
 offsets={'body':(0,[0x10,0x14,0x18]),'supply':(11,[0x10,0x14]),'detection':(22,[0x14]),'element':(8,[0x20]),'organization':(7,[0x88,0xac,0xb0,0xb8,0x5c,0x60]),'defender':(20,[0x2c,0x30])}
 for key,(slot,word_offsets) in offsets.items():
  if slot not in c['present']:
   assert before[key] is None,(key,before[key]);continue
  q=m.alloc(0x400);parts[key]=q;m.w(actor+0x60+slot*4,q);m.w(q+4,actor)
  values=before[key] if isinstance(before[key],list)else[before[key]]
  assert len(values)==len(word_offsets)
  for off,value in zip(word_offsets,values):
   # Poison unknown native fields and assert they stay untouched/unread.
   if value is None:m.w(q+off,0xdeadfeef)
   elif (key,off)in [('supply',0x14),('organization',0xb0),('defender',0x30)]:m.wb(q+off,value)
   else:m.w(q+off,value)
   fields.append((key,q,off,value))
 ability_ptrs={}
 for slot in [5,20]:
  if slot not in c['present']:continue
  entries=[v for v in before['abilities']if v['slot']==slot]
  q=parts['defender']if slot==20 else m.alloc(0x100)
  m.w(actor+0x60+slot*4,q);m.w(q+4,actor)
  abilities=[]
  for entry in entries:
   captured=[v for v in c['source']['abilities']if v['slot']==slot and v['handle']==entry['handle']]
   assert len(captured)==1;definition=captured[0]
   ability=m.alloc(0x80);shared=m.alloc(0x100);m.w(ability+4,shared);m.w(ability+0x20,entry['power'])
   m.w(shared+0x3c,m.alloc(4)if definition['projectile']else 0);m.w(shared+0x64,definition['power'])
   ability_ptrs[entry['handle']]=ability;abilities.append(ability)
  at=0x14 if slot==5 else 0x10;m.w(q+at,m.array(abilities));m.w(q+at+4,len(abilities))
 for slot in c['present']:
  if slot in [0,5,7,8,11,20,22]:continue
  # Unused present components retain actual non-null identity. Their payloads
  # are not fabricated; any unexpected access is a refusal.
  m.w(actor+0x60+slot*4,0xdead0000+slot*0x1000)
 source=c['source'];sources={0x284:('body',0),0x288:('body',1),0x420:('supply',None),0x1cc:('detection',None),0x2cc:('resupply',None),0x308:('morale_breakpoint',None),0x2fc:('org_control',None),0x218:('defender_control',None),0x174:('mask',None)}
 unknown=[]
 for off,(key,index)in sources.items():
  value=source[key]
  if value is not None and index is not None:value=value[index]
  if value is None:unknown.append(data+off);value=0xdeadfeef
  m.w(data+off,value)
 # Null/stale GTID has a real generation-table lookup before source arithmetic.
 linked=c.get('linked');m.w(actor+0x168,0xdeadfeef if linked is None else linked)
 registry=m.alloc(0x60004);m.w(0xb364c0,registry)
 if linked not in [None,0]:
  raise AssertionError('non-null linked Body requires captured registry payload')
 for q,off,value in [(q,off,value)for key,q,off,value in fields]:
  if value is None:unknown.append(q+off)
 if linked is None:unknown.append(actor+0x168)
 from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
 def read(u,access,address,size,value,user):
  assert not any(address<z+4 and address+size>z for z in unknown),('unbound native field read',hex(address),c['kind'],c['index'])
 m.u.hook_add(UC_HOOK_MEM_READ,read)
 written=set()
 def write(u,access,address,size,value,user):
  for key,q,off,previous in fields:
   if address<q+off+4 and address+size>q+off:written.add((key,off))
 m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 boundary=None
 for address,label in [(0x576c70,'mover'),(0x578300,'morale')]:
  def stop(mm,label=label):raise v1.Boundary(label)
  m.observe(address,stop)
 try:m.call(0x4fe590 if c['kind']=='Add'else 0x4fe260,descriptor,[actor,modifier])
 except v1.Boundary as e:boundary=str(e)
 for key,(slot,word_offsets)in offsets.items():
  if key not in parts:continue
  q=parts[key];values=[]
  for off in word_offsets:
   previous=next(v for k,p,o,v in fields if k==key and o==off)
   if previous is None and (key,off)not in written:values.append(None)
   elif (key,off)in [('supply',0x14),('organization',0xb0),('defender',0x30)]:values.append(m.rb(q+off))
   else:values.append(m.r(q+off))
  out[key]=values if isinstance(before[key],list)else values[0]
 for entry in out['abilities']:
  entry['power']=m.r(ability_ptrs[entry['handle']]+0x20)
 return dict(state=out,boundary=boundary),m.visited

def main():
 capture=json.loads((HERE/'actual-inputs.json').read_text());rows=[];provided_rows=[];pins={}
 for row in capture['provided_multiply']:
  for pc in nm.PRECISIONS:
   a,pa=provided(row,'original',pc);b,pb=provided(row,'v14',pc)
   assert a==b and a['boundary']is None,(a,b)
   assert a['state']==row['output'],('provided',pc,a['state'],row['output'])
   provided_rows.append(dict(precision=pc,input=row['input'],output=a));pins.update(pa);pins.update(pb)
 for row in capture['rows']:
  a,pa=run(row['input'],'original',row['precision']);b,pb=run(row['input'],'v14',row['precision'])
  assert a==b,(a,b)
  assert a['state']==row['output']and a['boundary']==row.get('boundary'),(row['input']['index'],row['precision'],a,row['output'])
  rows.append(dict(precision=row['precision'],input=row['input'],output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,provided=provided_rows,rows=rows,native_executions=2*(len(rows)+len(provided_rows)),instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())])
 path=HERE/'native-numeric.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),provided=len(provided_rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
