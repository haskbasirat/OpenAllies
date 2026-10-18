"""Real4B3C10 source Bool registration then existing-variable overwrite.
Native5BA010 Boolean source nodes, Settings ctor/hash lookup/Add, typed getters,
4B3190 and actual raw byte store execute. File/source-label/String/heap host
transports follow the existing native-settings-order harness. Native wildcard
ordering is separately pinned/proved in that baseline; no OS enumeration claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-definitions'));from probe_lists import Machine as Base
import trace_registry as T
from unicorn.x86_const import *
from unicorn import UC_HOOK_CODE
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
ORIGINAL=T.P;EXPANDED=T.V
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stop(Exception):pass
class Machine(Base):
 def hook(self,u,pc,n,z):
  if hasattr(self,'monitor')and self.monitor and pc in self.labels:
   label=self.labels[pc];self.events.append(label)
   if self.stop_at==label:raise Stop(label)
  if pc in(0x404f60,0x404ed0,0x404ea0):self.ret()
  elif pc==0x5b3b10:
   out=self.arg();self.w(out,self.string(list(map(ord,'explicit-source-fixture'))));self.ret(out,pop=4)
  else:super().hook(u,pc,n,z)
 def make(self,value):
  node=self.node('SVars');parser=self.alloc(0x80);token=self.alloc(0x30)
  self.w(parser,node);self.w(parser+0x40,self.string(list(map(ord,'SupplyProvideUnderSiege'))));self.w(parser+0x38,5)
  self.w(token+0x18,self.string(list(map(ord,value))));self.w(token+0x10,7)
  self.invoke(0x5ba010,parser,[token]);return node

def run(image,pc,initial,value,stop=None):
 T.P=ORIGINAL if image=='original'else EXPANDED;m=Machine();m.u.reg_write(UC_X86_REG_FPCW,PC[pc]);L.guard_null_page(m.u)
 executed={};seams=set()
 def pin(u,at,size,_):
  if ORIGINAL.base<=at<ORIGINAL.base+ORIGINAL.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:executed[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,pin)
 settings=m.alloc(0x80);m.invoke(0x4b2ce0,settings,[])
 m.invoke(0x4b3c10,settings,[m.make(initial)])
 first=m.r(m.r(settings+8));before=dict(value=m.r(first)&255,kind=m.r(first+36),flags=m.r(first+40),count=m.r(settings+12))
 node=m.make(value);m.events=[];m.labels={0x4b3cda:'named_lookup',0x4b3fd4:'load_flags',0x4b3fea:'source_type',
  0x4b4207:'registered_type',0x4b4231:'var_flags',0x4b427d:'source_value',0x4b4287:'setter',0x4b3202:'store'}
 m.stop_at=stop;m.monitor=True;stopped=None
 try:m.invoke(0x4b3c10,settings,[node])
 except Stop as e:stopped=str(e)
 var=m.r(m.r(settings+8));after=dict(value=m.r(var)&255,kind=m.r(var+36),flags=m.r(var+40),count=m.r(settings+12),same_var=var==first)
 return dict(before=before,after=after,events=m.events,stopped=stopped),executed,seams

def main():
 rows=[];pins={};seams=set()
 for initial,value in [('false','true'),('true','false'),('true','true'),('false','false')]:
  baseline,_,_=run('original','Double53',initial,value)
  for stop in [None]+baseline['events']:
   for pc in PC:
    a,pa,sa=run('original',pc,initial,value,stop);b,pb,sb=run('v14',pc,initial,value,stop);assert a==b
    assert a['after']['kind']==1 and a['after']['flags']==0 and a['after']['count']==1 and a['after']['same_var']
    assert a['after']['value']==int((value if stop is None else initial)=='true')
    rows.append(dict(pc=pc,input=dict(initial=initial,value=value),stop=stop,output=a));pins.update(pa);pins.update(pb);seams|=sa|sb
 out=dict(schema=1,scope=__doc__,rows=rows,executed=[dict(address=hex(a),bytes=pins[a])for a in sorted(pins)],seams=[hex(a)for a in sorted(seams)])
 raw=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'native-supply-source-order.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
