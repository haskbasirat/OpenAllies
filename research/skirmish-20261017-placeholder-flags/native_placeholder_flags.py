"""Actual4BC5E0 +46D8F0 replacement containment, masks and partial stores.
Actual source rows execute standard ActorData A05540+3C->4C9FA0(FALSE).
Labelled alternate-vtable probes execute actual4F0500(TRUE), not a kind heuristic.
Diagnostics are named stopped/returning seams; native mask stores execute on
the labelled full-write probes. This does not execute all source completion.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def __init__(self,image,pc,c):
  super().__init__(b'',pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   for vs,r,size,off in image.sections:
    if size:self.u.mem_write(image.base+r,image.b[off:off+size])
  self.c=c;self.stopped=None;self.events=[]
  self.subject=self.alloc(0x5e0,0xa5);self.target=self.alloc(0x5e0,0xa5)
  for ptr,name,shape,flags in[(self.subject,'subject',c['subject_shape'],c['subject_flags']),(self.target,'replacement',c['target_shape'],c['target_flags'])]:
   self.w(ptr,0xa05540);self.w(ptr+8,self.cstring(name));self.w(ptr+0x1c,flags)
   self.u.mem_write(ptr+0x38,struct.pack('<6I',*shape))
  if c.get('virtual_true'):
   self.w(self.subject,self.vtable({0x3c:0x4f0500}))
  descriptor=self.alloc(0x10);self.w(descriptor+0xc,31);self.w(self.subject+0x18,descriptor)
  name=self.alloc(0xc);self.w(name+8,self.cstring('Placeholder'))
  array=self.alloc(32*4);self.w(array+31*4,name)
  manager=self.alloc(0x28);self.w(manager+0x24,array)
  game=self.alloc(0x3b8);self.w(game+0x3b4,manager);self.w(0xb36530,game)
  def fatal(m):self.events.append('containment_fatal');self.stopped='containment';m.outcome={'kind':'required_containment_diagnostic'};m.u.emu_stop()
  def warning(m):
   self.events.append('flag_warning')
   if c.get('stop_warning',True):self.stopped='warning';m.outcome={'kind':'required_warning_and_mutable_flags'};m.u.emu_stop()
   else:m.ret(0)
  self.seam(0x410c40,fatal);self.seam(0x404ed0,warning)
 def run(self):
  self.call(0x4bc5e0,0,[self.subject,self.target])
  return {'outcome':self.stopped or'complete','flags':self.r(self.subject+0x1c),'replacement_flags':self.r(self.target+0x1c),'events':self.events}
def shape(kind,x=1,y=1):
 f=h.f32
 return[0,0,0,0,0,0]if kind==0 else[1,f(x),0,0,0,0]if kind==1 else[kind,0x7fc12345,f(-x),f(-y),f(x),f(y)]
def cases():
 for a in range(4):
  for b in range(4):
   for scale in[0.5,1.0,2.0]:
    yield {'subject_shape':shape(a,1,2),'target_shape':shape(b,scale,scale*2),'subject_flags':0x20816500,'target_flags':0x20816500,'virtual_true':False,'stop_warning':True,'case':'shape'}
 for virtual in[False,True]:
  for original,target in[(0x20816500,0x20816700),(0xffffffff,0),(0,0xffffffff),(0x40000,0),(0,0x40000)]:
   for stop in[False,True]:yield {'subject_shape':shape(1),'target_shape':shape(1),'subject_flags':original,'target_flags':target,'virtual_true':virtual,'stop_warning':stop,'case':'mask_or_prefix'}
def main():
 rows=[]
 for c in cases():
  for pc in h.CW:
   out=[Machine(image,pc,c).run()for image in(h.P,h.V)];assert out[0]==out[1];rows.append({'pc':pc,'input':c,'native':out[0]})
 source=json.loads((HERE/'actual-source-census.json').read_text())
 actual=[]
 for p in source['precisions']:
  for placeholder in p['placeholders']:
   for nation in placeholder['rows']:
    target=nation['target'];assert target['kind']=='present'
    c={'subject_shape':placeholder['shape'],'target_shape':target['shape'],'subject_flags':placeholder['flags'],'target_flags':target['flags'],'virtual_true':False,'stop_warning':True,'case':'actual_source','placeholder':placeholder['id'],'nation':nation['nation'],'replacement':target['id']}
    out=[Machine(image,p['pc'],c).run()for image in(h.P,h.V)];assert out[0]==out[1]
    assert out[0]['outcome']=='complete'and out[0]['flags']==c['subject_flags']and out[0]['events']==[]
    actual.append({'pc':p['pc'],'input':c,'native':out[0]})
 seams={0x410c40:'stopped_containment_diagnostic',0x404ed0:'warning_service_or_stopped_required_entry'}
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'source_census_sha256':H((HERE/'actual-source-census.json').read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'rows':rows,'actual':actual,'machines':2*(len(rows)+len(actual)),'pins':[{'va':hex(a),'bytes':v,'kind':seams.get(a,'executed')}for a,v in sorted(h.VISITED.items())],'vtable_pins':[{'va':hex(a),'bytes':h.P.range(a,n).hex()}for a,n in[(0xa05540+0x38,8),(0x4c9fa0,3),(0x4f0500,6)]]}
 for p in doc['vtable_pins']:assert h.P.range(int(p['va'],16),len(bytes.fromhex(p['bytes'])))==h.V.range(int(p['va'],16),len(bytes.fromhex(p['bytes'])))
 path=HERE/'native-placeholder-flags.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'actual':len(actual),'machines':doc['machines'],'pins':len(h.VISITED),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
