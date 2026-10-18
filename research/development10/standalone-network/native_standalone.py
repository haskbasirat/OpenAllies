"""Actual785560 and its three real child constructors, original/v14 all PCs.

Allocator, typed current-variable lookup and Windows QPF/QPC are explicit
laboratory leaves. Real59AA70/59AB60, QR2 key registration8ADA60 and integer
CRT arithmetic execute. Required cutoffs are observation boundaries, not C++
exception/unwind claims. This proves standalone creation, not networking.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_EIP
class Stop(Exception):pass
QPF=0x7700b00;QPC=QPF+0x10
class StandaloneMachine(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'ready',False):
   if pc==0x6f7810:
    size=self.arg();self.trace.append('allocate:'+str(size));self.alloc_count+=1
    if self.cutoff=='allocate:'+str(size):raise Stop()
    if self.fail_alloc==size:self.ret(value=0);return
    p=self.alloc(size);u.mem_write(p,b'\xa5'*size);self.allocations[size]=p;self.ret(value=p);return
   if pc in (QPF,QPC):
    label='frequency'if pc==QPF else 'counter:'+str(self.counter_count)
    self.trace.append(label)
    if self.cutoff==label:raise Stop()
    value=self.frequency if pc==QPF else self.counter+self.counter_count
    if pc==QPC:self.counter_count+=1
    u.mem_write(self.arg(),struct.pack('<q',value));self.ret(4,1);return
   if pc==0x405370:
    name=self.text(self.r(self.arg()));self.trace.append(name)
    if self.cutoff==name:raise Stop()
    value=self.sb if name=='GameSpySBThinkPeriod'else self.qr
    p=self.alloc(8);self.w(p,p+4);self.w(p+4,value);self.ret(4,p);return
   if pc==0x8ada60:
    self.trace.append('key:'+str(self.arg()))
    if self.cutoff=='key:'+str(self.arg()):raise Stop()
  super().hook(u,pc,size,user)
def snap(m,p,offsets):return None if not p else {hex(o):m.r(p+o) for o in offsets}
def run(m,pc,case):
 m.fpu(pc);m.ready=True;m.actual={};m.record_actual=True;m.trace=[];m.allocations={};m.alloc_count=0;m.counter_count=0
 m.cutoff=case.get('cutoff');m.fail_alloc=case.get('fail_alloc');m.frequency=case.get('frequency',10_000_000);m.counter=case.get('counter',123456789)
 m.sb=case.get('sb',50);m.qr=case.get('qr',50)
 for va in [0xb40c9c,0xb40d18,0xb40d1c,0xb40d20,0xb40d28,0xb40d2c,0xb40d30,0xb40d34,0xb40d38,0xb40d3c,0xb4b180,0xb3d768,0xb3d760,0xb3d764,0xb3d758,0xb3d75c]:m.w(va,0)
 m.w(0xb40c9c,case.get('prior_online',0))
 for k in range(50,63):m.w(0xb326b0+k*4,0)
 m.w(0x9f0220,QPF);m.w(0x9f021c,QPC)
 error=None
 try:m.call(0x785560)
 except Stop:error=m.cutoff
 a=m.r(0xb40d28);b=m.r(0xb40d18);c=m.r(0xb40d34)
 keys={str(k):m.u.mem_read(m.r(0xb326b0+k*4),64).split(b'\0')[0].decode()for k in range(50,63)if m.r(0xb326b0+k*4)}
 out={'error':error,'trace':m.trace.copy(),'standalone_flag':m.r(0xb4b180),'online':m.r(0xb40c9c),
  'sb':snap(m,a,[0,8,12,16,20]),'qr':snap(m,b,[0,4,8,12,16,20,24,28,32,36]),
  'hash':snap(m,c,[0,8,12,16]),'bucket_count':None if not c or 0xf94 not in m.allocations else sum(m.r(m.r(c+4)+i*4)==0 for i in range(997)),
  'keys':keys,'timer':{'initialized':m.r(0xb3d768),'frequency_low':m.r(0xb3d760),'frequency_high':m.r(0xb3d764),'quotient':m.r(0xb3d758),'inverse':m.r(0xb3d75c)}}
 return out,m.actual.copy()
def main():
 cases=[{}, {'sb':-1,'qr':16777217},{'sb':-2147483648,'qr':2147483647},
  {'sb':2147483647,'qr':-2147483648},{'sb':0,'qr':0},{'prior_online':0x12345678}]
 cases += [{'fail_alloc':n}for n in [0x18,0x28,0x14]]
 cases += [{'cutoff':x}for x in ['allocate:24','frequency','counter:0','GameSpySBThinkPeriod','allocate:40','counter:1','GameSpyQR2ThinkPeriod','key:50','key:56','key:62','allocate:20','allocate:3988']]
 rows=[];pins={}
 for image in ['original','v14']:
  m=StandaloneMachine(image)
  for a in [QPF,QPC]:m.u.mem_write(a,b'\xc3')
  for pc in PC:
   for case in cases:
    out,p=run(m,pc,case);pins.update(p);rows.append({'image':image,'precision':pc,'input':case,'output':out})
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append({'va':hex(va),'bytes':raw.hex()})
 globals=[]
 for va in [0xb40c9c,0xb40d18,0xb40d28,0xb40d34,0xb4b180]:
  for image,pe in [('original',PE0),('v14',PE1)]:
   vs,rva,raw,off=next(s for s in pe.sections if s[1]<=va-pe.base<s[1]+s[0]);within=va-pe.base-rva
   value=pe.b[off+within:off+within+4]if within+4<=raw else bytes(4);assert value==bytes(4)
   globals.append({'image':image,'va':hex(va),'value':0,'initialized_raw_bytes':within+4<=raw})
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rows':rows,'globals':globals,'instructions':instructions}
 path=HERE/'native-standalone.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'pins':len(instructions),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
