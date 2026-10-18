"""Actual whole5A46D0 outer successful value-copy, both PEs/all PCs.
Nested rules/parameter/entry-array assignment functions are explicitly named
successful callback seams. This fixture proves outer field addresses/order,
not their allocator/reentry behavior or those separate container algorithms.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import UC_X86_REG_ECX

class Machine(BootstrapMachine):
 def __init__(self,image):
  self.copy_active=False
  super().__init__(image)
 def hook(self,u,pc,size,user):
  if self.copy_active:
   if pc in (0x41d130,0x514240,0x514430,0x5a12d0):
    target=u.reg_read(UC_X86_REG_ECX);source=self.arg()
    expected={0x41d130:(self.dr+12,self.sr+12,'rules'),
      0x514240:(self.dp+4,self.sp+4,'teams'),
      0x514430:(self.dp+16,self.sp+16,'kingdoms'),
      0x5a12d0:(self.d+0x44,self.s+0x44,'entries')}[pc]
    assert (target,source)==expected[:2],(hex(pc),hex(target),hex(source))
    self.events.append(expected[2]);self.ret(4,target);return
   if pc==0x405b80:
    target=u.reg_read(UC_X86_REG_ECX);source=self.arg()
    off=source-self.s;assert off in (0x20,0x24,0x28,0x2c,0x38,0x3c)
    assert target-self.d==off
    self.events.append(f'string_{off:02x}')
  super().hook(u,pc,size,user)

def main():
 rows=[];visited={}
 for image in ('original','v14'):
  m=Machine(image);m.begin(1,list(range(16)),0)
  for pc in PC:
   m.fpu(pc)
   for variant in range(3):
    m.s=m.alloc(0x68);m.d=m.alloc(0x68)
    m.u.mem_write(m.s,b'\x5a'*0x68);m.u.mem_write(m.d,b'\xa5'*0x68)
    m.sr=m.alloc(0x20);m.dr=m.alloc(0x20);m.sp=m.alloc(0x28);m.dp=m.alloc(0x28)
    m.w(m.s+4,m.sr);m.w(m.d+4,m.dr);m.w(m.s+8,m.sp);m.w(m.d+8,m.dp)
    words={4:variant+7,8:0x12345678,0x18:0xfedcba98}
    for off,value in words.items():m.w(m.sr+off,value)
    fields={o:((0x80123400+o)^variant*0x11111111)&0xffffffff
      for o in (0xc,0x10,0x14,0x18,0x1c,0x30,0x34,0x40)}
    for off,value in fields.items():m.w(m.s+off,value)
    strings={o:('' if variant==0 else f'field{o:02x}' if variant==1 else f'A\0{o:02x}')
      for o in (0x20,0x24,0x28,0x2c,0x38,0x3c)}
    for off,value in strings.items():m.cs(m.s+off,value);m.cs(m.d+off,'old')
    m.events=[];m.copy_active=True
    returned=m.call(0x5a46d0,[m.s],this=m.d,callee_pop=4)
    m.copy_active=False
    assert returned==m.d
    assert all(m.r(m.dr+off)==value for off,value in words.items())
    assert all(m.r(m.d+off)==value for off,value in fields.items())
    assert all(m.rs(m.d+off)==[ord(c)for c in value]for off,value in strings.items())
    assert m.r(m.d+0x50)==0xa5a5a5a5 and m.r(m.d+0x64)==0xa5a5a5a5
    assert m.events==['rules','teams','kingdoms','string_20','string_24','string_28',
      'string_2c','string_38','string_3c','entries']
    rows.append({'image':image,'precision':pc,'variant':variant,
      'fields':{f'{o:02x}':v for o,v in fields.items()},
      'strings':{f'{o:02x}':[ord(c)for c in value]for o,value in strings.items()},
      'rule_fields':{f'{o:02x}':v for o,v in words.items()},'events':m.events,
      'uncopied_50_64':True})
  visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']
 b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-game-settings.json').write_text(json.dumps(out,indent=2)+'\n')
 print('passed',len(rows),'actual5A46D0 outer-copy rows',flush=True)
if __name__=='__main__':main()
