"""416370 AudioManager lookup: actual empty/translation/data traversal.
Only CString pool/copy, hashes and CRT case comparison are transport seams;
4154D0/4915F0 collision traversal and all416370 branches execute unchanged.
47D880 manager-base constructor executes with only heap allocation seamed.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
ORIGINAL=nm.original

def machine(image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(ORIGINAL.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(ORIGINAL.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 return m

def run(c,image,pc):
 m=machine(image,pc);events=[];strings={};values={}
 def string(s):
  if s not in strings:
   p=m.alloc(16+2*(len(s)+1));m.w(p+4,len(s));m.w(p+12,1000000);m.u.mem_write(p+16,(s+'\0').encode('utf-16-le'));strings[s]=p+16;values[p+16]=s
  return strings[s]
 def text(p):return values[p]
 manager=m.alloc(0x4c);m.u.mem_write(manager,b'\xcd'*0x4c)
 m.seam(0x6f7810,lambda mm:mm.ret(m.alloc(mm.arg(1))))
 m.call(0x47d880,manager,[0])
 assert m.r(manager+8)==0 and m.r(manager+0x14)==0
 # Every explicit test-map key shares one bucket, so actual native collision
 # and case-insensitive traversal execute for both maps.
 translations=m.alloc(4);m.w(manager+0xc,translations);m.w(manager+0x10,1);m.w(manager+0x14,len(c['translations']))
 head=0
 for old,new in reversed(c['translations']):head=m.array([string(old),string(new),head])
 m.w(translations,head)
 data=m.alloc(4);m.w(manager+0x30,data);m.w(manager+0x34,1);m.w(manager+0x38,len(c['data']))
 head=0;identities={}
 for ids in reversed(c['data']):
  obj=m.alloc(0x20);m.w(obj+8,string(ids));identities[obj]=ids;head=m.array([obj,head])
 m.w(data,head)
 pool=m.alloc(4);vt=m.alloc(16);m.w(pool,vt)
 m.w(vt+12,m.stub(lambda mm:mm.ret(string('')-16)))
 m.seam(0x856370,lambda mm:mm.ret(pool))
 m.seam(0x405b80,lambda mm:(m.w(mm.ecx(),m.r(mm.arg(1))),mm.ret(mm.ecx(),4)))
 m.seam(0x5043f0,lambda mm:(events.append(['translation',text(mm.arg(1))]),mm.ret(0,4)))
 m.seam(0x405030,lambda mm:(events.append(['data',text(m.r(mm.arg(1)))]),mm.ret(0)))
 m.seam(0x96cad6,lambda mm:mm.ret(0 if text(mm.arg(1)).lower()==text(mm.arg(2)).lower() else 1))
 result=m.call(0x416370,manager,[string(c['query'])]);assert not m.null_accesses
 return dict(id=identities.get(result),events=events,constructor_mode=m.r(manager+8)),m.visited

def cases():
 out=[]
 for translations in [[],[['old','new']],[['noise','new'],['old','new']],[['old','missing']],[['old','new'],['new','last']]]:
  for data in [[],['old'],['new'],['noise','old','new','last']]:
   for query in ['','old','OLD','new','missing']:
    out.append(dict(query=query,translations=translations,data=data))
 return out

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(pc,c)
   pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 # Static original bytes proving GameData's actual+584 constructor call.
 for va,size in [(0x48ad48,0x30),(0x482090,0x2b)]:
  assert ORIGINAL.range(va,size)==nm.modified.range(va,size)
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(v),bytes=b.hex())for v,b in sorted(pins.items())],
  constructor_binding=[dict(va=hex(v),bytes=ORIGINAL.range(v,n).hex())for v,n in [(0x48ad48,0x30),(0x482090,0x2b)]])
 path=HERE/'native-audio-lookup.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
