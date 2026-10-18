"""Actual546AF0 manager-local operations with native validation/register setup.

String allocation/assignment, dictionary lookup/insertion and logging are explicit
seams. Numeric comparison/arithmetic, validity, trigger lookup and activation,
variable field writes, opcode dispatch and FPU precision execute unchanged.
"""
from pathlib import Path
import copy,hashlib,itertools,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from native_saved_writer import Writer,BASE,STACK,STOP
from decode_census import registry
sys.path.insert(0,str(ROOT/'research/phase4-body'));from original_body import IMAGE,ACCEPTED,pe,bits
from unicorn.x86_const import *
MANAGER,WORLD,CONTEXT=BASE+0x6000,BASE+0x7000,BASE+0x8000
ARG={r['name']:r['index'] for r in registry['arguments']};ITEM={r['name']:r for r in registry['items']};NAME={r['index']:r['name'] for r in registry['elements']}
class Machine(Writer):
 def __init__(self,c):
  self.c=c;self.variable_ptrs={};self.variable_names={};self.diags=[];super().__init__();self.w(0xb36520,MANAGER);self.w(0xb36534,WORLD);self.w(WORLD+0xe0,c['time_bits']);self.w(MANAGER+0x2c,CONTEXT)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.call(0x4c5dc0)
  self.triggerptr=[]
  for t in c['triggers']:
   p=self.alloc(0x38);self.w(p+8,t['id']);self.w(p+4,t['activation_bits']);self.u.mem_write(p+0x34,bytes([t['enabled']]));self.triggerptr.append(p)
  self.w(MANAGER+4,self.linked(self.triggerptr))
  for name,value in c['variables'].items():
   p=self.alloc(8);self.w(p,self.string(name));self.w(p+4,value);self.put_variable(name,p)
 def text(self,p):
  raw=bytearray()
  while True:
   pair=bytes(self.u.mem_read(p,2));p+=2
   if pair==b'\0\0':return raw.decode('utf-16le')
   raw.extend(pair)
 def put_variable(self,name,p):
  node=self.alloc(12);self.w(node+4,p);self.variable_ptrs[name.upper()]=(p,node);self.variable_names[name.upper()]=name
 def call(self,address,owner=0,args=None):
  args=args or [];self.w(STACK,STOP)
  for i,v in enumerate(args):self.w(STACK+4+i*4,v)
  self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,owner);self.u.emu_start(address,STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP
 def hook(self,u,a,n,x):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  sp=u.reg_read(UC_X86_REG_ESP)
  if a==0x405b80:self.w(u.reg_read(UC_X86_REG_ECX),self.r(self.r(sp+4)));self.ret(4)
  elif a==0x4012c0:self.ret(value=self.r(sp+4))
  elif a==0x6f7810:self.ret(value=self.alloc(self.r(sp+4)))
  elif a==0x54ce70:
   name=self.text(self.r(self.r(sp+4)));p=self.r(self.r(sp+8));self.put_variable(name,p);self.ret(8)
  elif a==0x683bd0:
   name=self.text(self.r(self.r(sp+4)));found=self.variable_ptrs.get(name.upper());self.ret(4,found[1] if found else 0)
  elif a in [0x404ed0,0x404f60]:
   if a==0x404ed0:self.diags.append('diagnostic')
   self.ret()
  else:super().hook(u,a,n,x)
 def run(self):
  e=self.c['element'];q=self.alloc(20);definition=self.alloc(0x60);self.w(definition+12,e['opcode']);self.call(0x4c6e40,definition)
  self.w(q,definition);self.w(q+4,len(e['parameters']));self.w(q+12,self.triggerptr[0]);params=self.alloc(0x34*len(e['parameters']));self.w(q+8,params)
  for i,p in enumerate(e['parameters']):self.parameter(p,params+i*0x34)
  # Native reusable argument contexts own empty strings before assignment.
  for i in range(8):self.w(CONTEXT+i*0x24+4,self.string(''));self.w(CONTEXT+i*0x24+8,self.string(''))
  self.call(0x546af0,q,[CONTEXT]);assert self.u.reg_read(UC_X86_REG_ESP)==STACK+8
  return {'result':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'variables':{self.variable_names[k]:self.r(p+4) for k,(p,_) in self.variable_ptrs.items()},'triggers':[{'id':self.r(p+8),'enabled':self.byte(p+0x34),'activation_bits':self.r(p+4)} for p in self.triggerptr],'current_element':self.r(MANAGER+0x4c),'diagnostics':len(self.diags)}
 def byte(self,p):return self.u.mem_read(p,1)[0]
def param(arg,item,value):
 row=ITEM[item];return {'span':[0,0],'version':4,'argument':arg,'item':item,'item_index':row['index'],'storage':row['storage'],'flag':1,'value':value}
def cases():
 rows=[];base={'time_bits':bits(10),'variables':{'counter':bits(.25)},'triggers':[{'id':10,'enabled':1,'activation_bits':bits(1)},{'id':20,'enabled':0,'activation_bits':bits(2)}]}
 def add(name,opcode,params,**kwargs):rows.append({**copy.deepcopy(base),'name':name,'element':{'name':NAME[opcode],'opcode':opcode,'parameters':params},**kwargs})
 add('true',0,[])
 for kind in range(8):
  for left,right in [(-1,0),(0,0),(1,0),(.1,.3),(-0.0,0.0)]:add(f'compare_{kind}_{left}_{right}',1,[param('ma_value','mi_fixed',bits(left)),param('ma_compare','mi_compare',kind),param('ma_value','mi_fixed',bits(right))])
 for value in [0,1,-1,16777217,-16777217,2147483647,-2147483648]:add(f'int_{value}',1,[param('ma_value','mi_int',value&0xffffffff),param('ma_compare','mi_compare',2),param('ma_value','mi_fixed',bits(float(value)))])
 for kind in range(6):
  for value in [0,9,9.000001,10]:add(f'activation_{kind}_{value}',16,[param('ma_compare','mi_compare',kind),param('ma_value','mi_fixed',bits(value))])
 for opcode in [80,81]:
  for id in [10,20,30]:add(f'trigger_{opcode}_{id}',opcode,[param('ma_trigger','mi_trigger',id)])
 for opcode in [91,92,93]:
  for name in ['counter','COUNTER','missing']:
   for value in [.1,-.1,16777216]:add(f'variable_{opcode}_{name}_{value}',opcode,[param('ma_value','mi_variable',name),param('ma_value','mi_fixed',bits(value))])
 add('read_missing_variable',1,[param('ma_value','mi_variable','missing'),param('ma_compare','mi_compare',2),param('ma_value','mi_int',0)])
 add('read_existing_variable',1,[param('ma_value','mi_variable','COUNTER'),param('ma_compare','mi_compare',2),param('ma_value','mi_fixed',bits(.25))])
 add('wrong_arg_class',1,[param('ma_trigger','mi_int',1),param('ma_compare','mi_compare',0),param('ma_value','mi_int',2)])
 add('wrong_param_count',1,[])
 for pc in ['Single24','Double53','Extended64']:
  for row in rows:yield {**copy.deepcopy(row),'name':row['name']+'_'+pc,'precision':pc}
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'seams':['string copy/assignment without native refcount ownership','variable dictionary lookup/insertion with real owned variable fields','heap allocation','diagnostic logging'],'scope':'Actual546AF0 intrinsic operations0/1/16/80/81/91/92/93, native validation and schema setup. Other opcodes not tested.'}
 path=HERE/'native-core-program.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
