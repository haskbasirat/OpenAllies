"""Original x86 position/query/snap routines with explicit world leaves.

No game process, source archive or native image is modified. Height sampling,
virtual shape access, tile ownership and presentation leaves are fixture seams.
The query, four descriptor constructors, snapping/CRT floor and commit run.
"""
from pathlib import Path
import copy, hashlib, json, random, struct, sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase3-terrain'))
from terrain_uv_oracle import Pe, ORIGINAL_SHA, ACCEPTED_SHA
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import *

OUT=Path(__file__).parent
A,W,VT,GRID,CELLS,DEFS,BH,OH,MGR,RETURN,VALUES,ARG,STOP=[0x2000000+i*0x1000 for i in range(13)]
SA,SB,POS=0x2010000,0x2010010,0x2010020
STACK=0x3008000
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def words(*xs):return list(map(bits,xs))
class Machine:
 def __init__(self,pe,c):
  self.c=c;self.u=Uc(UC_ARCH_X86,UC_MODE_32);pe.load(self.u)
  self.u.mem_map(0,4096);self.u.mem_map(0x2000000,0x20000);self.u.mem_map(0x3000000,0x10000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']])
  self.put(A,VT);self.put(VT+12,SA,SB);self.put(VT+0xc8,POS)
  self.put(0xb36534,W);self.put(W+0x30,GRID,BH,OH);self.put(W+0xd8,c['phase']);self.put(W+0x130,MGR);self.put(MGR+4,c['presentation_target'])
  g=c['grid'];self.put(GRID+4,g['coordinate_shift'],g['cell_size']);self.put(GRID+0x1c,g['width'],g['height']);self.put(GRID+0x28,CELLS)
  for i,cell in enumerate(g['cells']):
   desc=DEFS+i*0x100;self.put(CELLS+i*0x30,desc);self.put(desc+0x1c,desc+0x40)
   self.put(desc+0x40+0x1c,cell['overlay_kind']);self.put(desc+0x40+0x68,cell['offset_min'],cell['offset_max'])
  self.set_actor(c['actor']);self.put(ARG,*c['candidate']);self.events=[];self.shape_count=0;self.executed={}
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,a,*values):self.u.mem_write(a,struct.pack('<'+'I'*len(values),*[v&0xffffffff for v in values]))
 def get(self,a,n=1):return list(struct.unpack('<'+'I'*n,self.u.mem_read(a,n*4)))
 def one(self,a):return self.get(a)[0]
 def args(self,n):return self.get(self.u.reg_read(UC_X86_REG_ESP)+4,n)
 def ret(self,n=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.one(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n);self.u.reg_write(UC_X86_REG_EAX,value)
 def actor(self):return {'flags':self.one(A+8),'position':self.get(A+0x20,3),'facing':self.get(A+0x2c,2)}
 def set_actor(self,a):
  self.put(A+8,a['flags']);self.put(A+0x20,*a['position']);self.put(A+0x2c,*a['facing'])
 def event(self,kind,**data):
  self.events.append({'kind':kind,'actor':self.actor(),**data})
  for m in self.c['mutations']:
   if m['event_index']==len(self.events)-1:
    a=self.actor();a.update(m.get('actor',{}));self.set_actor(a)
    if 'phase' in m:self.put(W+0xd8,m['phase'])
    if 'presentation_target' in m:self.put(MGR+4,m['presentation_target'])
 def desc(self,p):
  a=self.get(p,6);return {'shape':a[0],'facing':a[1:3],'position':a[3:5],'trailing_zero':a[5]}
 def hook(self,u,address,size,_):
  if 0x400000<=address<0xc00000 and address not in [0x550190,0x550210,0x5539b0,0x43b760]:self.executed[address]=size
  if address in [0x550190,0x550210]:
   layer='Base' if u.reg_read(UC_X86_REG_ECX)==BH else 'Overlay'
   method='Nearest' if address==0x550190 else 'Interpolated'
   self.event('height',layer=layer,method=method,position=self.args(2))
   pair=self.c['height_'+layer.lower()];self.put(VALUES,*pair)
   u.mem_write(RETURN,b'\xd9\x05'+struct.pack('<I',VALUES)+b'\xd8\x05'+struct.pack('<I',VALUES+4)+b'\xc2\x08\x00')
   u.reg_write(UC_X86_REG_EIP,RETURN)
  elif address in [SA,SB]:
   kind='A' if address==SA else 'B';value=self.c['shape_returns'][self.shape_count];self.shape_count+=1
   self.event('shape',shape_kind=kind,result=value);self.ret(value=value)
  elif address==0x5539b0:
   actor,ob,nb,oa,na=self.args(5);assert actor==A
   self.event('tiles',old_b=self.desc(ob),new_b=self.desc(nb),old_a=self.desc(oa),new_a=self.desc(na));self.ret(20)
  elif address==POS:self.event('position');self.ret()
  elif address==0x43b760:
   assert self.args(1)==[A]
   self.event('presentation',target=u.reg_read(UC_X86_REG_ECX));self.ret(4)
 def run(self):
  op=self.c['operation'];args={'query':[A+0x20,self.c['query_flags']],'move':self.c['candidate'][:2],'commit':[ARG],'snap':[ARG,self.c['grid']['cell_size']]}[op]
  entry={'query':0x4f0e50,'move':0x4eeb40,'commit':0x4ee5c0,'snap':0x46ee00}[op]
  # Capture returned ST0 through an explicit caller FSTP float32.
  if op=='query':self.u.mem_write(STOP,b'\xd9\x1d'+struct.pack('<I',VALUES+16)+b'\x90');end=STOP+6
  else:end=STOP
  self.put(STACK,STOP,*args);self.u.reg_write(UC_X86_REG_ESP,STACK)
  self.u.reg_write(UC_X86_REG_ECX,W if op=='query' else A)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(entry,end,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==end
  assert self.u.reg_read(UC_X86_REG_ESP)==STACK+4+4*len(args)
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  return {'actor':self.actor(),'phase':self.one(W+0xd8),'presentation_target':self.one(MGR+4),'events':self.events,
    'height_bits':self.one(VALUES+16) if op=='query' else None,'candidate':self.get(ARG,3),'abi_preserved':True}

def base(name,op):
 return {'name':name,'operation':op,'precision':'Double53','actor':{'flags':0,'position':words(6.25,6.5,99),'facing':words(1,0)},
 'candidate':words(12.75,17.125,-3),'grid':{'width':3,'height':3,'coordinate_shift':2,'cell_size':bits(4),'cells':[{'overlay_kind':0,'offset_min':bits(-.2),'offset_max':bits(.8)} for _ in range(9)]},
 'query_flags':0x122,'phase':1,'presentation_target':0x12345678,'height_base':words(2.25,2**-25),'height_overlay':words(-.125,2**-28),
 'shape_returns':[0x11220000+i for i in range(8)],'mutations':[]}
def cases():
 out=[]
 for flags in [0,1,2,3,5,7,0x10,0x11,0x12,0x13,0x15,0x122,0x123]:
  for mode in ['none','self1','self2','neighbor1','neighbor2']:
   c=base(f'query_flags_{flags:x}_{mode}','query');c['query_flags']=flags
   if mode!='none':c['grid']['cells'][4 if mode.startswith('self') else 0]['overlay_kind']=int(mode[-1])
   out.append(c)
 for i,point in enumerate([[-5.25,-2.9],[100.5,100.5],[-.9,4],[0,0],[11.99,11.99],[4,4]]):
  c=base(f'query_clamp_{i}','query');c['actor']['position'][:2]=words(*point);c['grid']['cells'][0]['overlay_kind']=1;out.append(c)
 for value in [-1,-.50001,-.5,-.49999,0,10]:
  c=base('query_floor_'+str(value),'query');c['query_flags']=3;c['grid']['cells'][4]['overlay_kind']=2;c['height_overlay']=words(value,0);out.append(c)
 c=base('height_mutates_query_point','query');c['grid']['cells'][0]['overlay_kind']=1;c['mutations']=[{'event_index':0,'actor':{'position':words(1,1,22)}}];out.append(c)
 for flags in [0,1<<16,1<<21,1<<26,(1<<16)|(1<<26),(1<<21)|(1<<12),(1<<21)|(1<<16)|(1<<26)]:
  c=base(f'move_flags_{flags:x}','move');c['actor']['flags']=flags;c['grid']['cells'][4]['overlay_kind']=1;out.append(c)
 for op in ['move','commit']:
  for phase,target in [(6,0x1234),(1,0),(1,0x1234)]:
   c=base(f'{op}_presentation_{phase}_{target}',''+op);c['phase']=phase;c['presentation_target']=target;out.append(c)
  c=base(op+'_all_shape_mutations',op);c['actor']['flags']=1<<26
  offset=2 if op=='move' else 0
  if op=='move':c['grid']['cells'][4]['overlay_kind']=1
  c['mutations']=[{'event_index':i+offset,'actor':{'position':words(20+i,30+i,40+i),'facing':words(.1+i,.2+i)}} for i in range(4)]
  out.append(c)
  c=base(op+'_position_suppresses_render',op);c['mutations']=[{'event_index':1 if op=='move' else 0,'phase':6,'actor':{'position':words(8,9,10)}}];out.append(c)
  c=base(op+'_position_enables_render',op);c['phase']=6;c['presentation_target']=0;c['mutations']=[{'event_index':1 if op=='move' else 0,'phase':1,'presentation_target':0x7abc}];out.append(c)
 c=base('move_height_changes_flags','move');c['mutations']=[{'event_index':0,'actor':{'flags':(1<<16)|(1<<26),'position':words(1,2,3)}}];out.append(c)
 c=base('commit_getter_clears_flags','commit');c['actor']['flags']=1<<26;c['mutations']=[{'event_index':0,'actor':{'flags':0}}];out.append(c)
 c=base('commit_tiles_changes_position','commit');c['actor']['flags']=1<<26;c['mutations']=[{'event_index':4,'actor':{'position':words(888,999,111),'facing':words(-1,0)}}];out.append(c)
 for cell in [0.1,0.2,0.3,1,3,4,7.25,16]:
  for i,xy in enumerate([[-7.51,9.49],[-.5,.5],[1.5,2.5],[-0.,0.],[0.10000001,123.4567]]):
   c=base(f'snap_{cell}_{i}','snap');c['grid']['cell_size']=bits(cell);c['candidate'][:2]=words(*xy);out.append(c)
 for cell,x in [(.1,-.925000011920929),(.3,-3.5250000953674316),(7.25,1.8125)]:
  c=base(f'snap_reciprocal_asymmetry_{cell}','snap');c['grid']['cell_size']=bits(cell);c['candidate'][:2]=words(x,x);out.append(c)
 rng=random.Random(0x4eeb40)
 for i in range(25):
  c=base(f'snap_random_{i}','snap');c['grid']['cell_size']=bits(rng.uniform(.1,32));c['candidate'][:2]=words(rng.uniform(-100,100),rng.uniform(-100,100));out.append(c)
 return out
def main():
 pe=Pe(ROOT.parent/'AA.exe',ORIGINAL_SHA);accepted=Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',ACCEPTED_SHA)
 ranges=json.loads((OUT/'native-ranges.json').read_text())
 for r in ranges['ranges']:
  a,z=int(r['start'],16),int(r['end'],16);b=pe.at(a,z-a);assert b==accepted.at(a,z-a);assert hashlib.sha256(b).hexdigest()==r['sha256']
 fixtures=[];executed={}
 for pc in ['Single24','Double53','Extended64']:
  for c in cases():
   c=copy.deepcopy(c);c['precision']=pc
   try:
    machine=Machine(pe,c);result=machine.run();executed.update(machine.executed)
   except Exception:print('FAIL',pc,c['name']);raise
   fixtures.append({'input':c,'output':result})
 runs=[]
 for address,size in sorted(executed.items()):
  if runs and address==runs[-1][1]:runs[-1][1]=address+size
  else:runs.append([address,address+size])
 traced=[]
 for a,z in runs:
  data=pe.at(a,z-a);assert data==accepted.at(a,z-a)
  traced.append({'start':hex(a),'end':hex(z),'sha256':hashlib.sha256(data).hexdigest()})
 (OUT/'native-executed-ranges.json').write_text(json.dumps({'passed':True,'instruction_count':len(executed),'ranges':traced},indent=2)+'\n')
 report={'schema':1,'passed':True,'original_sha256':ORIGINAL_SHA,'v14_sha256':ACCEPTED_SHA,'case_count':len(fixtures),
 'seams':['550190/550210 supplied height samples with native FADD and return ABI','actor virtual shape/position callbacks','5539B0 tile ownership','43B760 presentation leaf'],
 'fixtures':fixtures}
 (OUT/'native-spatial-fixtures.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'passed':True,'cases':len(fixtures)}))
if __name__=='__main__':main()
