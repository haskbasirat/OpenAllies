"""Actual native current-reference readers, both immutable PEs/all x87 PCs.

76A4B0/76A5A0 execute the real40D5D0 bit reader and current slot directory;
Actor/Kingdom B4 dispatch executes real6A71D0.765F70 executes real76B8F0
over authored linked live/film player records. No source/lookup/virtual/codec
callbacks are supplied. Memory layouts are explicit laboratory inputs, not
proof of a completed game's registry construction or an activated Frame.
"""
from pathlib import Path
import argparse,json,sys,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
P=h.P;V=h.V
H=lambda b:hashlib.sha256(b).hexdigest()

class Machine(h.Machine):
 def __init__(self,pc):
  super().__init__(pc)
  self.registry=self.alloc(0x60004);self.stream=self.alloc(16)
  self.data=self.alloc(16);self.output=self.alloc(4);self.session=self.alloc(0x200)
  self.actor=self.alloc(0x200);self.kingdom=self.alloc(0x400)
  self.w32(self.actor,0xa273f0);self.w32(self.kingdom,0xa299d8)
  self.w32(0xb364c0,self.registry);self.w32(0xb36550,self.session)
  self.players=[]
  #Same-ID film/live entries intentionally test actual first matching record.
  tail=0
  for ident,film in reversed([(0,0),(7,1),(7,0),(0xffffffff,0),(7,0)]):
   p=self.alloc(0x80);node=self.alloc(8)
   self.w32(p+0x20,ident);self.w8(p+0xe,film);self.w32(node,p,tail)
   tail=node;self.players.insert(0,p)
  self.w32(self.session+0xbc,tail)
 def read(self,entry,value,bits):
  #At initial bit0 the native raw primitive emits the final partial byte
  #first, then the preceding little-endian whole bytes (phase3 cursor proof).
  raw=value.to_bytes(bits//8,'little')
  self.u.mem_write(self.data,raw[-1:]+raw[:-1]+b'\0'*8)
  self.w32(self.stream,0,self.data,0,0);self.w32(self.output,0xcdcdcdcd)
  self.call(entry,0,[self.stream,self.output])
  assert self.final_esp==h.STACK+4 and self.fpu_top()==0
  assert self.r32(self.stream+8)==bits//8-1 and self.r32(self.stream+12)==8
  return self.r32(self.output)
 def thing(self,c):
  slot=c['slot'];p={'none':0,'actor':self.actor,'kingdom':self.kingdom}[c['kind']]
  self.w32(self.registry+0x20004+slot*4,p)
  #Native reader ignores upper generation16. The pointed Thing ID retains it.
  if p:self.w32(p+0x14,(c['generation']<<16)|slot)
  actual=self.read(0x76a4b0 if c['check']=='Any'else 0x76a5a0,slot,16)
  assert actual==p
  return {'resolved':actual!=0,'slot':None if not actual else self.r32(actual+0x14)&0xffff,
          'position':[self.r32(self.stream+8),self.r32(self.stream+12)]}
 def player(self,c):
  actual=self.read(0x765f70,c['id'],32)
  expected=next((p for p in self.players if self.r32(p+0x20)==c['id'] and self.r8(p+0xe)==0),0)
  assert actual==expected,(c,hex(actual),hex(expected),self.r32(self.output))
  return {'ordinal':None if not actual else self.players.index(actual),'id':None if not actual else self.r32(actual+0x20),
          'position':[self.r32(self.stream+8),self.r32(self.stream+12)]}

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');a=ap.parse_args()
 rows=[]
 things=[dict(check=check,slot=slot,kind=kind,generation=gen)for check,slot,kind,gen in itertools.product(['Any','B4'],[0,1,0xffff],['none','actor','kingdom'],[0,1,0xffff])]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   m=Machine(pc)
   for c in things:rows.append(dict(image=label,precision=pc,kind='thing',input=c,output=m.thing(c)))
   for ident in [0,7,8,0xffffffff]:
    c={'id':ident};rows.append(dict(image=label,precision=pc,kind='player',input=c,output=m.player(c)))
 h.P=P
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 base=[{k:v for k,v in r.items()if k not in ['image','precision']}for r in rows[:len(things)+4]]
 for n in range(1,6):assert [{k:v for k,v in r.items()if k not in ['image','precision']}for r in rows[n*len(base):(n+1)*len(base)]]==base
 report=dict(scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,producer_sha256=H(Path(__file__).read_bytes()),rows=rows,
             vtable_pins={hex(t):{'B4':hex(P.u32(t+0xb4))}for t in [0xa273f0,0xa299d8]},instructions=[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())])
 p=HERE/'native-references.json'
 if a.write:p.write_text(json.dumps(report,indent=1)+'\n')
 else:assert json.loads(p.read_text())==report
 print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
