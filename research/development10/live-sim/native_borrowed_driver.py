"""Fresh75CA70/755290/76B780 frames on both native images/all three PCs.
Uses the frozen tick oracle's recorded bounded network/presentation seams.
Named seam failures retain real instruction-prefix writes. A host-record
callback mutation of the same +28C is explicit scripted intervention, proving
that the following native add reads the live owner rather than a snapshot.
No real network/mission/application readiness is asserted.
"""
from pathlib import Path
import json,sys,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-tick'))
import native_tick_driver as h
from unicorn.x86_const import UC_X86_REG_ECX
ORIGINAL,V14=h.NATIVE,h.ACCEPTED
PINS={}
class Stop(Exception):pass
class Machine(h.Machine):
    def __init__(self,c,pc):
        self.c=c;self.failed=None
        super().__init__(pc)
        self.set_gates(dict(session_f4=1,session_mode=2,flags_b396c4=0,single_player_auto_pause=0,avi=0,avi_frames_per_tick=0))
        self.set_sim(dict(begin_game=h.bits(2),begin_admin=h.bits(5),end_game=h.bits(3),end_admin=h.bits(6),speed=0,max_speed=h.bits(64),paused=0,frozen=0,game_per_admin_tick=h.bits(1/16),avi_frames=0))
        self.set_world(48,h.bits(3),h.bits(3),h.bits(1/16))
        self.w32(self.admin+0x28c,h.bits(6));self.w32(self.admin+0x20c,4);self.w32(self.admin+0x218,10)
        self.w32(self.admin+0x2dc,0);self.w32(self.netnode+0x58,h.bits(1));self.w32(self.cell_real,h.bits(100))
        if c['client']:self.w32(self.admin+0x2c4,self.other_admin)
    def hook(self,u,pc,size,data):
        native=(ORIGINAL.base<=pc<ORIGINAL.base+ORIGINAL.image_size)
        if native:PINS[pc]=bytes(u.mem_read(pc,size)).hex()
        event=None
        if pc in getattr(self,'seams',{}):event=self.seams[pc][0]
        elif pc in [0x77d930,0x77d9d0,0x77da70]:event=self.wstr(self.arg(0))
        elif pc==0x74fd10:event='client_replay_history'
        if event is not None and event==self.c['stop']:
            # Record the same seam arguments before the declared service error.
            before=len(self.events);super().hook(u,pc,size,data)
            assert len(self.events)==before+1
            # client replay's actual callback may have already committed its
            # own final store; this fixture seam fails BEFORE that store.
            if pc==0x74fd10:self.w32(self.admin+0x20c,4)
            self.failed=event;raise Stop()
        if event=='host_record_history' and self.c['mutate_admin']:
            self.w32(self.admin+0x28c,h.bits(10))
        return super().hook(u,pc,size,data)
    def run(self):
        initial=self.state()
        try:self.call(0x75ca70,self.admin)
        except Stop:pass
        return dict(initial=initial,final=self.state(),events=self.events,failed=self.failed)
def main():
    rows=[];base=dict(client=False,stop=None,mutate_admin=False)
    cases=[('host-success',{}),('host-live-admin-write',dict(mutate_admin=True)),('client-success',dict(client=True))]
    for stop in ['net_pump','process_commands','admin_history_752DA0','host_rate_75FD80','host_execute_requests','host_record_history','IsFrozen()','m_admin_tick_end_game_time','m_admin_tick_end_admin_time','GetNextTickEventAdminTime()','heartbeat']:
        cases.append(('host-error-'+stop,dict(stop=stop)))
    cases.append(('client-replay-error',dict(client=True,stop='client_replay_history')))
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.NATIVE=image
        for pc in [24,53,64]:
            for name,change in cases:
                c=base|change;n=Machine(c,pc).run()
                assert n['failed']==c['stop'],(name,n)
                if c['stop']=='IsFrozen()':
                    assert n['final']['admin_time']==h.bits(6+1/16)
                    assert n['final']['sim']==n['initial']['sim']
                if c['mutate_admin']:assert n['final']['admin_time']==h.bits(10+1/16)
                rows.append(dict(build=build,precision=pc,name=name,input=c,native=n))
    h.NATIVE=ORIGINAL
    excluded={0x776540,0x75c9a0,0x751770,0x7642a0,0x75b820,0x759680,0x752e60,0x752ea0,0x752da0,0x75fd80,0x74e8b0,0x74e670,0x41c180,0x76f600,0x41c0e0,0x41c110,0x77d930,0x77d9d0,0x77da70,0x74fd10,0x752ae0,0x4f9970}
    pins={hex(a):v for a,v in sorted(PINS.items()) if a not in excluded}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    out=dict(schema=1,scope=__doc__,producer_sha256=h.digest(Path(__file__).read_bytes()),machine_sha256=h.digest(Path(h.__file__).read_bytes()),rows=rows,pins=pins)
    p=HERE/'native-borrowed-driver.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'borrowed driver replay differs'
    print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=h.digest(p.read_bytes()))))
if __name__=='__main__':main()
