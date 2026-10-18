"""Concrete native Properties/Modifier constructors and scalar/type getters."""
from pathlib import Path
import copy,csv,hashlib,json,random,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe,bits
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
PROP,MOD,OWNER,DOMAIN,ROOTDATA,CACHE,SETTINGS,ARRAY,ENTRIES,STACK,STOP,OUTPUT,HEAP=[0x2000000+i*0x1000 for i in range(13)]
MULTIPLY={0x38:0x5014c0,0x3c:0x5014e0,0x44:0x501500,0x50:0x5014f0,0x54:0x501520}
DIVIDE={0x40:0x5014d0,0x48:0x501510,0x64:0x501560}
ADD={offset:0x501b10+index*0x60 for index,offset in enumerate(range(0x10,0x30,4))}
FUNCTIONS=[0x5812f0,0x4db020,0x50ab90,0x502750,0x501530,0x501610,0x501650,*MULTIPLY.values(),*DIVIDE.values(),*ADD.values()]
class Machine:
 def __init__(self,c):
  self.c=c;self.visited={};self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0,4096);self.u.mem_map(0x2000000,0x20000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.u.hook_add(UC_HOOK_CODE,self.hook)
  self.w(0xb36530,ROOTDATA);self.w(ROOTDATA+0x3a8,DOMAIN);self.w(DOMAIN+0x28,c['domain_count']);self.w(0xb39ab0,CACHE);self.w(CACHE+0x1a4,SETTINGS);self.w(SETTINGS,c['minimum'])
  self.u.mem_write(PROP,b'\xa5'*0xe0);self.u.mem_write(MOD,b'\xa5'*0xc0)
  for i,value in enumerate(c['scalars']):self.w(MOD+4+i*4,value)
  if c['method']=='immune':
   self.w(MOD+0x68,ARRAY);self.w(MOD+0x6c,len(c['immunities']))
   for i,value in enumerate(c['immunities']):self.w(ARRAY+i*4,value or 0)
  if c['method']=='factor':
   self.w(MOD+0x74,ARRAY);self.w(MOD+0x78,len(c['factors']))
   for i,value in enumerate(c['factors']):
    self.w(ARRAY+i*4,ENTRIES+i*8);self.w(ENTRIES+i*8,value['damage_type']);self.w(ENTRIES+i*8+4,value['factor'])
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def hook(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a==0x6f7810:
   sp=u.reg_read(UC_X86_REG_ESP);assert self.r(sp+4)==8;u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4);u.reg_write(UC_X86_REG_EAX,HEAP)
 def run(self):
  method=self.c['method'];self.w(STACK,STOP);self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,PROP if method=='properties_new' else MOD)
  if method=='properties_new':entry=0x5812f0;args=[OWNER]
  elif method=='modifier_new':entry=0x502750;args=[]
  elif method=='multiply':entry=MULTIPLY[self.c['offset']];args=[self.c['base']]
  elif method=='divide':entry=DIVIDE[self.c['offset']];args=[self.c['base']]
  elif method=='add':entry=ADD[self.c['offset']];args=[self.c['base']]
  elif method=='offense':entry=0x501530;args=[]
  elif method=='immune':entry=0x501610;args=[self.c['identity']]
  elif method=='factor':entry=0x501650;args=[self.c['identity']]
  else:raise ValueError(method)
  for i,value in enumerate(args):self.w(STACK+4+i*4,value)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(entry,STOP,count=10000);assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+4+len(args)*4
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  if method in ['properties_new','modifier_new']:
   address=PROP+0x1c if method=='properties_new' else MOD
   for offset in range(0x68,0xbc,12):assert self.r(address+offset)==0 and self.r(address+offset+4)==0
   m={'flags':self.r(address),'scalars':[self.r(address+4+i*4) for i in range(25)],'immunity_types':[],'damage_factors':[],'other_tables':[[] for _ in range(5)],'growth_hints':[self.r(address+offset+8) for offset in range(0x68,0xbc,12)]}
   if method=='modifier_new':return {'modifier':m,'abi_preserved':True}
   assert self.r(PROP+4)==OWNER and self.r(PROP+0x10)==HEAP and self.r(PROP+0x14)==0
   return {'properties':{'owner':1,'scheduler_link':None if self.r(PROP+0xc)==0 else self.r(PROP+0xc),'effects':[],'effect_growth_hint':self.r(PROP+0x18),'modifier':m,'modifier_flags':self.r(PROP+0xd8)},'abi_preserved':True}
  if method=='immune':return {'result':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'abi_preserved':True}
  self.u.mem_write(STOP,b'\xd9\x15'+struct.pack('<I',OUTPUT)+b'\xdb\x3d'+struct.pack('<I',OUTPUT+8));self.u.emu_start(STOP,STOP+12)
  sig,exp=struct.unpack('<QH',self.u.mem_read(OUTPUT+8,10));return {'stored':self.r(OUTPUT),'extended_significand':sig,'extended_sign_exponent':exp,'abi_preserved':True}
def cases():
 base={'domain_count':19,'minimum':bits(2),'scalars':[0]*25,'base':bits(10),'offset':0x10,'identity':7,'immunities':[],'factors':[]};rows=[]
 def add(name,method,fn=None):
  c=copy.deepcopy(base);c.update(name=name,method=method)
  if fn:fn(c)
  rows.append(c)
 for count in [0,1,8,19,1000]:
  for method in ['properties_new','modifier_new']:add(method+str(count),method,lambda c,count=count:c.update(domain_count=count))
 for method,offsets in [('multiply',MULTIPLY),('divide',DIVIDE),('add',ADD)]:
  for offset in offsets:
   for factor,initial,minimum in [(.1,.3,.2),(2,10,2),(-2,1,3),(.5,-1,0),(100,0,2),(-10,20,5),(.0001,.25,-1)]:
    if method=='divide' and factor==0:continue
    add(f'{method}_{offset}_{factor}_{initial}_{minimum}',method,lambda c,o=offset,f=factor,b=initial,m=minimum:(c.update(offset=o,base=bits(b),minimum=bits(m)),c['scalars'].__setitem__(o//4-1,bits(f))))
 for a,b in [(0,0),(-1,2),(1,2),(2,-1),(-1,-2),(.1,.2)]:add(f'offense_{a}_{b}','offense',lambda c,a=a,b=b:(c['scalars'].__setitem__(21,bits(a)),c['scalars'].__setitem__(22,bits(b))))
 for values in [[],[7],[8],[None],[8,None],[8,7],[7,7]]:add('immunity_'+str(values),'immune',lambda c,v=values:c.update(immunities=v))
 for values in [[],[(7,.25)],[(8,2)],[(7,.25),(7,.75)],[(8,2),(7,.5)]]:add('factor_'+str(values),'factor',lambda c,v=values:c.update(factors=[{'damage_type':k,'factor':bits(f)} for k,f in v]))
 for p in ['Single24','Double53','Extended64']:
  for row in rows:yield {**copy.deepcopy(row),'name':row['name']+'_'+p,'precision':p}
def main():
 sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')};ranges=[]
 for a in FUNCTIONS:
  raw=IMAGE.at(a,sizes[a]);assert raw==ACCEPTED.at(a,len(raw));ranges.append({'address':hex(a),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
  (HERE/f'{a:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,a))+'\n')
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'ranges':ranges,'visited_windows':windows,'cases':rows,'seams':['only the8-byte empty effect-vector allocation in Properties constructor; getter/Modifier constructor cases have no behavioral hooks']}
 path=HERE/'native-properties-fixtures.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
