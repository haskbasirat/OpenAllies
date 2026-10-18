"""Full7B1080 AIPlayer constructor, including97x2 command containers and ego getter.

Only operator-new is intercepted. The profile head-ego and manager table are
explicit source-layout fixtures; this is not a source profile-selection proof.
Unused capacity allocation bytes are not interpreted as initialized fields.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
H=lambda b:hashlib.sha256(b).hexdigest();O=h.P;V=h.V

def run(c,image):
 prior=h.P;h.P=O if image=='original'else V
 try:m=h.Machine(c['precision'])
 finally:h.P=prior
 h.VISITED.clear();allocation=[];sizes={}
 def new(m):
  n=m.arg();p=m.alloc(n);m.u.mem_write(p,bytes([c['poison']])*n);allocation.append(n);sizes[p]=n;m.ret(value=p)
 m.seams[0x6f7810]=new
 actor=m.alloc(0x19c);m.u.mem_write(actor,bytes([c['poison']])*0x19c)
 profile=m.alloc(0x100);head=m.alloc(4);kingdom=m.alloc(0x300);root=m.alloc(0x900)
 egos=m.alloc(0x40);items=m.alloc(16);ego=m.alloc(0x20)
 m.w32(0xb36530,root);m.w32(root+0x568,egos);m.w32(egos+0x24,items)
 m.w32(items+12,ego);m.w32(head,3);m.w32(profile+0x34,head)
 value=m.call(0x7b1080,actor,[c['slot'],kingdom,profile,c['word198']])
 assert value==actor and m.final_esp==h.STACK+20
 def words(off,n):return m.r32(actor+off,n)
 assert m.r32(actor+8)==kingdom and m.r32(actor+0x18)==profile and m.r32(actor+0x1c)==ego
 # Fixed-size vectors own initialized entries only up to native count.
 buckets=[]
 for off in [0x54,0x60]:
  p,count,cap=words(off,3);assert count==cap==17
  entries=[m.r32(m.r32(p+4*i),3)for i in range(count)];buckets.append(entries)
 hashes=[]
 for off in range(0x6c,0x180,4):
  ptr,cap,count=words(off,3);flag=m.r8(actor+off+12)
  if ptr not in sizes or cap not in [5,47,97] or sizes[ptr]!=cap*4 or count!=0 or flag!=1:continue
  assert all(x==0 for x in m.r32(ptr,cap));hashes.append(dict(offset=off,capacity=cap,count=count,flag=flag))
 commands=[]
 for off in [0x180,0x18c]:
  ptr,count,cap=words(off,3);assert count==0 and cap==97 and m.r32(ptr-4)==97
  commands.append([dict(data_vtable=m.r32(ptr+24*i),list_words=m.r32(m.r32(ptr+24*i+4),4),data_words=m.r32(ptr+24*i+8,4))for i in range(97)])
 output=dict(slot=m.r32(actor),ready=bool(m.r8(actor+4)),controller=m.r32(actor+0xc),
  words10_14=words(0x10,2),ego_index=3,words24_50=words(0x24,12),
  vectors={hex(off):words(off,n)for off,n in [(0x9c,2),(0x104,2),(0x12c,2),(0x144,3)]},word198=m.r32(actor+0x198),buckets=buckets,hashes=hashes,commands=commands)
 pins=dict(h.VISITED);leaf=pins.pop(0x6f7810)
 return output,pins,leaf,allocation

def main():
 rows=[];pins={};leaf=None
 for pc in h.PRECISION:
  for slot,poison,word in [(0,0,0),(7,0xcd,0xdeadbeef),(0xffffffff,0xff,1)]:
   c=dict(precision=pc,slot=slot,poison=poison,word198=word)
   a,ap,al,aa=run(c,'original');b,bp,bl,ba=run(c,'v14');assert a==b and aa==ba and al==bl
   pins.update(ap|bp);leaf=al;rows.append(dict(input=c,output=a,allocations=aa))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(O.b),v14_sha256=H(V.b),
  rows=rows,machines=2*len(rows),executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],
  intercepted_entries=[dict(va='0x6f7810',bytes=leaf)],dependency_sha256=H((ROOT/'research/phase8-movement/harness.py').read_bytes()))
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-ai-player-constructor.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),intercepted_entries=1,sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
