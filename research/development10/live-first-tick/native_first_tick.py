"""Actual FirstTick4F96D0 including76AC60/765E00. AVars410CB0,
Audio registry416370, required fatal410C40 and actual SoundManager747630
backend are explicit service seams. No full sound backend/BeginPlaying proof.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import UC_X86_REG_ECX
ORIGINAL,V14=h.P,h.V
class Machine(h.Machine):
    def __init__(self,c,pc):
        super().__init__(b'',pc);self.c=c;self.events=[]
        self.world=self.alloc(0x2a0);self.player=self.alloc(0x60);self.node=self.alloc(0x10)
        self.kingdom=self.alloc(0x400);self.nation=self.alloc(0x90);self.audio=self.alloc(0x90)
        self.sound=self.alloc(0xa0);self.data=self.alloc(0x600);self.audio_mgr=self.alloc(0x50)
        self.w(0xb36534,self.world if c['world'] else 0);self.w(0xb36558,self.node)
        self.w(self.node+8,self.player if c['player'] else 0);self.w(self.player+0x28,self.kingdom if c['kingdom'] else 0)
        self.w(self.kingdom+0x234,self.nation if c['nation'] else 0)
        self.w(self.nation+0x38,self.audio if c['nation_music'] else 0)
        self.w(0xb36530,self.data);self.w(self.data+0x584,self.audio_mgr);self.w(self.audio_mgr+0x20,self.cstring('Audio'))
        self.w(0xb36548,self.sound)
        self.probes[0x76ac60]=lambda m:m.events.append(['local_player'])
        self.probes[0x765e00]=lambda m:m.events.append(['kingdom'])
        self.probes[0x4f96fd]=lambda m:m.events.append(['nation'])
        self.probes[0x4f970b]=lambda m:m.events.append(['nation_music'])
        def observer(m):
            assert m.read_wide(m.arg(1))=='ObserverGameMusic'
            m.events.append(['observer'])
            if c.get('error')=='observer':m.outcome='RequiredObserverError';raise h.Throw()
            m.w(m.arg(0),m.cstring(c['observer']));m.ret(8,m.arg(0))
        self.seam(0x410cb0,observer)
        def lookup(m):
            assert m.u.reg_read(UC_X86_REG_ECX)==m.audio_mgr
            name=m.read_wide(m.arg(0));m.events.append(['lookup',name])
            m.ret(4,m.audio if name=='audio_fixture' else 0)
        self.seam(0x416370,lookup)
        def fatal(m):m.events.append(['fatal']);m.outcome='UnknownAudioFatal';raise h.Throw()
        self.seam(0x410c40,fatal)
        def play(m):
            assert m.u.reg_read(UC_X86_REG_ECX)==m.sound and m.arg(0)==m.audio
            m.events.append(['play'])
            if c.get('error')=='audio':m.outcome='RequiredAudioError';raise h.Throw()
            m.ret(4,0x76543210)
        self.seam(0x747630,play)
    def run(self):
        self.call(0x4f96d0,0,[])
        if isinstance(self.outcome,dict) and self.outcome.get('kind')=='NullDereference':self.outcome='NullPlayerDereference'
        return dict(outcome=self.outcome,events=self.events)
def main():
    base=dict(world=True,player=True,kingdom=True,nation=True,nation_music=True,observer='audio_fixture',error=None)
    changes=[('nation',{}),('null-nation',dict(nation=False)),('null-nation-music',dict(nation_music=False)),
       ('observer',dict(kingdom=False)),('observer-empty',dict(kingdom=False,observer='')),
       ('observer-unknown',dict(kingdom=False,observer='unknown')),('absent-world-null-player',dict(world=False,player=False)),
       ('null-player',dict(player=False)),('observer-required-error',dict(kingdom=False,error='observer')),
       ('nation-audio-required-error',dict(error='audio')),('observer-audio-required-error',dict(kingdom=False,error='audio')),
       ('observer-nul-prefix',dict(kingdom=False,observer='audio_fixture\0ignored'))]
    rows=[]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for name,change in changes:
                c=base|change;rows.append(dict(build=build,precision=pc,name=name,input=c,native=Machine(c,pc).run()))
    h.P=ORIGINAL
    excluded={0x410cb0,0x416370,0x410c40,0x747630,0x6f7430,0x855d30}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    out=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()),rows=rows,pins=pins,
       seams=['410CB0 supplies actual current string value at the native read; no appvar loader/profile policy proof from this seam',
              '416370 lookup input/result is explicit laboratory Audio registry response',
              '410C40 required fatal records stop before exception/unwind',
              '747630 actual audio backend is a required callback; injected throws are bounded prefix probes, not native hardware-error semantics',
              'shared CString allocation/retain/free services outside gameplay selection'])
    p=HERE/'native-first-tick.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'native first tick replay differs'
    print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=h.H(p.read_bytes()))))
if __name__=='__main__':main()
