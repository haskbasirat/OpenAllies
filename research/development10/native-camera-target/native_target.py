"""Native6FF130→6FEDB0 bounds/ground-height/zoom producer, original+v14/allPC.
Camera/shape/descriptor words are explicit finite laboratory current inputs,
not a completed7016A0 constructor. Real46E0E0/46EF30/46E4F0/6FE670/6FE470
execute. Same-world scalar reads and550210/6FE050 children have declared
observation/error seams; final NiCamera update remains a required integration.
FSIN is transported from THIS CPU's isolated trusted x87 helper, not Unicorn.
"""
from pathlib import Path
import sys,json,itertools,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-native-camera'));import native_camera_reset as R
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
class Machine(R.Machine):
 def _hook(self,u,pc,size,data):
  if R.original.base<=pc<R.original.base+R.original.image_size:
   op=bytes(u.mem_read(pc,size))
   if op==b'\xd9\xfe':
    v=self.st0_live();raw=struct.pack('<QH',v['extended_significand'],v['extended_sign_exponent']);out=self.hardware.trig('fsin',R.PRECISIONS[self.precision],raw)
    assert out['status_word']&(1|4|8|0x400)==0
    self.trans.append(dict(pc=hex(pc),op='fsin',input80=raw.hex(),output80=out['raw80'],control_word=out['control_word']))
    p=self.alloc(16);u.mem_write(p,bytes.fromhex(out['raw80']));code=b'\xdd\xd8'+b'\xdb\x2d'+struct.pack('<I',p)+b'\x68'+struct.pack('<I',pc+size)+b'\xc3';stub=self.alloc(len(code));u.mem_write(stub,code);u.reg_write(UC_X86_REG_EIP,stub);return
  super()._hook(u,pc,size,data)
def cases():
 base=dict(entry='target',target=[R.f32(40),R.f32(60)],position=[R.f32(25),R.f32(35),R.f32(50)],look=[R.f32(30),R.f32(40),R.f32(10)],disabled=0,tracking=1,
  dimensions=[256,128],fallback=[300,500],margin=20,initial_rectangle=[R.f32(100),R.f32(100)],mode2c0=0,editor=0,session=2,height=R.f32(17.5),fail=None)
 yield base
 for tracking,disabled in itertools.product((0,1,128),(0,1,128)):yield dict(base,tracking=tracking,disabled=disabled)
 for target in[[-200,-300],[1000,2000],[0,0],[128,64],[246,118],[11,11],[1.0000001,65535.996]]:yield dict(base,target=list(map(R.f32,target)))
 for dims in[None,[255,129],[1,1],[1024,1024]]:yield dict(base,dimensions=dims)
 yield dict(base,initial_rectangle=[R.f32(236),R.f32(25)]) #Width equality must preserve OLD height.
 for editor,session,mode in itertools.product((0,1),(2,3),(0,1)):yield dict(base,editor=editor,session=session,mode2c0=mode)
 yield dict(base,entry='position',target=list(map(R.f32,[-20,90,42])))
 yield dict(base,entry='position',target=list(map(R.f32,[8192.125,-0.000123,100.25])),tracking=0)
 for position,look,target in[
  ([0.,-0.,0.],[-0.,0.,-0.],[0.,-0.]),
  ([16777216.,-16777216.,42.125],[16777218.,-16777218.,.125],[16777220.,-16777220.]),
  ([1.000000119,-.0000000123,10.125],[.0000000123,1.000000119,-10.125],[.999999881,-.999999881]),
 ]:yield dict(base,tracking=0,position=list(map(R.f32,position)),look=list(map(R.f32,look)),target=list(map(R.f32,target)))
 for fail in range(1,7):yield dict(base,mode2c0=1,fail=fail)
