"""Full4E5DF0 on captured actual tutorial02 Kingdom/source-owner inputs.

Actor4D7230 and restriction4E1810 answers are explicitly captured service seams;
the separate Actor/restriction bindings prove those services. Source fields are
the actual completed catalog projection, not an emulated native TGI load here.
Native category/list traversal, all flow writes, plain-item construction/hash
insertion,4F8AF0 world registration,4E5250 relations and both masks execute
unchanged. Successful allocation/free and the diagnostic sink are host seams.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
ORIGINAL=nm.original
RESOURCE_WITNESS=ROOT/'research/phase7-properties/resource-domain-evidence.json'
RESOURCE_COUNT=json.loads(RESOURCE_WITNESS.read_text())['resolved_catalog_evidence']['count']

def run(c,image):
 pc=c['live_precision'];nm.original=ORIGINAL if image=='original' else nm.modified;m=nm.Machine(pc)
 k=m.alloc(0x400);world=m.alloc(0x300);data=m.alloc(0x400);sai=m.alloc(0x40)
 m.w(0xb36534,world);m.w(0xb36530,data);m.w(0xb36528,sai);m.w(k,0xa299d8)
 m.seam(0x6f7810,lambda mm:mm.ret(mm.alloc(mm.arg(1))))
 m.seam(0x6f7430,lambda mm:mm.ret())
 m.call(0x4e1cf0,k+0x1fc,[k])
 for offset,value in c['initial']['derived_words'].items():
  if int(offset) not in (0x1fc,0x200,0x204,0x208,0x20c,0x210,0x214,0x218,0x21c,0x220,0x224):m.w(k+int(offset),value)
 for offset,v in c['initial']['flows'].items():
  offset=int(offset)
  if offset==0x214:continue # real recruit constructor owns its singles vector.
  values=v['values'];p=m.array(values+[0xa5a5a5a5]*max(0,v['capacity']-len(values)))
  m.w(k+offset,p);m.w(k+offset+4,len(values));m.w(k+offset+8,v['capacity'])
 m.w(k+0x1ec,0);assert c['initial']['team'] is None
 color=m.alloc(16);m.w(color+12,c['resolved_color_index']);m.w(k+0x1e8,color)
 for offset,key in ((0x148,'all'),(0x154,'majors'),(0x160,'color_counts')):
  values=c['registry_initial'][key];assert all(v in (None,0) for v in values)
  m.w(world+offset,m.array([0]*len(values)))
  if offset!=0x160:m.w(world+offset+4,len(values));m.w(world+offset+8,len(values))
 m.w(world+0x144,c['registry_initial']['count']);m.w(world+0x2c,c['world_version'])
 # This is the pinned source-resolved Resource registry count, not capacity8
 # or a count inferred from the reset's output. Saved flow lengths are checked
 # separately against it for this actual input's initially empty arrays.
 assert all(c['initial']['flows'][str(off)]['values']==[] for off in (0x1a0,0x1ac,0x1b8))
 resource_count=RESOURCE_COUNT;m.w(data+0x38,resource_count)
 actor_calls=[]
 m.seam(0x4d7230,lambda mm:(actor_calls.append(mm.arg(1)),mm.ret(1,pop=4))[-1])
 string=m.alloc(64)+16;m.w(string-4,2)
 m.u.mem_write(string,'kingdom_0\0'.encode('utf-16-le'))
 def name(mm):out=mm.arg(1);mm.w(out,string);mm.ret(out,pop=4)
 m.seam(0x4bedf0,name)
 diagnostics=[]
 m.seam(0x404f60,lambda mm:(diagnostics.append(dict(line=mm.arg(3),format=hex(mm.arg(5)))),mm.ret())[-1])
 manager=m.alloc(0x100);heads=m.array([0]*43);m.w(0xb39f58,manager);m.w(manager+0x5c,heads)
 defs={};nodes={};reverse={};forbidden={};allowed={};expected_recruits=[]
 for row in c['definitions']:
  p=m.alloc(0x510);token=row['token'];defs[token]=p;reverse[p]=token
  m.w(p+0x174,row['mask']);links=m.alloc(0x18);m.w(p+0x2c8,links)
  for i,(off,owner) in enumerate(((0,links),(12,links),(0x17c,p))):
   value=row['links'][i];address=owner+off
   if value is None:forbidden[address]=row['definition']+':'+str(i);m.w(address,0xdeadbeef)
   else:m.w(address,1 if value else 0)
  allowed[token]=row['allowed'];nodes[token]=m.array([p,0,0])
  if row['allowed'] is True:expected_recruits.append(token)
 for category in (19,42):
  group=[r for r in c['definitions'] if r['category']==category]
  m.w(heads+4*category,nodes[group[0]['token']] if group else 0)
  for a,b in zip(group,group[1:]):m.w(nodes[a['token']]+8,nodes[b['token']])
 forbidden_bytes={cell+i:label for cell,label in forbidden.items() for i in range(4)}
 def on_read(u,access,address,size,value,_):
  for byte in range(address,address+size):
   if byte in forbidden_bytes:raise AssertionError('unbound source field read '+forbidden_bytes[byte]+' at '+hex(u.reg_read(UC_X86_REG_EIP)))
 m.u.hook_add(UC_HOOK_MEM_READ,on_read)
 restriction_calls=[]
 def restriction(mm):
  assert mm.ecx()==k;token=reverse[mm.arg(1)];answer=allowed[token];assert answer is not None
  restriction_calls.append(token);mm.ret(int(answer),pop=4)
 m.seam(0x4e1810,restriction)
 recruits=[]
 def recruit(mm):
  assert mm.ecx()==k+0x1fc;item=mm.arg(1);assert mm.r(item)==0xa29650 and mm.rb(item+8)==1
  recruits.append(reverse[mm.r(item+4)])
 m.observe(0x4e4c30,recruit)
 assert m.call(0x4e5df0,k,[0])&255==1
 assert actor_calls==[0] and recruits==expected_recruits
 assert c['initial']['recruit_items']==dict(changes=0,entries=[],singles=[])
 assert c['recruit_items']['entries']==[[token,False] for token in recruits]
 assert c['recruit_items']['singles']==[]
 def vector(offset):
  values=[m.r(m.r(world+offset)+4*i) for i in range(m.r(world+offset+4))]
  return [c['owner'] if v==k else None if v==0 else (_ for _ in ()).throw(AssertionError(hex(v))) for v in values]
 colors=[m.r(m.r(world+0x160)+4*i) for i in range(len(c['registry_initial']['color_counts']))]
 flows={str(off):[m.r(m.r(k+off)+4*i) for i in range(m.r(k+off+4))] for off in (0x1a0,0x1ac,0x1b8)}
 output=dict(returned=True,actor_calls=actor_calls,restriction_calls=restriction_calls,recruits=recruits,
  recruit_changes=m.r(k+0x224),recruit_count=m.r(k+0x20c),flows=flows,
  stock=[m.r(m.r(k+0x1c4)+4*i) for i in range(m.r(k+0x1c8))],
  registry_state=dict(count=m.r(world+0x144),all=vector(0x148),majors=vector(0x154),color_counts=colors),
  word228=m.r(k+0x228),word22c=m.r(k+0x22c),diagnostics=diagnostics)
 assert output['registry_state']==c['registry_state']
 assert flows=={str(off):c['flows'][str(off)]['values'] for off in (0x1a0,0x1ac,0x1b8)}
 assert output['stock']==c['flows']['452']['values']==c['initial']['flows']['452']['values']
 assert output['word228']==c['derived_words']['552']
 low=c['derived_bytes']['556']|(c['derived_bytes']['557']<<8)
 assert output['word22c']&65535==low
 assert output['recruit_changes']==c['recruit_items']['changes'] and output['recruit_count']==len(c['recruit_items']['entries'])
 return output,m.visited

def main():
 source=HERE/'actual-first-kingdom-final.json';actual=json.loads(source.read_text());rows=[];pins={}
 for c in actual['rows']:
  assert c['outcome']=={'Ok':True} and c['definitions']
  a,pa=run(c,'original');b,pb=run(c,'v14');assert a==b
  rows.append(dict(precision=c['live_precision'],output=a))
  for pp in (pa,pb):
   for va,raw in pp.items():
    assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw;pins[va]=raw
 doc=dict(schema=1,scope=__doc__,source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
  resource_count=RESOURCE_COUNT,resource_source_witness_sha256=hashlib.sha256(RESOURCE_WITNESS.read_bytes()).hexdigest(),
  separate_image_runs=6,cases=rows,instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())])
 path=HERE/'native-actual-kingdom.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=6,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
