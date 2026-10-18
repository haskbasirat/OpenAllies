"""Native shape setters/enums and scoped CRT precision setup, read-only."""
from pathlib import Path
import sys,json,struct,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *

def bits(f):return struct.unpack('<I',struct.pack('<f',f))[0]
def utf16(u,p):
 b=bytearray()
 while (c:=bytes(u.mem_read(p,2)))!=b'\0\0':b.extend(c);p+=2
 return b.decode('utf-16-le')
def enum(a,b,leaf):
 m=Machine();m.reset(0);m.u.reg_write(UC_X86_REG_EBX,0);m.u.reg_write(UC_X86_REG_ESI,m.STATE);rows=[]
 def hook(u,pc,n,user):
  if pc==leaf:
   sp=u.reg_read(UC_X86_REG_ESP);rows.append({'name':utf16(u,m.r(sp+8)),'value':m.r(sp+4)})
   u.reg_write(UC_X86_REG_EIP,m.r(sp));u.reg_write(UC_X86_REG_ESP,sp+16)
 m.u.hook_add(UC_HOOK_CODE,hook);m.u.emu_start(a,b,count=10000);assert m.u.reg_read(UC_X86_REG_EIP)==b
 return rows

def main():
 rows=[];fpu=[];rng=random.Random(0x46e0e0)
 dimensions=[(0.,0.),(1.,1.),(4.,8.),(-4.,8.),(0.,2.),(0.2,0.5),(1e-20,1e-20),(1e8,1.)]+[(rng.uniform(.001,1000),rng.uniform(.001,1000))for _ in range(32)]
 for precision,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  m=Machine(cw)
  for kind in range(4):
   for x,y in dimensions:
    m.reset(0);m.call(0x46e060)
    m.w(m.STACK,m.STOP);m.u.reg_write(UC_X86_REG_ESP,m.STACK);m.u.reg_write(UC_X86_REG_ECX,m.STATE)
    args=[]if kind==0 else [bits(x)]if kind in(1,2)else[bits(x),bits(y)]
    for i,v in enumerate(args):m.w(m.STACK+4+i*4,v)
    m.u.emu_start([0x46d8d0,0x46d8e0,0x46eb60,0x46e0e0][kind],m.STOP,count=1000)
    assert m.u.reg_read(UC_X86_REG_EIP)==m.STOP and m.u.reg_read(UC_X86_REG_ESP)==m.STACK+4+len(args)*4
    rows.append({'precision':precision,'selector':kind,'arguments':args,'words':list(struct.unpack('<6I',m.u.mem_read(m.STATE,24)))})
  m.reset(0);m.call(0x972c6b);fpu.append({'initial_control_word':cw,'after_default_precision':m.u.reg_read(UC_X86_REG_FPCW)})
 ranges={'shape_zero_ctor':(0x46e060,0x46e088),'point':(0x46d8d0,0x46d8df),'circle':(0x46d8e0,0x46d8f4),'square':(0x46eb60,0x46eb79),'rectangle':(0x46e0e0,0x46e147),'shape_enum':(0x46f5f6,0x46f631),'turn_enum':(0x463136,0x463162),'object_shape_loading':(0x49dc40,0x49e0a3),'shape_constructor_calls':(0x49c760,0x49ca90),'default_precision':(0x972c6b,0x972c7e),'controlfp':(0x96f658,0x96f7c0)}
 for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a),hex(a)
 report={'schema':1,'passed':True,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'shape_cases':rows,'shape_enum':enum(0x46f5f6,0x46f631,0x46f440),'turn_enum':enum(0x463136,0x463162,0x461cf0),'crt_default_precision':fpu,'native_ranges':{n:{'start':hex(a),'end':hex(b),'sha256':sha(original.range(a,b-a))}for n,(a,b)in ranges.items()},'scope':['Original shape zero constructor/setters execute unchanged with finite dimensions in all3precision modes; actual enum registration callarguments captured at insertion leaf.','ObjectData49C760 calls zero constructor for all3shape slots;49DC40 applies setters, softnull retains blockingkind andreadssoftdimensions;targetnull copies fullsoftshape. Loader branch is statically pinned here, not fully executed yet.','CRT default precision routine executes whole; this does not establish D3D/game-loop FPU lifecycle.']}
 (HERE/'native-shapes.json').write_text(json.dumps(report,indent=2));print(json.dumps({'cases':len(rows),'enums':report['shape_enum'],'turn':report['turn_enum'],'fpu':fpu}))
if __name__=='__main__':main()
