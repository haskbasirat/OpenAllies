"""Full79FC40 inactive AddSAIPlayer with explicit profile and full7B1080 child.

Heap allocation/free and diagnostic logger are named leaf seams. Existing
slots/worklists and duplicate Kingdoms are explicit fixtures. Profile choice
and ready=true activation are separate required services, not empty defaults.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
H=lambda b:hashlib.sha256(b).hexdigest();O=h.P;V=h.V
LEAVES={0x6f7810,0x6f7430,0x404f60}
def run(c,image):
 prior=h.P;h.P=O if image=='original'else V
 try:m=h.Machine(c['precision'])
 finally:h.P=prior
 h.VISITED.clear();alloc=[]
 def new(m):n=m.arg();p=m.alloc(n);alloc.append(n);m.ret(value=p)
 m.seams[0x6f7810]=new;m.seams[0x6f7430]=lambda m:m.ret();m.seams[0x404f60]=lambda m:m.ret()
 manager=m.alloc(0xb0);kingdom=m.alloc(0x300);profile=m.alloc(0x100);head=m.alloc(4)
 root=m.alloc(0x900);egos=m.alloc(0x40);items=m.alloc(4);ego=m.alloc(0x20)
 m.w32(0xb36528,manager);m.w32(0xb36530,root);m.w32(root+0x568,egos);m.w32(egos+0x24,items)
 m.w32(items,ego);m.w32(profile+0x34,head);m.w32(head,0);m.w32(kingdom+0x190,c['kingdom_category'])
 old=[];slots=m.alloc(max(1,len(c['slots']))*4)
 for i,present in enumerate(c['slots']):
  ptr=m.alloc(0x19c)if present else 0;old.append(ptr);m.w32(slots+4*i,ptr)
  if ptr:m.w32(ptr+8,kingdom if c['duplicate']else m.alloc(0x300))
 m.w32(manager+0x60,slots,len(old),len(old));m.w8(manager+0x20,0)
 for off,values in [(0x4c,c['list4c']),(0x54,c['list54'])]:
  first=last=0
  for value in reversed(values):
   node=m.alloc(12);m.w32(node,value,0,first);first=node;last=last or node
  m.w32(manager+off,first,last)
 result=m.call(0x79fc40,manager,[0 if c['null_kingdom']else kingdom,profile,c['association']])
 assert m.final_esp==h.STACK+16
 count=m.r32(manager+0x64);slots=m.r32(manager+0x60);values=[];new_player=None
 for i in range(count):
  p=m.r32(slots+4*i)
  if not p:values.append(None)
  elif p in old:values.append(old.index(p)+1)
  else:
   values.append(100);assert new_player is None
   new_player=dict(slot=m.r32(p),ready=bool(m.r8(p+4)),kingdom=m.r32(p+8)==kingdom,
      profile=m.r32(p+0x18)==profile,ego=m.r32(p+0x1c)==ego,association=m.r32(p+0x198))
 def chain(off):
  a=m.r32(manager+off);out=[]
  while a:out.append(m.r32(a));a=m.r32(a+8);assert len(out)<100
  return out
 output=dict(result=result,slots=values,list4c=chain(0x4c),list54=chain(0x54),new_player=new_player)
 pins=dict(h.VISITED);leaves={a:pins.pop(a)for a in LEAVES if a in pins}
 return output,pins,leaves
def main():
 rows=[];pins={};leaves={}
 for pc in h.PRECISION:
  for slots,dup,cat,null in [([],False,0,False),([True,None,True],False,0,False),
      ([True,True],False,0,False),([True],True,0,False),([],False,1,False),([],False,0,True)]:
   c=dict(precision=pc,slots=slots,duplicate=dup,kingdom_category=cat,null_kingdom=null,association=0xdead0001,list4c=[8,9],list54=[4])
   a,ap,al=run(c,'original');b,bp,bl=run(c,'v14');assert a==b
   pins.update(ap|bp);leaves.update(al|bl);rows.append(dict(input=c,output=a))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(O.b),v14_sha256=H(V.b),
  rows=rows,machines=2*len(rows),executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],
  intercepted_entries=[dict(va=hex(a),bytes=b)for a,b in sorted(leaves.items())],dependency_sha256=H((ROOT/'research/phase8-movement/harness.py').read_bytes()))
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-add-ai-player.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),intercepted_entries=len(leaves),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
