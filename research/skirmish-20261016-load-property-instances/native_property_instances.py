"""Original/v14 property constructor and attachment on captured real held-save inputs.
No original instructions are patched. Heap allocation, numeric actor consequences,
and explicitly unavailable clock/clamp/Performer callbacks are named boundaries.
Definitions and prior allocations are captured from the same runtime source Bank.
"""
from pathlib import Path
import sys,json,hashlib,copy,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_EAX
class Boundary(Exception):pass

def run(row,image,pc):
 c=row['input'];m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 events=[];stop=None;allocated=[]
 def malloc(mm):
  size=mm.arg(1);p=mm.alloc(size);mm.u.mem_write(p,b'\xcd'*size)
  if size==0x18:allocated.append(p)
  mm.ret(p)
 m.seam(0x6f7810,malloc);m.seam(0x6f7430,lambda mm:mm.ret())
 actor=m.alloc(0x400);owner=c['owner'];m.w(actor+0x14,owner)
 # Actual classes3 and4 share native4D1E80 for virtual108.
 assert struct.unpack('<I',nm.original.range(0xa299d8+0x108,4))[0]==0x4d1e80
 vt=m.alloc(0x150);m.w(vt+0x108,0x4d1e80);m.w(actor,vt)
 allegiance=c['allegiance'];allegiance_ptr=0
 if allegiance:
  allegiance_ptr=actor if allegiance==owner else m.alloc(0x100);m.w(allegiance_ptr+0x14,allegiance)
 m.w(actor+0xe8,allegiance_ptr)
 for slot in c['components']:
  m.w(actor+0x60+slot*4,m.alloc(0x200))
 p=m.alloc(0xe0);m.w(actor+0x78,p)
 if c['organization'] is not None:
  org=m.r(actor+0x7c);members=[];m.w(org+4,actor)
  for member in c['organization']:
   if not member['id']:members.append(0);continue
   q=m.alloc(0x100);m.w(q+0x14,member['id']);m.w(q+0x78,m.alloc(0xe0)if member['properties']else 0);m.w(q+0xa4,m.alloc(0x40)if member['experience']else 0);members.append(q)
  m.w(org+0x28,m.array(members));m.w(org+0x2c,len(members))
 before=c['before'];props=before['properties'];m.w(p+4,actor)
 def vector(at,values):
  arr=m.array(values+[0]*8);m.w(at,arr);m.w(at+4,len(values));m.w(at+8,len(values)+8)
 defs={int(key):m.alloc(0x100) for key in c['definitions']};ptrdefs={v:k for k,v in defs.items()}
 dtype={}
 def dtypeptr(key):
  if key not in dtype:dtype[key]=m.alloc(0x20)
  return dtype[key]
 def numeric_stop(mm):
  events.append('numeric');raise Boundary('numeric')
 for key,d in c['definitions'].items():
  q=defs[int(key)];m.w(q+0x24,d['flags']);m.wb(q+0x42,int(c['pass_members'][key]));nodes=[]
  for mod in d['modifiers']:
   z=m.alloc(0x30);nodes.append(z)
   if 'Numeric'in mod:
    v=mod['Numeric'];original=0xa2d79c if v['kind']=='Add'else 0xa2d690;vt=m.alloc(0x34);m.u.mem_write(vt,nm.original.range(original,0x34));m.w(vt+0x30,m.stub(numeric_stop));m.w(z,vt);m.w(z+8,1 if v['kind']=='Add'else 2);m.w(z+12,v['amount']);m.w(z+16,v['index']);m.w(z+20,int(v['repair']))
   elif'Resistance'in mod:
    v=mod['Resistance'];m.w(z,0xa2d708);m.w(z+8,5);m.w(z+12,v['amount']);m.w(z+16,dtypeptr(v['damage_type']))
   elif'Flag'in mod:
    m.w(z,0xa2d66c);m.w(z+8,0);m.w(z+12,mod['Flag']['index'])
   else:
    # These descriptors are NEVER used by the captured active effects or the
    # requested definition. Guard their access instead of fabricating payloads.
    m.w(z,0xdead0000);m.w(z+8,0xfeed)
  links=[m.alloc(8)for _ in nodes]
  for i,z in enumerate(nodes):m.w(links[i],z);m.w(links[i]+4,links[i+1]if i+1<len(nodes)else 0)
  m.w(q+0x1c,links[0]if links else 0)
 instances={}
 for key,v in before['instances'].items():
  key=int(key);ptr=m.alloc(0x18);instances[key]=ptr;m.w(ptr,0xa29678);m.w(ptr+4,v['created']);m.w(ptr+8,v['kind']);m.w(ptr+12,defs[v['definition']]);m.wb(ptr+16,int(v['disabled']));m.w(ptr+20,v['source_id'])
 vector(p+0x10,[instances[x]for x in props['effects']]);mod=props['modifier'];m.w(p+0x1c,mod['flags']);m.w(p+0xd8,props['modifier_flags'])
 for i,bits in enumerate(mod['scalars']):m.w(p+0x20+4*i,bits)
 factors=[];reverse_dtype={v:k for k,v in dtype.items()}
 for d in mod['damage_factors']:
  q=m.alloc(12);m.w(q,dtypeptr(d['damage_type']));m.w(q+4,d['factor']);m.w(q+8,d['resistance']);factors.append(q)
 vector(p+0x1c+0x74,factors);reverse_dtype={v:k for k,v in dtype.items()}
 world=m.alloc(0x500);m.w(0xb36534,world);m.w(world+0xe0,c['clock'])
 if c['phase']=='missing_clock':
  def clock(mm):raise Boundary('missing_clock')
  m.observe(0x50312f,clock)
 def clamp(mm):events.append('clamp');raise Boundary('missing_clamp')
 # All captured clamps are deliberately unavailable. Preserve all earlier writes.
 m.observe(0x50252f,clamp)
 # Multiply general/special clamp sites are added from disassembly below.
 for address in [0x57f4c0,0x584f80]:
  def unavailable(mm,address=address):
   name={0x57f4c0:'performer',0x5787f0:'organization',0x584f80:'settlement'}[address];events.append(name);raise Boundary(name)
  m.seam(address,unavailable)
 def forbidden_apply(mm):raise Boundary('positive_organization_apply')
 m.seam(0x4cef10,forbidden_apply)
 m.observe(0x503050,lambda mm:events.append('property_fx_empty'))
 # Empty42A590 uses poses but429080 exits on actual listheadNULL before OS/time.
 try:m.call(0x5817a0,p,[defs[c['property']],c['kind'],actor if c['source_id']else 0])
 except Boundary as e:stop=str(e)
 assert len(allocated)==1,(c['phase'],len(allocated))
 newptr=allocated[0];newid=max([int(k)for k in before['instances']]+[v['handle']for v in before['constructors']]+[0])+1
 after=copy.deepcopy(before)
 if stop=='missing_clock':
  assert m.r(newptr+4)==0xcdcdcdcd
  after['constructors'].append(dict(handle=newid,definition=ptrdefs[m.r(newptr+12)],kind=m.r(newptr+8),source_id=m.r(newptr+20)))
 else:
  instances[newid]=newptr
  after['instances']={str(key):dict(handle=key,definition=ptrdefs[m.r(q+12)],kind=m.r(q+8),created=m.r(q+4),source_id=m.r(q+20),disabled=bool(m.rb(q+16)))for key,q in instances.items()}
  revinst={v:k for k,v in instances.items()};after['properties']['effects']=[revinst[m.r(m.r(p+0x10)+4*i)]for i in range(m.r(p+0x14))]
 after['properties']['modifier']['scalars']=[m.r(p+0x20+4*i)for i in range(25)]
 after['properties']['modifier']['flags']=m.r(p+0x1c);after['properties']['modifier_flags']=m.r(p+0xd8)
 after['properties']['modifier']['damage_factors']=[]
 for i in range(m.r(p+0x1c+0x78)):
  q=m.r(m.r(p+0x1c+0x74)+4*i);after['properties']['modifier']['damage_factors'].append(dict(damage_type=reverse_dtype[m.r(q)],factor=m.r(q+4),resistance=m.r(q+8)))
 return dict(state=after,boundary=stop,events=events),m.visited

def main():
 inputs=json.loads((HERE/'actual-inputs-02.json').read_text());rows=[];extra=[];pins={}
 for row in inputs['rows']:
  if row.get('operation')=='applicable':continue
  a,pa=run(row,'original',row['precision']);b,pb=run(row,'v14',row['precision']);assert a==b
  assert a['state']==row['output'],(row['input']['phase'],row['precision'],a['state'],row['output'])
  phase=row['input']['phase'];assert a['boundary']==(None if phase in ['success','duplicate']else phase),(phase,a['boundary'])
  rows.append(dict(precision=row['precision'],input=row['input'],output=a));pins.update(pa);pins.update(pb)
  if row['input']['organization'] is not None:
   for pc in nm.PRECISIONS:
    if pc==row['precision']:continue
    x,px=run(row,'original',pc);y,py=run(row,'v14',pc);assert x==y==a
    extra.append(dict(precision=pc,input=row['input'],output=x));pins.update(px);pins.update(py)
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,rows=rows,organization_other_pc=extra,native_executions=(len(rows)+len(extra))*2,instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())])
 path=HERE/'native-property-instances.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
