"""Run original terrain samplers with real CRT fmod/truncation and x87 state.
No native callback or arithmetic is replaced; only synthetic grid storage and
return observation are supplied. Original/v14 images remain read-only.
"""
from pathlib import Path
import copy,hashlib,json,random,struct,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe,ORIGINAL_SHA,ACCEPTED_SHA
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE,UC_HOOK_MEM_READ
from unicorn.x86_const import *
OBJECT,CELLS,RETURN,OUTPUT,STACK=0x2000000,0x2001000,0x2002000,0x2003000,0x3008000

def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
class Machine:
 def __init__(self,pe,c):
  self.c=c;self.u=Uc(UC_ARCH_X86,UC_MODE_32);pe.load(self.u)
  self.u.mem_map(0x2000000,0x10000);self.u.mem_map(0x3000000,0x10000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']])
  g=c['grid'];self.put(OBJECT+0xc,g['width'],g['height'],g['coordinate_shift'],g['cell_size_bits'],g['index_bias']);self.put(OBJECT+0x40,CELLS);self.put(CELLS,*g['height_bits'])
  self.executed={};self.branches=[];self.reads=[]
  self.u.hook_add(UC_HOOK_CODE,self.hook);self.u.hook_add(UC_HOOK_MEM_READ,self.read_hook)
 def put(self,a,*v):self.u.mem_write(a,struct.pack('<'+'I'*len(v),*(x&0xffffffff for x in v)))
 def hook(self,u,a,size,_):
  if 0x400000<=a<0xc00000:self.executed[a]=size
  names={0x550251:'nearest_fallback',0x55026a:'average_y',0x5502c6:'average_x',0x550317:'bilinear'}
  if a in names:self.branches.append(names[a])
 def read_hook(self,u,access,a,size,value,_):
  if CELLS<=a<CELLS+len(self.c['grid']['height_bits'])*4:
   assert size==4 and (a-CELLS)%4==0
   self.reads.append((a-CELLS)//4)
 def run(self):
  entry=0x550190 if self.c['method']=='Nearest' else 0x550210
  # Preserve the exact x87 return before an additional explicit float32 store.
  stub=b'\xdb\x3d'+struct.pack('<I',OUTPUT)+b'\xdb\x2d'+struct.pack('<I',OUTPUT)+b'\xd9\x1d'+struct.pack('<I',OUTPUT+16)
  self.u.mem_write(RETURN,stub);self.put(STACK,RETURN,*self.c['position_bits'])
  self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,OBJECT)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(entry,RETURN+len(stub),count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==RETURN+len(stub)
  assert self.u.reg_read(UC_X86_REG_ESP)==STACK+12
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  assert bytes(self.u.mem_read(CELLS,len(self.c['grid']['height_bits'])*4))==struct.pack('<'+'I'*len(self.c['grid']['height_bits']),*self.c['grid']['height_bits'])
  sig,exp=struct.unpack('<QH',self.u.mem_read(OUTPUT,10))
  return {'extended_significand':sig,'extended_sign_exponent':exp,'height_bits':struct.unpack('<I',self.u.mem_read(OUTPUT+16,4))[0],'branches':self.branches,'sample_reads':self.reads,'abi_preserved':True,'grid_unchanged':True}

def cases():
 base={'name':'base','method':'Interpolated','precision':'Double53','position_bits':[bits(1.25),bits(6.75)],'grid':{'width':3,'height':3,'coordinate_shift':2,'cell_size_bits':bits(4),'index_bias':2,'height_bits':list(map(bits,[.1,2.3,-4.7,6.1,9.2,12.3,15.1,18.4,21.5]))}}
 rows=[]
 def add(name,xy=None,method='Interpolated',grid=None):
  c=copy.deepcopy(base);c['name']=name;c['method']=method
  if xy is not None:c['position_bits']=list(map(bits,xy))
  if grid:c['grid'].update(grid)
  rows.append(c)
 points=[(0,0),(0,1),(1,0),(1,1),(4,4),(4,5),(5,4),(5,5),(-1,-1),(-1,1),(1,-1),(9,9),(9,5),(5,9),(20,20),(-20,20),(20,-20),(-.0001,.0001),(.0001,-.0001),(.0001,.0001),(3.9999998,4.0000005),(-.9999999,6.5),(2147483520,1),(-2147483648,1)]
 for method in ['Nearest','Interpolated']:
  for i,xy in enumerate(points):add(method+'_'+str(i),xy,method)
 for dimension,vals in [('single',{'width':1,'height':1,'height_bits':[bits(-2.25)]}),('row',{'width':3,'height':1,'height_bits':list(map(bits,[.25,3.25,-8.5]))}),('column',{'width':1,'height':3,'height_bits':list(map(bits,[.25,3.25,-8.5]))})]:
  for i,xy in enumerate([(1,1),(5,1),(1,5),(5,5),(9,9),(0,5),(5,0)]):add(dimension+'_'+str(i),xy,grid=vals)
 for bias in [0,-2,2147483647,-2147483648]:
  for method in ['Nearest','Interpolated']:add('bias_'+str(bias)+'_'+method,(5.25,6.5),method,{'index_bias':bias})
 for shift in [0,1,31,32,34,255]:
  for method in ['Nearest','Interpolated']:add('shift_'+str(shift)+'_'+method,(5.25,6.5),method,{'coordinate_shift':shift})
 for cell in [1.,2.,3.7,16.,.125,-4.,-3.7]:
  for xy in [(1.125,2.25),(0,2.25),(1.125,0),(-1.125,2.25)]:add('cell_'+str(cell)+'_'+str(xy),xy,grid={'cell_size_bits':bits(cell)})
 for code in [0x38d1b716,0x38d1b717,0x38d1b718]:
  value=struct.unpack('<f',struct.pack('<I',code))[0]
  for xy in [(value,value),(value,1),(1,value)]:add('epsilon_'+hex(code)+'_'+str(xy),xy)
 for cell_bits in [1,0x80000001,0x00800000]:
  add('fmod_large_quotient_'+hex(cell_bits),(2147483520,1),grid={'cell_size_bits':cell_bits})
 for zero in [0.,-0.]:
  for method in ['Nearest','Interpolated']:
   for xy in [(zero,zero),(1,1),(0,1),(1,0)]:add('zero_'+str(zero)+'_'+method+'_'+str(xy),xy,method,{'height_bits':[bits(zero)]*9})
 rng=random.Random(550210)
 for i in range(100):
  vals=[bits(rng.uniform(-100,100)) for _ in range(9)]
  add('fractional_'+str(i),(rng.uniform(-2,12),rng.uniform(-2,12)),grid={'height_bits':vals})
 return [{**copy.deepcopy(r),'name':r['name']+'_'+pc,'precision':pc} for pc in ['Single24','Double53','Extended64'] for r in rows]

def main():
 pe=Pe(ROOT.parent/'AA.exe',ORIGINAL_SHA);v14=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA)
 rows=[];executed={}
 for c in cases():
  try:m=Machine(pe,c);output=m.run()
  except Exception as e:raise RuntimeError(c['name']) from e
  executed.update(m.executed);rows.append({'input':c,'output':output})
 ranges=[]
 for start,size in sorted(executed.items()):
  end=start+size
  if ranges and ranges[-1][1]==start:ranges[-1][1]=end
  else:ranges.append([start,end])
 pinned=[]
 for start,end in ranges:
  raw=pe.at(start,end-start);assert raw==v14.at(start,end-start)
  pinned.append({'start':hex(start),'end_exclusive':hex(end),'sha256':hashlib.sha256(raw).hexdigest(),'v14_identical':True})
 constants={}
 for addr in [0xa3a854,0x9fbdb8,0x9f32e0]:
  raw=pe.at(addr,4);assert raw==v14.at(addr,4);constants[hex(addr)]={'bits':struct.unpack('<I',raw)[0],'v14_identical':True}
 doc={'schema':1,'passed':True,'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'oracle_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'case_count':len(rows),'seams':['synthetic heightfield storage; no arithmetic/callback hooks','clean initial x87 state with explicit precision and nearest-even rounding'],'constants':constants,'fixtures':rows}
 p=OUT/'native-heightfield-fixtures.json';p.write_text(json.dumps(doc,indent=2)+'\n')
 (OUT/'executed-ranges.json').write_text(json.dumps(pinned,indent=2)+'\n')
 print(json.dumps({'cases':len(rows),'passed':True,'fixture_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'constants':constants}))
if __name__=='__main__':main()
