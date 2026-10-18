"""Real49C760 aspect-field constructor slice and49DC40 View aspect subsequence.
Real49C670 constructor/49C360 append run;46A8A0 is an explicit Aspect resource
loader seam populated from labelled typed inputs, not a completed model loader.
Successful CString/heap transports are named. Stops preserve prior source stores.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def __init__(self,image,pc,c):
  super().__init__(b'',pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   for vs,r,size,off in image.sections:
    if size:self.u.mem_write(image.base+r,image.b[off:off+size])
  self.c=c;self.stop=None;self.trace=[];self.next_other=0
  self.d=self.alloc(0x180,0xa5);self.view=self.alloc(4);self.nodes={}
  self.w(self.d+8,self.cstring('definition'));self.w(self.d+0xb0,self.cstring('prefix'))
  for key,v in [('normal',c.get('normal'))]+[(str(i),a)for i,a in enumerate(c['others'])]:
   if v is not None:
    p=self.alloc(4);self.nodes[p]=(key,v)
  self.normal=next((p for p,(key,_)in self.nodes.items()if key=='normal'),0)
  self.others=[p for p,(key,_)in self.nodes.items()if key!='normal']
  self.seam(0x5b5380,lambda m:m.ret(4,int(c['variations'])))
  self.seam(0x5b4410,lambda m:m.ret(8,self.normal))
  self.seam(0x5b4220,lambda m:m.ret(4,len(self.others)))
  def item(m):
   p=self.others[self.next_other];self.next_other+=1;m.ret(0,p)
  self.seam(0x42be10,item)
  def ids(m):
   key,v=self.nodes[m.u.reg_read(UC_X86_REG_ECX)];p=m.alloc(4);m.w(p,m.cstring(v['id']));m.ret(4,p)
  self.seam(0x5b54d0,ids)
  def resource(m):
   key,v=self.nodes[m.arg(0)];self.trace.append(key)
   if c.get('stop_resource')==key:self.stop='required_resource_'+key;m.u.emu_stop();return
   obj=m.u.reg_read(UC_X86_REG_ECX);m.w(obj+0x10,v['kind']);m.w(obj+0x2c,v['weight']);m.ret(8)
  self.seam(0x46a8a0,resource)
  self.seam(0x410c40,lambda m:self.required('native_data_error'))
  self.seam(0x96cb83,lambda m:m.ret(0,0 if m.read_wide(m.arg(0))==m.read_wide(m.arg(1))else 1))
 def required(self,reason):self.stop=reason;self.u.emu_stop()
 def run(self):
  self.u.reg_write(UC_X86_REG_ESI,self.d);self.u.reg_write(UC_X86_REG_EBX,0);self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.emu_start(0x49c887,0x49c8ea,count=100000)
  before={'count':self.r(self.d+0x110),'default':self.r(self.d+0x118),'construction_count':self.r(self.d+0x120)}
  self.u.reg_write(UC_X86_REG_EBP,self.d);self.u.reg_write(UC_X86_REG_ESI,self.view);self.u.reg_write(UC_X86_REG_EBX,0);self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.emu_start(0x49e309,0x49e3a3,count=1000000)
  if not self.stop:assert self.u.reg_read(UC_X86_REG_EIP)==0x49e3a3
  count=self.r(self.d+0x110);base=self.r(self.d+0x10c)
  return {'constructor':before,'outcome':self.stop or'complete','variations':self.b(self.d+0x108),'count':count,'default':self.r(self.d+0x118),'total':self.r(self.d+0x128),'aspects':[self.read_wide(self.r(self.r(base+i*4)+0xc))for i in range(count)],'construction':[self.r(self.r(self.d+0x11c)+i*4)for i in range(self.r(self.d+0x120))],'resource_trace':self.trace}
def aspect(id,kind=1,weight=0x3f800000):return {'id':id,'kind':kind,'weight':weight}
def cases():
 for variations in [False,True]:
  for count in [0,1,3]:yield {'variations':variations,'normal':aspect('normal'),'others':[aspect('other'+str(i),i%2,0x3eaaaaab)for i in range(count)]}
 yield {'variations':False,'normal':None,'others':[]}
 yield {'variations':False,'normal':aspect('normal'),'others':[aspect('normal')]}
 yield {'variations':False,'normal':aspect('normal'),'others':[aspect('first'),aspect('first')]}
 yield {'variations':False,'normal':aspect('normal'),'others':[],'stop_resource':'normal'}
 yield {'variations':False,'normal':aspect('normal'),'others':[aspect('first'),aspect('second')],'stop_resource':'1'}
 yield {'variations':True,'normal':None,'others':[aspect('first'),aspect('second')],'stop_resource':'1'}
def main():
 rows=[]
 for c in cases():
  for pc in h.CW:
   out=[Machine(image,pc,c).run()for image in(h.P,h.V)];assert out[0]==out[1];rows.append({'input':c,'pc':pc,'native':out[0]})
 seams={0x6f7810:'heap_transport',0x6f7430:'heap_transport',0x856370:'CString_allocator_transport',0x401b00:'CString_literal_transport',0x856560:'CString_literal_transport',0x5b5380:'typed_bool_getter_seam',0x5b4410:'actual_child_getter_seam',0x5b4220:'actual_list_count_seam',0x42be10:'actual_list_element_seam',0x5b54d0:'typed_ids_getter_seam',0x46a8a0:'required_Aspect_resource_loader_seam',0x410c40:'stopped_native_diagnostic_entry',0x96cb83:'CRT_wide_comparison_transport'}
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'rows':rows,'machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':b,'kind':seams.get(a,'executed')}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-object-aspect-source.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
