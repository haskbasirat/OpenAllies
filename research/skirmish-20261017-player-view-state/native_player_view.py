"""Focused unchanged native Player+50 constructor/retirement oracle.

The inherited bootstrap harness supplies explicit CString allocator/refcount,
Localization table and CRT string-manager seams; it does not certify a whole
application. Real4ED6D0/4ED160/4E1020/4EFB10/4ED210/4E3ED0/4E28D0 and765E10
execute. Both images run separately at all three precision controls. No capture
is overwritten without --write. Native heapfailure/SEH unwind is not admitted.
"""
from pathlib import Path
import argparse, hashlib, json, sys
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-local-bootstrap'))
from native_bootstrap import BootstrapMachine, PE0, PE1
from unicorn.x86_const import UC_X86_REG_FPCW, UC_X86_REG_EIP
PCS={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Probe(BootstrapMachine):
 def __init__(self,image):
  self.monitor=None;self.trace=[];self.intercepted=set()
  super().__init__(image)
  self.visited={};self.intercepted=set()
 def hook(self,u,pc,size,user):
  if self.monitor and pc in (0x4ed210,0x4e3ed0,0x4e28d0,0x6f7430):
   v=self.monitor
   self.trace.append({'event':f'{pc:08X}','head':self.r(v+4)!=0,
    'tail':self.r(v+8)!=0,'wildcard':self.r(v+0x24)!=0,
    'free':self.arg() if pc==0x6f7430 else None})
  super().hook(u,pc,size,user)
  if u.reg_read(UC_X86_REG_EIP)!=pc and PE0.base<=pc<PE0.base+PE0.image_size:self.intercepted.add(pc)
 def view(self,p):
  group=self.r(p+0x24)
  actors=[];n=self.r(p+4)
  while n:actors.append(self.r(n));n=self.r(n+4)
  return {'kingdom':self.r(p),'word_0c':self.r(p+0xc),
   'mask_words': [self.r(p+o)for o in(0x10,0x14,0x18,0x1c)],
   'group_null':self.r(p+0x20)==0,'look_at':[self.r(p+0x28),self.r(p+0x2c)],
   'selection':actors,'tail_null':self.r(p+8)==0,
   'wildcard':None if not group else {'key':self.rs(group),'title':self.rs(group+4),'text':self.rs(group+8),
    'members_count':self.r(group+0x10),'capacity':self.r(group+0x14),
    'initial_slot':self.r(self.r(group+0xc)),'flag_18':self.rb(group+0x18),
    'word_1c':self.r(group+0x1c),'flag_20':self.rb(group+0x20)}}
 def run(self,c):
  self.monitor=None;self.trace=[];self.visited={};self.intercepted=set()
  self.u.reg_write(UC_X86_REG_FPCW,PCS[c['pc']])
  self.formats={'static_group_wildcard_name':c['caption']}
  v=self.alloc(0x30);self.u.mem_write(v,b'\xa5'*0x30)
  self.call(0x4ed6d0,[c['kingdom']],this=v,callee_pop=4)
  before=self.view(v);self.monitor=v;allocated={}
  for i,a in reversed(list(enumerate(c['selected']))):
   n=self.alloc(8);self.w(n,a);self.w(n+4,self.r(v+4));self.w(v+4,n)
   if not self.r(v+8):self.w(v+8,n)
   allocated[n]=f'selection:{i}'
  group=self.r(v+0x24);allocated[group]='wildcard';allocated[self.r(group+0xc)]='member_buffer'
  if c['operation']=='destroy':
   self.call(0x4ed210,this=v)
   after=self.view(v)
  elif c['operation']=='replace':
   player=self.alloc(0x54);self.u.mem_write(player,b'\xa5'*0x54);self.w(player+0x50,v)
   self.call(0x765e10,[c['new_kingdom']],this=player,callee_pop=4)
   after=self.view(self.r(player+0x50));after['raw28']=self.r(player+0x28)
  else:after=self.view(v)
  #String frees carry allocator-local identities, not native Actor GTIDs.
  for event in self.trace:
   if event['free'] is not None:event['free']=allocated.get(event['free'],'string')
  return {'case':c,'before':before,'after':after,'trace':self.trace}
def cases():
 rows=[]
 for pc in PCS:
  for kingdom in (0,0x01000055):
   for caption in ('All Units','Toutes les unités Ω'):
    for op in ('construct','destroy','replace'):
     for selected in ([[]]if op=='construct' else [[],[0x01000010,0x01000011]]):
      rows.append({'pc':pc,'kingdom':kingdom,'caption':caption,'operation':op,'selected':selected,
       'new_kingdom':0x02000056 if kingdom==0 else 0})
 return rows
def main():
 args=argparse.ArgumentParser();args.add_argument('--write',action='store_true');args=args.parse_args()
 rows=[];visited={};intercepted=set()
 machines={(pc,image):Probe(image) for pc in PCS for image in ('original','v14')}
 for c in cases():
  pair=[]
  for image in('original','v14'):
   p=machines[(c['pc'],image)];assert p.image==image
   pair.append(p.run(c));visited.update(p.visited);intercepted.update(p.intercepted)
  assert pair[0]==pair[1],c
  rows.append(pair[0])
 #Classify each inherited pre-hook pin after actual EIP redirection. Exclude
 #all intercepted entry bytes from executed-instruction counts.
 pins=[]
 for addr,size in sorted(visited.items()):
  if PE0.base<=addr<PE0.base+PE0.image_size:
   a=PE0.range(addr,size);b=PE1.range(addr,size);assert a==b,hex(addr)
   pins.append({'address':f'{addr:08X}','bytes':a.hex(),'classification':'intercepted entry' if addr in intercepted else 'executed'})
 result={'scope':'successful allocation/CString/localization seams; constructed wildcard has zero members',
  'rows':rows,'observed_address_pins':pins,'original_sha256':hashlib.sha256(PE0.b).hexdigest(),
  'v14_sha256':hashlib.sha256(PE1.b).hexdigest()}
 path=HERE/'native-player-view.json'
 if args.write:path.write_text(json.dumps(result,indent=2),encoding='utf-8')
 else:assert json.loads(path.read_text(encoding='utf-8'))==result
 print(json.dumps({'rows':len(rows),'native_runs':2*len(rows),'address_pins':len(pins),'executed_pins':sum(p['classification']=='executed' for p in pins),'intercepted_pins':sum(p['classification']=='intercepted entry' for p in pins),'fixture_sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
