"""Full7AA0B0 SAI map constructor; only heap allocation is intercepted."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
import original_rng as base
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
ORIGINAL=base.original;V14=base.modified
def run(precision,poison,image):
 prior=base.original;base.original=ORIGINAL if image=='original'else V14
 try:m=base.Machine({24:0x7f,53:0x27f,64:0x37f}[precision])
 finally:base.original=prior
 m.u.mem_map(0,0x1000);m.w(0,0);m.reset(0)
 m.u.mem_write(m.STATE,bytes([poison])*0x44)
 visited={};leaves={};alloc=[];cursor=0x2002000
 def guard(u,access,address,size,value,user):
  if address<0x1000:
   ip=u.reg_read(UC_X86_REG_EIP);assert bytes(u.mem_read(ip,1))==b'\x64',('nonFS null access',hex(ip),address)
 def hook(u,pc,size,user):
  nonlocal cursor
  if ORIGINAL.base<=pc<ORIGINAL.base+ORIGINAL.image_size:
   raw=bytes(u.mem_read(pc,size));assert raw==ORIGINAL.range(pc,size)==V14.range(pc,size)
   (leaves if pc==0x6f7810 else visited)[pc]=raw.hex()
  if pc==0x6f7810:
   sp=u.reg_read(UC_X86_REG_ESP);n=m.r(sp+4);alloc.append(n)
   ptr=cursor;cursor+=(n+15)&~15
   u.reg_write(UC_X86_REG_EAX,ptr);u.reg_write(UC_X86_REG_EIP,m.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4)
 m.u.hook_add(UC_HOOK_CODE,hook);m.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,guard)
 m.u.emu_start(0x7aa0b0,m.STOP,count=10000)
 assert m.u.reg_read(UC_X86_REG_EIP)==m.STOP and m.u.reg_read(UC_X86_REG_ESP)==m.STACK+4
 assert m.u.reg_read(UC_X86_REG_EAX)==m.STATE and alloc==[0x400,0x4000,0x4000]
 assert m.r(0xb3652c)==m.STATE and m.r(0xb40da4)==m.STATE and m.r(0xb40da0)==0xb3652c
 output=dict(analysed=bool(m.u.mem_read(m.STATE+4,1)[0]),dynamic=bool(m.u.mem_read(m.STATE+5,1)[0]),
  counts={hex(off):m.r(m.STATE+off)for off in [0xc,0x18,0x24]},primary_island=m.r(m.STATE+0x2c),diagonal_bits=m.r(m.STATE+0x30),
  list34=[m.r(m.STATE+0x34),m.r(m.STATE+0x38)],byte3c=int(m.u.mem_read(m.STATE+0x3c,1)[0]),route_cache=m.r(m.STATE+0x40))
 assert not output['analysed'] and not output['dynamic'] and output['route_cache']==0
 return output,visited,leaves
def main():
 rows=[];executed={};leaves={}
 for pc in [24,53,64]:
  for poison in [0,0xcd,0xff]:
   a,av,al=run(pc,poison,'original');b,bv,bl=run(pc,poison,'v14');assert a==b
   executed.update(av|bv);leaves.update(al|bl);rows.append(dict(precision=pc,poison=poison,output=a))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(ORIGINAL.b),v14_sha256=H(V14.b),rows=rows,machines=18,
  executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(executed.items())],intercepted_entries=[dict(va=hex(a),bytes=b)for a,b in sorted(leaves.items())],
  dependency_sha256=H((ROOT/'research/phase4-rng/original_rng.py').read_bytes()))
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-map-constructor.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=9,machines=18,executed_pins=len(executed),intercepted_entries=len(leaves),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
