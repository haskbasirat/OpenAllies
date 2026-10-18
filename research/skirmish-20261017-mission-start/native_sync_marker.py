"""Actual77DC50 scope marker hash/entry/guard and retention boundary.
Vector append/retire are explicit storage transport seams (existing independent
native-sync proof owns their internal allocation). GetCurrentThreadId is an
IAT/data seam; no executable instruction is patched. Both original/v14 execute.
"""
from pathlib import Path
import hashlib,json,itertools,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P

def run(c,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);seams=set();s=m.alloc(0x40);records={}
 def seam(a,f):m.seams[a]=f;seams.add(a)
 m.w8(s,c['enabled']);m.w32(s+4,c['suspend']&0xffffffff,c['hash']&0xffffffff,c['max']);m.w32(s+0x10,0,0)
 app=m.alloc(0x80);m.w32(0xb36528,0 if c['null_app']else app);m.w8(app+0x5c,c['flag5c']);m.w8(app+0x5d,c['flag5d'])
 tid=m.stub_next;m.stub_next+=16;m.w32(0x9f01e4,tid);m.w32(0xb39828,123)
 seam(tid,lambda _:m.ret(value=123 if c['thread']else 999))
 def append(_):
  end=m.r32(s+0x14);p=m.alloc(0x14);records[end]=p;m.w32(s+0x14,end+1);m.ret(value=p)
 def retire(_):m.w32(s+0x10,m.arg());m.ret(4)
 seam(0x77d680,append);seam(0x77d7d0,retire)
 label=m.alloc(32);m.u.mem_write(label,'MergeKingdoms\0'.encode('utf-16le'))
 states=[]
 for _ in range(5):
  m.call(0x77dc50,s,[label,0]);begin,end=m.r32(s+0x10),m.r32(s+0x14)
  def signed(v):return v if v<0x80000000 else v-0x100000000
  entries=[]
  for i in range(begin,end):
   p=records[i];assert m.r32(p)==label
   entries.append({'kind':m.r32(p+4),'value':m.r32(p+8),'hash':signed(m.r32(p+16))})
  states.append({'hash':signed(m.r32(s+8)),'begin':begin,'end':end,'entries':entries})
 return states,seams

def main():
 cases=[];seams=set()
 for pc in h.PRECISION:
  for enabled,suspend,thread,null_app,flags,maximum,value in [(1,0,1,0,(0,0),n,v)for n,v in itertools.product((1,2,5),(-1,0,65535,0x7fffffff,-0x80000000))]+[(0,0,1,0,(0,0),2,-1),(1,1,1,0,(0,0),2,-1),(1,-1,0,0,(0,0),2,-1),(1,0,1,0,(1,0),2,-1),(1,0,1,0,(0,128),2,-1),(1,-1,1,1,(255,255),2,-1)]:
   c={'enabled':enabled,'suspend':suspend,'thread':thread,'null_app':null_app,'flag5c':flags[0],'flag5d':flags[1],'max':maximum,'hash':value}
   a,s=run(c,'original',pc);b,t=run(c,'v14',pc);assert a==b;seams|=s|t
   cases.append({'precision':pc,'input':c,'output':a})
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'cases':cases,
  'seams_not_executed':[hex(a)for a in sorted(seams)if a<0x1000000],
  'executed_instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams]}
 p=HERE/'native-sync-marker.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(cases),'machines':len(cases)*2,'pins':len(doc['executed_instructions']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
