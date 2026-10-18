"""Original57E670/4BE6C0 execute unhooked against authored-layout fixture weights.
Final57E670 result is native-f32 stored maximum returned through x87.
"""
from native_constructors import *
from unicorn import UC_HOOK_CODE
class Machine(Base):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=c;self.visited={}
  self.u.mem_map(0x4000000,0x40000);self.w(PERF+4,OWNER);self.w(OWNER+4,DEF);self.w(PERF+0x14,ARRAY);self.w(PERF+0x18,len(c['entries']))
  for i,v in enumerate(c['actor_bonuses']):self.w(DEF+0x204+i*4,v)
  for i,e in enumerate(c['entries']):
   a=0x4010000+i*0x100;l=LOCAL+i*0x80;d=SHARED+i*0x100;self.w(ARRAY+i*4,a);self.w(a+4,d);self.w(a+8,l);self.u.mem_write(a+0x18,bytes([e['enabled']]));self.w(d+0x28,e['targets']['required']);self.w(d+0x2c,e['targets']['forbidden']);w=e['weights'];self.w(l+0x20,w['priority'])
   for j,v in enumerate(w['factors']):self.w(l+0x24+j*4,v)
   for j,v in enumerate(w['bonuses']):self.w(l+0x30+j*4,v)
  if c['excluded'] is not None:self.w(PROP,PROP+0x100);self.w(PROP+4,PROP+0x100);self.w(PROP+0x128,c['excluded']['required']);self.w(PROP+0x12c,c['excluded']['forbidden'])
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.visited[pc]=raw.hex()
 def run(self):
  self.reset(0);self.w(self.STACK+4,PROP if self.c['excluded'] is not None else 0);self.u.reg_write(UC_X86_REG_ECX,PERF)
  regs={r:0x12ef0000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for r,v in regs.items():self.u.reg_write(r,v)
  self.u.emu_start(0x57e670,self.STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert all(self.u.reg_read(r)==v for r,v in regs.items())
  self.u.mem_write(self.STOP,b'\xd9\x1d'+struct.pack('<I',self.STATE));self.u.emu_start(self.STOP,self.STOP+6,count=1)
  return self.r(self.STATE)
def cases():
 rows=[]
 for count in [0,1,3,7]:
  for excluded in [None,{'required':2,'forbidden':4},{'required':1,'forbidden':4}]:
   for enabled in [True,False]:
    rows.append({'name':f'{count}_{excluded}_{enabled}','actor_bonuses':list(map(bits,[1.717,-2.1,.3333333])),'excluded':excluded,'entries':[{'enabled':enabled if i%2==0 else not enabled,'targets':{'required':i%3,'forbidden':4},'weights':{'priority':bits((i-2)*.3333333),'factors':list(map(bits,[2.717,3.17,-.819])),'bonuses':list(map(bits,[1.19,-2.5,3.77]))}}for i in range(count)]})
 return [{**copy.deepcopy(c),'precision':pc,'name':c['name']+pc}for pc in ['Single24','Double53','Extended64']for c in rows]
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 doc={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 (HERE/'native-weight-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
