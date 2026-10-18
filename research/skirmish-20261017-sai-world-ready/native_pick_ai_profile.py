"""Full79FAF0 profile choice including real hash predicates and private85D840 RNG.

Generated explicit source-profile layouts. Only allocation/free and fallback
diagnostic logger are intercepted. Both eligible-pool and full-registry
fallback branches execute; decompiler's unreachable-branch warning is false.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
H=lambda b:hashlib.sha256(b).hexdigest();O=h.P;V=h.V
LEAVES={0x6f7810,0x6f7430,0x404ed0}
PROFILES=[dict(difficulty=0,public=True,nations=[],factions=[]),dict(difficulty=2,public=False,nations=[],factions=[]),
 dict(difficulty=2,public=True,nations=[0x111],factions=[]),dict(difficulty=1,public=True,nations=[],factions=[0x222])]
def run(c,image):
 prior=h.P;h.P=O if image=='original'else V
 try:m=h.Machine(c['precision'])
 finally:h.P=prior
 h.VISITED.clear();logs=[]
 m.seams[0x6f7810]=lambda m:m.ret(value=m.alloc(m.arg()))
 m.seams[0x6f7430]=lambda m:m.ret()
 def log(m):logs.append('fallback');m.ret()
 m.seams[0x404ed0]=log
 manager=m.alloc(0xb0);root=m.alloc(0x900);registry=m.alloc(0x40);array=m.alloc(4*len(PROFILES));rng=m.alloc(4)
 m.w32(0xb36530,root);m.w32(root+0x564,registry);m.w32(registry+0x24,array,len(PROFILES));m.w32(manager+0x2c,rng);m.w32(rng,c['seed'])
 ptrs=[]
 for i,p in enumerate(PROFILES):
  ptr=m.alloc(0x100);ptrs.append(ptr);m.w32(array+4*i,ptr);m.w32(ptr+0x1c,p['difficulty']);m.w8(ptr+0x20,p['public'])
  for off,key in [(0x48,'nations'),(0x58,'factions')]:
   buckets=m.alloc(47*4);m.w32(ptr+off,buckets,47,len(p[key]))
   for value in p[key]:node=m.alloc(12);head=buckets+(value%47)*4;m.w32(node,value,0,m.r32(head));m.w32(head,node)
 result=m.call(0x79faf0,manager,[c['nation'],c['faction']]);assert m.final_esp==h.STACK+12
 pins=dict(h.VISITED);leaves={a:pins.pop(a)for a in LEAVES if a in pins}
 return dict(selected=ptrs.index(result),seed=m.r32(rng),fallback=bool(logs)),pins,leaves
def main():
 rows=[];pins={};leaves={}
 for pc in h.PRECISION:
  for seed in [0,1,0xffffffff,0x12345678]:
   for nation,faction in [(0,0),(0x111,0x222),(0x333,0x444),(0x111,0x444)]:
    c=dict(precision=pc,seed=seed,nation=nation,faction=faction,profiles=PROFILES)
    a,ap,al=run(c,'original');b,bp,bl=run(c,'v14');assert a==b
    pins.update(ap|bp);leaves.update(al|bl);rows.append(dict(input=c,output=a))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(O.b),v14_sha256=H(V.b),
  rows=rows,machines=2*len(rows),executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],
  intercepted_entries=[dict(va=hex(a),bytes=b)for a,b in sorted(leaves.items())],dependency_sha256=H((ROOT/'research/phase8-movement/harness.py').read_bytes()))
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-pick-ai-profile.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),intercepted_entries=len(leaves),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
