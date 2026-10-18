"""Unchanged5784C0 guard/read order, separate original/v14, all x87 PCs.

Actual saved captures stop before unresolved instance/member memory is read.
Coordinator rows provide explicit test-only4CEF10 recording/mutating callbacks;
they do not claim native property admission/insertion/effects were executed.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE
ORIGINAL=h.P
class Required(Exception):pass
def entry(i,kind=2,pass_down=True,disabled=False):
 return dict(handle=1000+i,definition=2000+i,kind=kind,pass_down=pass_down,disabled=disabled)
def coordinator_inputs():
 rows=[]
 for member in ('null','absent','present','experience'):
  rows.append(dict(name='no_owner_'+member,properties=None,member=member,callback='record'))
  rows.append(dict(name='empty_'+member,properties=[],member=member,callback='record'))
 for member in ('null','absent','present','experience'):
  rows.append(dict(name='mixed_'+member,properties=[entry(1,7),entry(2,7,False),entry(3,3,True,True),entry(4,8)],member=member,callback='record'))
 for member in ('present','experience'):
  for kind in (7,8,10,0xffffffff):
   rows.append(dict(name=f'required_{member}_{kind}',properties=[entry(1,kind)],member=member,callback='required'))
 for callback in ('truncate','append','replace_next','replace_owner'):
  rows.append(dict(name='live_'+callback,properties=[entry(1),entry(2,7)],member='present',callback=callback,extra=entry(3,3)))
 return rows
def lookup_inputs():
 rows=[]
 for present in (None,[]):
  for source in (0,4099):rows.append(dict(name=f'find_empty_{present}_{source}',properties=present,property=2001,source=source))
 for source in (0,4099,4101):
  for prop in (0,2001,2002,2999):
   entries=[dict(entry(1,2,True,True),source_id=0x10001,resolved_source=0),dict(entry(2),source_id=0x20002,resolved_source=4099)]
   rows.append(dict(name=f'find_{source}_{prop}',properties=entries,property=prop,source=source))
 for source in (0,4099):
  entries=[dict(entry(1,2,True,True),source_id=0x10001,resolved_source=0),dict(entry(2),source_id=0x20002,resolved_source=4099)]
  entries[1]['definition']=2001
  rows.append(dict(name=f'find_duplicate_{source}',properties=entries,property=2001,source=source))
 return rows
class Probe(h.Machine):
 def __init__(self,c,image,pc,scope):
  h.P=ORIGINAL if image=='original' else h.V;super().__init__(pc)
  self.c=c;self.scope=scope;self.trace=[];self.applies=0
  self.org=self.alloc(0x40);self.owner=self.alloc(0x180);self.member=self.alloc(0x180);self.other_owner=self.alloc(0x180)
  self.w32(self.org+4,self.owner);self.props=0
  self.entries=[];self.ih={};self.dh={};self.labels={self.owner:4099,self.member:4100,self.other_owner:4101}
  values=c['properties']
  if values is not None:
   self.props=self.alloc(0x40);self.slots=self.alloc(256*4);self.w32(self.owner+0x78,self.props)
   self.w32(self.props+0x10,self.slots,len(values))
   if scope=='coordinator':
    for e in values:self.entries.append(self.make_entry(e))
    for i,p in enumerate(self.entries):self.w32(self.slots+4*i,p)
   else:
    for i,value in enumerate(values):self.w32(self.slots+4*i,value)
  if c.get('member') not in (None,'null','absent','stale'):self.w32(self.member+0x78,self.alloc(0x40))
  if c.get('member')=='experience':self.w32(self.member+0xa4,self.alloc(0x40))
  self.u.hook_add(UC_HOOK_CODE,self.observe)
 def make_entry(self,e):
  p=self.alloc(0x20);d=self.alloc(0x80);self.ih[p]=e['handle'];self.dh[d]=e['definition']
  self.w32(p+8,e['kind'],d);self.w8(p+0x10,e['disabled']);self.w8(d+0x42,e['pass_down']);return p
 def reg(self,r):return self.u.reg_read(r)
 def observe(self,u,pc,size,_):
  if self.scope=='actual_saved':
   if pc==0x5784ea and self.c['member']=='stale':raise Required('actor_required')
   if pc==0x5784f1:raise Required('instance_required')
   if pc==0x5032f6:raise Required('instance_required')
   return
  eax=self.reg(h.UC_X86_REG_EAX);ecx=self.ecx()
  if pc in (0x5784c3,0x578512):self.trace.append(['owner',self.labels[self.r32(self.org+4)]])
  elif pc==0x5784c7:self.trace.append(['properties',4099,bool(self.props)])
  elif pc in (0x5784ce,0x578522):self.trace.append(['count',self.r32(self.props+0x14)])
  elif pc==0x5784e5:
   i=self.reg(h.UC_X86_REG_EBX);p=self.r32(ecx+4*i);self.trace.append(['slot',i,self.ih[p]])
  elif pc==0x5784ea:self.trace.append(['properties',4100,bool(self.r32(self.member+0x78))])
  elif pc==0x5784f1:self.trace.append(['definition',self.ih[eax],self.dh[self.r32(eax+0xc)]])
  elif pc==0x5784f4:self.trace.append(['pass_down',self.dh[ecx],bool(self.r8(ecx+0x42))])
  elif pc==0x5784fb:self.trace.append(['experience',4100,bool(self.r32(self.member+0xa4))])
  elif pc in (0x578505,0x578515):self.trace.append(['kind',self.ih[eax],self.r32(eax+8)])
  elif pc==0x57850b:self.trace.append(['disabled',self.ih[eax],bool(self.r8(eax+0x10))])
  elif pc==0x4cef10:
   self.trace.append(['apply',self.labels[self.ecx()],self.dh[self.arg()],self.arg(1),self.labels[self.arg(2)]])
   self.applies+=1;callback=self.c['callback']
   if callback=='required':raise Required('apply_required')
   if callback=='truncate':self.entries=self.entries[:1];self.w32(self.props+0x14,1)
   elif callback=='append' and self.applies==1:
    self.entries.append(self.make_entry(self.c['extra']));self.w32(self.slots+4*(len(self.entries)-1),self.entries[-1]);self.w32(self.props+0x14,len(self.entries))
   elif callback=='replace_next' and self.applies==1 and len(self.entries)>1:
    self.entries[1]=self.make_entry(self.c['extra']);self.w32(self.slots+4,self.entries[1])
   elif callback=='replace_owner' and self.applies==1:self.w32(self.org+4,self.other_owner)
   self.ret(12,1)
 def run(self):
  outcome='ok'
  try:self.call(0x5784c0,self.org,[0 if self.c['member']=='null' else self.member])
  except Required as e:outcome=str(e)
  if self.scope=='actual_saved':
   finds=[]
   #5032E3 captures registry base even when no source filter is requested.
   #No field in this unavailable arena is answered for a nonempty list.
   self.w32(0xb364c0,self.alloc(0x60004))
   for prop,source in ((0,0),(0x7fffffe0,self.owner)):
    try:finds.append(bool(self.call(0x4cef80,self.owner,[prop,source])))
    except Required as e:finds.append(str(e))
   return dict(outcome=outcome,properties=self.c['properties'],find=finds)
  def original(p):
   handle=self.ih[p]
   return next(e for e in (self.c['properties'] or [])+[self.c.get('extra',{})] if e.get('handle')==handle)
  return dict(outcome=outcome,trace=self.trace,entries=[original(p) for p in self.entries])
class FindProbe(Probe):
 def __init__(self,c,image,pc):
  super().__init__(c,image,pc,'coordinator');self.manager=self.alloc(0x60004);self.w32(0xb364c0,self.manager)
  self.definitions={}
  for p,e in zip(self.entries,c['properties'] or []):
   d=self.definitions.setdefault(e['definition'],self.r32(p+0xc));self.w32(p+0xc,d)
  for p,e in zip(self.entries,c['properties'] or []):
   gtid=e['source_id'];self.w32(p+0x14,gtid);index=gtid&0xffff
   if e['resolved_source']:
    self.u.mem_write(self.manager+4+index*2,(gtid>>16).to_bytes(2,'little'))
    self.w32(self.manager+0x20004+index*4,self.owner if e['resolved_source']==4099 else self.other_owner)
 def observe(self,u,pc,size,_):
  if pc==0x4cef80:self.trace.append(['properties',4099,bool(self.props)])
  elif pc==0x5032d6:self.trace.append(['count',self.r32(self.props+0x14)])
  elif pc==0x5032f0:
   index=self.reg(h.UC_X86_REG_EDI);p=self.r32(self.reg(h.UC_X86_REG_EBX));self.trace.append(['slot',index,self.ih[p]])
  elif pc==0x5032f6:
   p=self.reg(h.UC_X86_REG_EDX);self.trace.append(['definition',self.ih[p],self.dh[self.r32(p+0xc)]])
  elif pc==0x503323:
   p=self.reg(h.UC_X86_REG_EDX);self.trace.append(['source',self.ih[p],self.labels.get(self.ecx(),0)])
  elif pc==0x503342:
   index=self.reg(h.UC_X86_REG_EDI);p=self.r32(self.ecx()+index*4);self.trace.append(['slot',index,self.ih[p]])
 def run(self):
  source=self.c['source'];source=0 if source==0 else self.owner if source==4099 else self.other_owner
  prop=self.c['property'];prop=self.definitions.get(prop,0 if prop==0 else self.alloc(0x80))
  result=self.call(0x4cef80,self.owner,[prop,source])
  return dict(found=self.ih[result] if result else None,trace=self.trace)
def main():
 cases=[]
 actual=json.loads((HERE/'actual-saved-inputs.json').read_text())['cases']
 for image in ('original','v14'):
  for pc in h.PRECISION:
   for c in coordinator_inputs():
    cases.append(dict(scope='coordinator',name=c['name'],image=image,precision=pc,input=c,output=Probe(c,image,pc,'coordinator').run()))
   for c in lookup_inputs():
    cases.append(dict(scope='lookup',name=c['name'],image=image,precision=pc,input=c,output=FindProbe(c,image,pc).run()))
  for row in actual:
   c=row['input'];pc=row['precision']
   cases.append(dict(scope='actual_saved',name=c['name'],image=image,precision=pc,input=c,output=Probe(c,image,pc,'actual_saved').run()))
 document=dict(schema=1,scope='5784C0 live guard/read coordinator with explicit4CEF10 callbacks and actual saved source boundaries',original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,cases=cases,
  instructions=[dict(va=hex(va),bytes=raw) for va,raw in sorted(h.VISITED.items())])
 path=HERE/'native-member-properties.json'
 if '--write' in sys.argv:path.write_text(json.dumps(document,indent=1)+'\n')
 else:assert json.loads(path.read_text())==document,'frozen native evidence differs'
 print(json.dumps(dict(cases=len(cases),instructions=len(h.VISITED),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
