"""Native fresh751770 -> localfirstplayer -> appendself/759790/757010.
Projected Session/Sim and named OS/allocator/string seams inherited explicitly.
Clock75FBF0 is a recorded argument seam; its separate native kernel is proved
in skirmish-20261016-server-clock. This producer does not certify full Host.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,NODE,NETNODE,SYNC_A,proof_ranges
from native_lobby import WRITER_VT,s32
from unicorn.x86_const import *
class HostMachine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'bootstrap_ready',False):
   if pc==0x75fbf0:
    self.native_events.append({'kind':'server_clock_restart','period':self.arg(),'start':self.arg(1)})
    self.ret(8);return
   if pc in (0x751770,0x7514f0,0x759790,0x74f1a0,0x759680,0x757010,0x74dbc0,0x77d900):
    self.native_events.append({'kind':'native_call','address':f'{pc:08X}'})
   if pc==0x757250:
    self.native_events.append({'kind':'required_backup_mode','enabled':self.arg()});self.u.emu_stop();return
  super().hook(u,pc,size,user)
 def setup(self,rate,behind):
  self.begin(0x12345678,list(range(16)),0)
  # Native constructor initialized history objects and output streams. Their
  # capacities are fixture-sized successful allocations, not max/caps claims.
  for off in (0x1f8,0x254,0x270):
   x=self.stream(WRITER_VT,size=64);self.u.mem_write(NODE+off,bytes(self.u.mem_read(x,16)))
  # Unlike own/decree histories,751770 leaves224 unchanged; execute its
  # actual75CE80 constructor dependency instead of inheriting NodeMachine0.
  self.call(0x74b790,[0x100],this=NODE+0x224,callee_pop=4)
  self.w(NODE+0x64,0);self.w(NODE+0x68,0)
  self.call(0x751770,this=NODE)
  self.after_reset=self.state()
  self.first_player('Local Host','fixture-key')
  # Resume the same captured Host frame through real append-self, SetServer,
  # forced-null host, SetHost and ResetSyncA, stopping before mode assignment.
  table=self.alloc(0x40);cell=self.alloc(4);self.w(cell,rate);self.w(table+0x20,cell);self.w(0xb39ab0,table)
  self.w(NETNODE+0x58,0x3f800000);self.b(NETNODE+0x64,behind)
  self.native_events=[]
  self.u.emu_start(0x75b1b1,0x75b20e,count=500000)
  pc=self.u.reg_read(UC_X86_REG_EIP)
  assert pc in (0x75b20e,0x757250),hex(pc)
  return pc
 def state(self):
  def hist(off): return {'begin':s32(self.r(NODE+off)),'end':s32(self.r(NODE+off+4))}
  return {'requests':hist(0xc),'backup_requests':hist(0x1c),'decrees':hist(0x214),'backup_decrees':hist(0x224),
    'client_next':s32(self.r(NODE+0x20c)),'last_request_player':self.r(NODE+0x210),'ordering_player':self.r(NODE+0x284),
    'host_is_node':self.r(NODE+0x2c4)==NODE,'host_is_null':self.r(NODE+0x2c4)==0,'server_is_node':self.r(NODE+0x2c8)==NODE,
    'clock_selected':self.r(NODE+0x158)==NODE+0x15c,'clock_null':self.r(NODE+0x158)==0,'backup_mode':self.rb(NODE+0x2d4),
    'backup_since':self.r(NODE+0x2d8),'vote_count':s32(self.r(NODE+0x2b8)),'max_patience':self.r(NODE+0x2bc),
    'own_patience':self.r(NODE+0x44),'order_version':self.rb(NODE+0x6c),'peers_changed':self.rb(NODE+0x2b4),
    'own_flags':[self.rb(NODE+o)for o in (0x30,0x31,0x48,0x49,0x4a,0x4b)],'own_rtt':self.r(NODE+0x4c),'own_received_fraction':self.r(NODE+0x50),
    'sync_a_hash':s32(self.r(SYNC_A+8)),'sync_a_begin':s32(self.r(SYNC_A+0x10)),'sync_a_end':s32(self.r(SYNC_A+0x14))}
def main():
 rows=[];visited={}
 for image in ('original','v14'):
  for p in PC:
   for rate,behind in [(0x41800000,0),(0x40400000,0),(0x3f800001,0),(0x41800000,1)]:
    print('host',image,p,hex(rate),behind,flush=True)
    m=HostMachine(image);m.fpu(p);pc=m.setup(rate,behind)
    rows.append({'image':image,'precision':p,'rate':rate,'behind':behind,'reset':m.after_reset,'cutoff':f'{pc:08X}','after':m.state(),'events':m.native_events})
    visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'Whole freshNode751770, native appendself/SetServer/SetHost/ResetSyncA local prefix, before launchmode assignment; positivebackup stops at first757250 boundary','producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-local-host.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'host rows')
if __name__=='__main__':main()

