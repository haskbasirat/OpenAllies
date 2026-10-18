"""Loading return4/Game node and UserReady/prompt bounded original/v14 proof.
Actual608C60/40CA10,40CE10/40CAA0/608C80/40CA80 and6E80F0/40D970 execute.
Native diagnostics/show/interface-update/status/CString transport are explicit
children. This does not execute full Game HUD, queue completion or unfreeze.
"""
from pathlib import Path
import sys,struct,itertools,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P;V14=h.V
class Cutoff(Exception):pass
class M(h.Machine):
 def run(self,c):
  self.trace=[];app=0xb39690;loading=self.alloc(0x18);game=self.alloc(8);menu=0x12345678
  self.u.mem_write(loading,b'\xa5'*0x18);self.u.mem_write(game,b'\xa5'*8)
  self.w32(0xb3974c,menu);self.call(0x682de0,loading);self.call(0x608c60,game)
  self.w32(app+0x10c,6);self.w32(app+0x110,99);self.w32(app+0xe4,loading);self.w32(app+0xdc,game)
  error=None
  if c['op']=='ctor':return dict(game=list(self.u.mem_read(game,8)))
  if c['op']=='ready':
   self.w8(loading+0x16,c['old']);self.call(0x6e80f0,0,[0x98765432]);return dict(ready=self.r8(loading+0x16))
  if c['op']=='transition':
   self.w8(loading+8,1)
   def diagnostic(m):m.trace.append('diagnostic');m.stop('diagnostic',c);m.ret()
   def show(m):m.trace.append(['show',m.ecx(),m.arg(0)]);m.stop('show',c);m.ret(4)
   self.seams[0x404f60]=diagnostic;self.seams[0x5c0440]=show
   try:self.call(0x40ce10,app,[4,0,0])
   except Cutoff:error=c['fail']
   return dict(current=self.r32(app+0x10c),previous=self.r32(app+0x110),loading_changed=self.r8(loading+8),game_menu=self.r32(game+4),trace=self.trace,error=error)
  assert c['op']=='prompt';ui=self.alloc(0x104);button=self.alloc(0x28);self.w32(ui+0x100,button);self.w32(button+0x24,c['visible']);self.w8(loading+0x17,c['complete']);self.w8(loading+0x16,c['ready'])
  self.seams[0x5c06b0]=lambda m:(m.trace.append('base_update'),m.ret())
  def string(m):p=m.alloc(32);m.w32(p+12,3);m.w32(m.ecx(),p+16);m.trace.append('string');m.ret(4,m.ecx())
  def status(m):m.trace.append('finished_status');m.stop('status',c);m.ret(12)
  def show_ready(m):m.trace.append(['show_ready',m.arg(0)]);m.w32(button+0x24,m.r32(button+0x24)|1);m.ret(4)
  self.seams[0x401b00]=string;self.seams[0x773ce0]=status;self.seams[0x5efb00]=show_ready
  def end(m):raise Cutoff()
  self.seams[0x6e8ca9]=end
  try:self.call(0x6e8c00,ui)
  except Cutoff:pass
  return dict(visible=self.r32(button+0x24),trace=self.trace)
 def stop(self,label,c):
  if c.get('fail')==label:raise Cutoff()
def main():
 h.VISITED.clear();rows=[]
 cases=[dict(op='ctor')]+[dict(op='ready',old=x)for x in [0,1,128,255]]+[dict(op='transition',fail=x)for x in [None,'diagnostic','show']]+[dict(op='prompt',visible=v,complete=c,ready=r,fail=f)for v,c,r,f in itertools.product([0,1],[0,1],[0,1],[None,'status'])]
 for pc,c in itertools.product(h.PRECISION,cases):
  h.P=ORIGINAL;a=M(pc).run(c);h.P=V14;b=M(pc).run(c);assert a==b;rows.append(dict(pc=pc,input=c,output=a))
 h.P=ORIGINAL;seams=[0x404f60,0x5c0440,0x5c06b0,0x401b00,0x773ce0,0x5efb00,0x6e8ca9]
 pins={hex(a):b for a,b in sorted(h.VISITED.items())if a not in seams}
 for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,pins=pins,excluded_entries=[hex(x)for x in seams],node4_vtable=dict(address='0xa56c00',words=[hex(ORIGINAL.u32(0xa56c00+i))for i in range(0,0x2c,4)]))
 p=HERE/'native-loading-return.json';raw=(json.dumps(doc,indent=2)+'\n').encode()
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(pairs=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
