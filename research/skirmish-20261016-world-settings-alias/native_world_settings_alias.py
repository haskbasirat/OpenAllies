"""Bounded native pointer-flow proof for current GWorld/MapResource settings.
Executes5A2E0B creator-call setup -> actual base WorldCreator5126C0 -> Sim76AA90
-> GWorld4F7D00 prefix, stopping immediately after the real global-this and+28
stores. Logger, allocation and WorldParameters constructor are explicit seams;
the remaining GWorld constructor and whole MapResource load do not execute.
Separate unchanged caller/read fragments then prove5A302A->4FC600 receives the
same global World and MapResource, and4F3BC9 reads current global settings+A8.
No complete constructor, load, native first-map capture or arbitrary-callback
immutability is claimed by these isolated fragments.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P;H=lambda b:hashlib.sha256(b).hexdigest()
def run(c,image):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(c['precision'])
 resource=m.alloc(0xb4);creator=m.alloc(0x80);sim=m.alloc(0x100);world=m.alloc(0x29c);otherworld=m.alloc(0x29c);otherresource=m.alloc(0xb4)
 m.w32(resource+0x88,creator);m.w32(resource+0xa8,c['initial_mode']);m.w32(creator,0xa3214c);m.w32(creator+0x4c,c['rng_seed']);m.w32(0xb36554,sim)
 m.w32(0xb36534,otherworld);m.w32(otherworld+0x28,otherresource);m.w32(otherresource+0xa8,c['foreign_mode'])
 calls=[]
 def logger(_): calls.append('logger');m.ret(0,0)
 def allocate(_): assert m.arg()==0x29c;calls.append('allocate29C');m.ret(0,world)
 def parameters(_):calls.append('WorldParametersConstructor');m.ret(0,m.u.reg_read(UC_X86_REG_ECX))
 m.seams[0x404f60]=logger;m.seams[0x6f7810]=allocate;m.seams[0x511df0]=parameters
 m.w32(h.STACK,h.STOP);m.u.reg_write(UC_X86_REG_ESP,h.STACK);m.u.reg_write(UC_X86_REG_ESI,resource)
 m.u.emu_start(0x5a2e0b,0x4f7d63,count=100000)
 assert m.u.reg_read(UC_X86_REG_EIP)==0x4f7d63;assert m.r32(0xb36534)==world;assert m.r32(world+0x28)==resource
 assert m.r32(0xb3cc34)==world;assert m.r32(0xb3cc30)==0xb36534
 assert calls==['logger','logger','allocate29C','WorldParametersConstructor'],calls
 m.w32(h.STACK,h.STOP);m.u.reg_write(UC_X86_REG_ESP,h.STACK);m.u.reg_write(UC_X86_REG_ESI,resource)
 m.u.emu_start(0x5a302a,0x4fc600,count=100)
 assert m.u.reg_read(UC_X86_REG_ECX)==world;assert m.arg()==resource
 def mode():
  m.u.emu_start(0x4f3bc9,0x4f3bd8,count=100);return m.u.reg_read(UC_X86_REG_ECX)
 first=mode();assert first==c['initial_mode'];m.w32(resource+0xa8,c['later_mode']);second=mode();assert second==c['later_mode']
 m.w32(0xb36534,otherworld);foreign=mode();assert foreign==c['foreign_mode'];assert m.r32(world+0x28)==resource
 assert m.fpu_top()==0
 return dict(ctor_global_is_new_world=True,ctor_settings_is_mapresource=True,load_uses_same_world_and_resource=True,mode_reads=[first,second],foreign_global_reads_foreign_mode=foreign,constructor_stopped_at='0x4f7d63',world_load_stopped_at='0x4fc600')
def main():
 rows=[]
 for pc in h.PRECISION:
  for i,(a,b)in enumerate(((1,2),(1,0),(0,1),(4,0xffffffff))):
   c=dict(precision=pc,initial_mode=a,later_mode=b,foreign_mode=0xA500+i,rng_seed=0x123400+i)
   x=run(c,'original');y=run(c,'v14');assert x==y;rows.append(dict(input=c,output=x))
 raw=ORIGINAL.range(0xa32170,4);assert raw==h.V.range(0xa32170,4)
 result=dict(schema=1,scope=__doc__,original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),cases=rows,native_executions=len(rows)*2,data_pins=[dict(address='0xa32170',bytes=raw.hex())],instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 p=HERE/'native-world-settings-alias.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=1)+'\n')
 else:assert json.loads(p.read_text())==result
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=H(p.read_bytes()))))
if __name__=='__main__':main()