def run(image,pc,c,hw):
 m=Machine(image,pc,hw);m.fail=c['fail'];camera=m.alloc(0x394);m.u.mem_write(camera,b'\xa5'*0x394)
 for off,word in [(0x208,R.f32(.9)),(0x20c,R.f32(.7)),(0x210,R.f32(2.1)),(0x2b8,R.f32(.6)),(0x2c0,c['mode2c0'])]:m.w(camera+off,word)
 m.wb(camera+0x214,c['disabled']);m.wb(camera+0x33c,c['tracking'])
 for off,values in[(0x240,c['position']),(0x24c,c['look']),(0x114,list(map(R.f32,[.2,.4,-.8])))]:
  for i,v in enumerate(values):m.w(camera+off+4*i,v)
 m.call(0x46e0e0,camera+0x258,c['initial_rectangle']);m.call(0x46e060,camera+0x288)
 m.w(camera+0x270,camera+0x258);m.w(camera+0x274,0);m.w(camera+0x278,R.f32(-1));m.w(camera+0x27c,R.f32(128));m.w(camera+0x280,R.f32(64));m.w(camera+0x284,0)
 m.w(camera+0x2a0,0)
 world=m.alloc(0x300);m.w(0xb36534,world);m.w(world+0x34,m.alloc(4));m.wb(world+0x275,c['editor'])
 session=m.alloc(0x110);m.w(0xb36550,session);m.w(session+0xe4,c['session'])
 gmap=0
 if c['dimensions']is not None:gmap=m.alloc(0x80);m.w(gmap+0x74,c['dimensions'][0]);m.w(gmap+0x78,c['dimensions'][1])
 m.w(0xb36518,gmap);uvars=m.alloc(0x100);m.w(0xb39940,uvars);margin=m.alloc(4);m.w(margin,c['margin']);m.w(uvars+0x24,margin)
 for off,word in zip((0xa8,0xac),c['fallback']):p=m.alloc(4);m.w(p,word);m.w(uvars+off,p)
 initial=[m.r(camera+4*i)for i in range(0x394//4)]
 def dimensions(x):x.event('dimensions',c['dimensions'])
 m.observe(0x6fee4e,dimensions);m.observe(0x6fee69,lambda x:x.event('fallback',c['fallback']))
 def mem(u,access,a,size,value,data):
  if a==margin and size==4:m.event('margin',c['margin'])
  if a==world+0x275 and size==1:m.event('editor',c['editor']!=0)
  if a==session+0xe4 and size==4:m.event('session',c['session'])
 m.u.hook_add(UC_HOOK_MEM_READ,mem)
 def height(x):xy=[x.arg(1),x.arg(2)];x.event('height',xy);x.ret_st0_f32(c['height'],8)
 m.seam(0x550210,height);m.seam(0x6fe050,lambda x:(x.event('update'),x.ret(0)))
 error=None
 try:m.call(0x6ff130 if c['entry']=='target' else 0x6fedb0,camera,c['target'])
 except R.Cutoff:error='required dependency cutoff'
 assert not m.null_accesses,m.null_accesses
 def words():return[m.r(camera+4*i)if i*4 not in(0x270,0x2a0)else None for i in range(0x394//4)]
 output=dict(words=words(),rectangle_alias=m.r(camera+0x270)==camera+0x258,point_alias=m.r(camera+0x2a0)==camera+0x288,trace=m.trace,error=error)
 initial[0x270//4]=None;initial[0x2a0//4]=None
 return dict(initial=initial,output=output),m.actual,m.trans
def main():
 hw=R.Hardware();rows=[];pins={};trans=[]
 try:
  for pc,c in itertools.product(R.PRECISIONS,cases()):
   outputs=[]
   for image in('original','v14'):
    output,visited,transport=run(image,pc,c,hw);outputs.append(output);pins.update(visited);trans.extend(transport)
   assert outputs[0]==outputs[1];rows.append(dict(precision=pc,input=c,**outputs[0]))
  #An observer that throws has NOT executed its current instruction. Exclude
  #all such possible entry PCs conservatively, including successful visits.
  excluded={0x6fee4e,0x6fee69,0x6fee84,0x6fe689,0x6fe699};pins={a:b for a,b in pins.items()if a not in excluded}
  for a,b in pins.items():assert R.original.range(a,len(bytes.fromhex(b))).hex()==R.modified.range(a,len(bytes.fromhex(b))).hex()==b
  doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__)),rows=rows,pins={hex(a):b for a,b in sorted(pins.items())},hardware_shims=hw.instructions,hardware_transports=trans,
   seams=['550210 current height child','6FE050 sameCamera update child','typed finite laboratory current words/links'],observed_cutoff_entries=[hex(a)for a in sorted(excluded)])
  data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-target.json'
  if '--write'in sys.argv:p.write_bytes(data)
  else:assert p.read_bytes()==data
  print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),hardware_transports=len(trans),sha256=hashlib.sha256(data).hexdigest())))
 finally:hw.close()
if __name__=='__main__':main()
