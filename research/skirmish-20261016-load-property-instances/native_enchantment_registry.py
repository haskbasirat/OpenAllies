"""EnchantmentType native manager ctor/vtable/Find identity witness.
Constructor prefix runs real486F00 and47D880 until first unrelated owner-string
service856370; actual Find thunk48D4A0 executes unchanged48D500 on captured
registration-name namespace fixtures, without translations. No full source-load
completion or historical forward-reference claim is made by this prefix.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
class Cut(Exception):pass

def run(image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 mgr=m.alloc(0x70);m.u.mem_write(mgr,b'\xcd'*0x70)
 def readtext(p):
  words=[]
  while (v:=int.from_bytes(m.u.mem_read(p,2),'little')):words.append(chr(v));p+=2
  return ''.join(words)
 #Named CRT wchar case-insensitive comparison seam; ASCII source IDs only.
 m.seam(0x96cad6,lambda mm:mm.ret(0 if readtext(mm.arg(1)).casefold()==readtext(mm.arg(2)).casefold() else 1))
 m.seam(0x6f7810,lambda mm:mm.ret(mm.alloc(mm.arg(1))))
 def cutoff(mm):raise Cut()
 m.observe(0x486f58,cutoff)
 try:m.call(0x486f00,mgr,[7,0])
 except Cut:pass
 assert m.r(mgr)==0xa0d410 and m.r(mgr+8)==0
 vt=[m.r(m.r(mgr)+offset)for offset in [0,8,0x18,0x1c]]
 assert vt==[0x483a40,0x6a71d0,0x48d4a0,0x487060]
 #Actual mode0 registration names from the current shipped source file.
 names=['ench_type_generic','ench_type_radiation','ench_type_courage','ench_type_fear']
 def string(s):
  b=(s+'\0').encode('utf-16le');p=m.alloc(len(b));m.u.mem_write(p,b);return p
 bucket=m.alloc(4);m.w(mgr+0x30,bucket);m.w(mgr+0x34,1);m.w(mgr+0x38,4)
 ptrs={}
 for name in names:
  p=m.alloc(0x20);m.w(p+8,string(name));entry=m.alloc(8);m.w(entry,p);m.w(entry+4,m.r(bucket));m.w(bucket,entry);ptrs[p]=name
 rows=[]
 for query in names+[n.upper()for n in names]+['missing','']:
  got=m.call(vt[2],mgr,[string(query)]);result=ptrs.get(got)
  assert result==(query.lower()if query.lower()in names else None)
  rows.append(dict(query=query,result=result))
 return dict(mode=m.r(mgr+8),vtable=hex(m.r(mgr)),leaves=[hex(x)for x in vt],lookups=rows),m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  a,pa=run('original',pc);b,pb=run('v14',pc);assert a==b;rows.append(dict(precision=pc,output=a));pins.update(pa);pins.update(pb)
 static=[(0x48b3c9,10),(0x48d4a0,5),(0xa0d410,0x20)]
 for va,size in static:
  raw=nm.original.range(va,size);assert raw==nm.modified.range(va,size);pins[va]=raw
 for va,raw in pins.items():assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 doc=dict(scope=__doc__,rows=rows,native_executions=6,instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())])
 path=HERE/'native-enchantment-registry.json'
 if'--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=6,lookups=60,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
