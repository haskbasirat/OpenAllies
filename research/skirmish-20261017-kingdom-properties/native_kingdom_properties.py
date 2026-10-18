"""Unchanged4E3AD0/5036D0/4CEF10/5035E0/5817A0/581630 on actual field captures.

Declared leaves: heap allocation/free, actual immutable restriction profile/key
lookup transport (4F0400/504530). The latter consumes the exact loaded profile
projection and current Team/Kingdom names. Positive actor health and resistance
effects execute original code; no successful property/effect callbacks are seams.
Unavailable clock or Data-name/GameEvent owners stop at their reached PCs.
"""
from pathlib import Path
import sys,json,hashlib,copy,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
class Boundary(Exception):pass
def bind_image(m,image):
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
def run(row,image,pc):
 c=row['input'];before=c['before'];m=nm.Machine(pc);bind_image(m,image)
 events=[];allocated=[];boundary=None
 def malloc(mm):
  size=mm.arg(1);q=mm.alloc(size);mm.u.mem_write(q,b'\xcd'*size)
  if size==0x18:allocated.append(q)
  mm.ret(q)
 m.seam(0x6f7810,malloc);m.seam(0x6f7430,lambda mm:mm.ret())
 actor=m.alloc(0x400);data=m.alloc(0x600);kingdom=m.alloc(0x400);p=m.alloc(0xe0);body=m.alloc(0x100)
 assert c['allegiance']==c['owner'] and c['linked']==0
 m.w(actor+4,data);m.w(actor+0x14,c['actor']);m.w(kingdom+0x14,c['owner']);m.w(actor+0xe8,kingdom)
 for i,bits in enumerate(c['position']):m.w(actor+0x20+4*i,bits)
 vt=m.alloc(0x150);m.w(vt+0x108,0x4d1e80);m.w(actor,vt);m.w(kingdom,vt)
 for slot in c['components']:m.w(actor+0x60+4*slot,0xdead0000+slot*0x1000)
 assert 0 in c['components']and 6 in c['components']and not any(i in c['components']for i in [7,14,5])
 m.w(actor+0x60,body);m.w(body+4,actor);m.w(actor+0x78,p);m.w(p+4,actor)
 m.w(data+0x284,c['base_health']);m.w(data+0x288,c['base_defense']);m.w(actor+0x168,c['linked'])
 for i,bits in enumerate(before['body']):m.w(body+0x10+4*i,bits)
 cache=m.alloc(0x200);m.w(0xb39ab0,cache)
 for offset,key in [(0x1a4,'clamp_add'),(0x1a8,'clamp_multiply')]:
  cell=m.alloc(4);m.w(cell,c[key]);m.w(cache+offset,cell)
 world=m.alloc(0x500);m.w(0xb36534,world);m.w(world+0xe0,c['clock'])
 gt=m.alloc(0x60004);m.w(0xb364c0,gt)
 for raw,q in [(c['actor'],actor),(c['owner'],kingdom)]:
  slot=raw&0xffff;m.u.mem_write(gt+4+2*slot,struct.pack('<H',raw>>16));m.w(gt+0x20004+4*slot,q)
 defs={int(k):m.alloc(0x100)for k in c['definitions']};reverse_defs={q:k for k,q in defs.items()}
 types={}
 def dtype(key):
  if key not in types:types[key]=m.alloc(4)
  return types[key]
 def vector(at,values,capacity=None):
  capacity=max(len(values),8)if capacity is None else capacity
  assert capacity>=len(values)
  q=m.array(values+[0]*(capacity-len(values)));m.w(at,q);m.w(at+4,len(values));m.w(at+8,capacity)
 def links(values):
  nodes=[m.alloc(8)for _ in values]
  for i,value in enumerate(values):m.w(nodes[i],value);m.w(nodes[i]+4,nodes[i+1]if i+1<len(nodes)else 0)
  return nodes[0]if nodes else 0
 def cstring(at,units):
  q=m.alloc(2*(len(units)+1));m.u.mem_write(q,struct.pack('<'+'H'*(len(units)+1),*units,0));m.w(at,q)
 def text(at):
  q=m.r(at);out=[]
  while True:
   u=struct.unpack('<H',m.u.mem_read(q,2))[0]
   if not u:break
   out.append(chr(u));q+=2
  return ''.join(out)
 #Exact shared PropertyData constructor/vtable receipt. The true virtual type
 #test is independent of loaded is_technology byte+5C.
 assert nm.original.range(0x47d934,6)==nm.modified.range(0x47d934,6)==bytes.fromhex('c70634c8a000')
 assert struct.unpack('<I',nm.original.range(0xa0c834+0x18,4))[0]==0x6a71d0
 for key,d in c['definitions'].items():
  key=int(key);q=defs[key];fact=c['facts'][str(key)];m.w(q,0xa0c834);m.w(q+0x24,d['flags']);m.wb(q+0x5c,int(fact['technology']));m.wb(q+0x42,int(fact['pass_members']))
  cstring(q+8,list(map(ord,fact['id'])));m.w(q+0x10,0xdeadf000)
  m.w(q+0x38,links([defs[value]for value in fact['required']]))
  modifiers=[]
  for v in d['modifiers']:
   z=m.alloc(0x30);modifiers.append(z)
   if 'Numeric'in v:
    v=v['Numeric'];m.w(z,0xa2d79c if v['kind']=='Add'else 0xa2d690);m.w(z+8,1 if v['kind']=='Add'else 2);m.w(z+12,v['amount']);m.w(z+16,v['index']);m.w(z+20,int(v['repair']))
   elif'Resistance'in v:
    v=v['Resistance'];m.w(z,0xa2d708);m.w(z+8,5);m.w(z+12,v['amount']);m.w(z+16,dtype(v['damage_type']))
   elif'Flag'in v:m.w(z,0xa2d66c);m.w(z+8,0);m.w(z+12,v['Flag']['index'])
   else:m.w(z,0xdead0000);m.w(z+8,0xfeed)
  m.w(q+0x1c,links(modifiers))
 instances={}
 for handle,v in before['instances'].items():
  handle=int(handle);q=m.alloc(0x18);instances[handle]=q;m.w(q,0xa29678);m.w(q+4,v['created']);m.w(q+8,v['kind']);m.w(q+12,defs[v['definition']]);m.wb(q+16,int(v['disabled']));m.w(q+20,v['source_id'])
 vector(kingdom+0x318,[instances[v]for v in before['kingdom_records']],before['capacity'])
 assert before['actors']==[c['actor']]
 head=links([actor]);m.w(kingdom+0x284,head);m.w(kingdom+0x288,head)
 props=before['properties'];vector(p+0x10,[instances[v]for v in props['effects']]);m.w(p+0x1c,props['modifier']['flags']);m.w(p+0xd8,props['modifier_flags'])
 for i,bits in enumerate(props['modifier']['scalars']):m.w(p+0x20+i*4,bits)
 factors=[]
 for v in props['modifier']['damage_factors']:
  q=m.alloc(12);m.w(q,dtype(v['damage_type']));m.w(q+4,v['factor']);m.w(q+8,v['resistance']);factors.append(q)
 vector(p+0x90,factors)
 profiles=c['profile_lookup'];profile_ptrs={i:m.alloc(0x80)for i,_ in enumerate(profiles['profiles'])};reverse_profiles={v:i for i,v in profile_ptrs.items()}
 if profiles['team']is not None:
  team=m.alloc(0x90);m.w(kingdom+0x1ec,team);cstring(team+0x64,profiles['team'])
 cstring(kingdom+0x348,profiles['kingdom'])
 def lookup(mm):
  name=text(mm.arg(1));found=next((i for i in reversed(range(len(profiles['profiles'])))if profiles['profiles'][i]['name'].lower()==name.lower()),None)
  events.append(dict(profile=name,found=found));mm.ret(0 if found is None else profile_ptrs[found],pop=4)
 def restricted(mm):
  name=text(mm.arg(1));profile=profiles['profiles'][reverse_profiles[mm.ecx()]]
  found=next((value for key,value in reversed(profile['technologies'])if key.lower()==name.lower()),False)
  events.append(dict(restricted=name,value=found));mm.ret(int(found),pop=4)
 m.seam(0x4f0400,lookup);m.seam(0x504530,restricted)
 #Do not let zero-filled omitted Actor/Data fields create successful guards.
 #All admitted reads must belong to the captured/explicitly bound fields.
 from unicorn import UC_HOOK_MEM_READ
 domains=[(actor,0x400,[(0,8),(0x14,4),(0x20,12),(0x60,31*4),(0xe8,4),(0x168,4)]),
          (data,0x600,[(0x284,8)]),
          (kingdom,0x400,[(0,4),(0x14,4),(0x1ec,4),(0x284,8),(0x318,12),(0x348,4)])]
 def guarded(u,access,address,size,value,user):
  for base,length,fields in domains:
   if base<=address<base+length:
    assert any(base+off<=address and address+size<=base+off+n for off,n in fields),('unbound field read',hex(address-base),hex(u.reg_read(nm.UC_X86_REG_EIP)))
 m.u.hook_add(UC_HOOK_MEM_READ,guarded)
 if c['phase']=='missing_clock':
  def clock(mm):raise Boundary('missing_clock')
  m.observe(0x50312f,clock)
 def notification(mm):raise Boundary('notification')
 m.observe(0x4e3bf5,notification)
 def unbound(mm):raise AssertionError('unexpected unbound positive component/diagnostic reached')
 for address in [0x5787f0,0x584f80,0x57f4c0,0x404f60]:m.seam(address,unbound)
 try:m.call(0x4e3ad0,kingdom,[defs[c['definition']],c['kind'],actor if c['origin']else 0,int(c['notify'])])
 except Boundary as e:boundary=str(e)
 after=copy.deepcopy(before);next_id=max([int(k)for k in before['instances']]+[int(k)for k in before['constructors']]+[0])+1
 for i,q in enumerate(allocated):
  handle=next_id+i
  if boundary=='missing_clock' and i+1==len(allocated):
   assert m.r(q+4)==0xcdcdcdcd
   after['constructors'][str(handle)]=dict(definition=reverse_defs[m.r(q+12)],kind=m.r(q+8),source_id=m.r(q+20))
  else:instances[handle]=q
 after['instances']={str(handle):dict(handle=handle,definition=reverse_defs[m.r(q+12)],kind=m.r(q+8),created=m.r(q+4),source_id=m.r(q+20),disabled=bool(m.rb(q+16)))for handle,q in instances.items()}
 revinst={v:k for k,v in instances.items()};after['kingdom_records']=[revinst[m.r(m.r(kingdom+0x318)+4*i)]for i in range(m.r(kingdom+0x31c))];after['capacity']=m.r(kingdom+0x320)
 after['properties']['effects']=[revinst[m.r(m.r(p+0x10)+4*i)]for i in range(m.r(p+0x14))]
 after['properties']['modifier']['scalars']=[m.r(p+0x20+4*i)for i in range(25)];after['properties']['modifier']['flags']=m.r(p+0x1c);after['properties']['modifier_flags']=m.r(p+0xd8)
 reverse_types={v:k for k,v in types.items()};after['properties']['modifier']['damage_factors']=[]
 for i in range(m.r(p+0x94)):
  q=m.r(m.r(p+0x90)+4*i);after['properties']['modifier']['damage_factors'].append(dict(damage_type=reverse_types[m.r(q)],factor=m.r(q+4),resistance=m.r(q+8)))
 after['body']=[m.r(body+0x10+4*i)for i in range(3)]
 return dict(state=after,boundary=boundary,events=events),m.visited
def main():
 capture=json.loads((HERE/'actual-inputs.json').read_text());rows=[];pins={}
 for row in capture['rows']:
  a,pa=run(row,'original',row['precision']);b,pb=run(row,'v14',row['precision']);assert a==b
  assert a['state']==row['output']and a['boundary']==row['boundary'],(row['input']['phase'],row['input']['source_actor_definition'],a,row['output'],row['boundary'])
  rows.append(dict(precision=row['precision'],input=row['input'],output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 static=[]
 for va,size in [(0x47d934,10),(0xa0c834,0x1c),(0x48b6a2,0x24),(0x4c48c0,0x20)]:
  raw=nm.original.range(va,size);assert raw==nm.modified.range(va,size)
  static.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())],static_shared_property_type_receipts=static)
 path=HERE/'native-kingdom-properties.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
