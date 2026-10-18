"""Actual Nation48C050 prefix producing+38 from required game_music.
Typed source/name lookup, Audio registry and fatal are explicit seams. Cutoff is
the next placement_group_IDS lookup, AFTER the actual+38 store. Not full Nation
loading, parser, exception unwinding, or a mutable-current-Nation receipt.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import UC_X86_REG_ECX
ORIGINAL,V14=h.P,h.V
class Machine(h.Machine):
    def __init__(self,c,pc):
        super().__init__(b'',pc);self.c=c;self.events=[]
        self.nation=self.alloc(0x90);self.node=self.alloc(0x30);self.data=self.alloc(0x600)
        self.manager=self.alloc(0x50);self.audio=self.alloc(0x90)
        self.w(self.nation+8,self.cstring('nation_fixture'))
        self.w(self.nation+0x10,self.cstring('already-loaded' if c['duplicate'] else ''))
        self.w(self.nation+0x38,0xa5a5a5a5);self.u.mem_write(self.nation+0x14,b'\xa7')
        self.w(0xb36530,self.data);self.w(self.data+0x584,self.manager)
        self.w(self.manager+0x20,self.cstring('Audio'))
        def name(m):
            assert m.read_wide(m.arg(0))=='name';m.events.append(['name'])
            m.ret(4,m.cstring('fixture name'))
        self.seam(0x5b5620,name)
        self.seam(0x405b80,lambda m:m.ret(4,m.u.reg_read(UC_X86_REG_ECX)))
        def field(m):
            key=m.read_wide(m.arg(0))
            if key=='placement_group_IDS':m.outcome='LoadedMusicPrefix';raise h.Throw()
            assert key=='game_music' and m.u.reg_read(UC_X86_REG_ECX)==m.node
            m.events.append(['game_music'])
            if c['missing']:m.outcome='RequiredGameMusicError';raise h.Throw()
            box=m.alloc(4);m.w(box,m.cstring(c['music']));m.ret(4,box)
        self.seam(0x5b54d0,field)
        def lookup(m):
            assert m.u.reg_read(UC_X86_REG_ECX)==m.manager
            name=m.read_wide(m.arg(0));m.events.append(['lookup',name])
            m.ret(4,m.audio if name=='audio_fixture' else 0)
        self.seam(0x416370,lookup)
        def fatal(m):
            m.events.append(['fatal']);m.outcome='DuplicateNationFatal' if c['duplicate'] else 'UnknownAudioFatal'
            raise h.Throw()
        self.seam(0x410c40,fatal)
    def run(self):
        self.call(0x48c050,self.nation,[self.node])
        value=self.r(self.nation+0x38)
        return dict(outcome=self.outcome,events=self.events,music='resolved-audio' if value==self.audio else value,
                    loaded_flag=self.u.mem_read(self.nation+0x14,1)[0])
def main():
    base=dict(music='audio_fixture',missing=False,duplicate=False)
    cases=[('known',{}),('empty',dict(music='')),('unknown',dict(music='unknown')),
           ('missing',dict(missing=True)),('duplicate',dict(duplicate=True)),
           ('nul-prefix',dict(music='audio_fixture\0ignored'))]
    rows=[]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for name,change in cases:
                c=base|change;n=Machine(c,pc).run()
                expected='DuplicateNationFatal' if c['duplicate'] else 'RequiredGameMusicError' if c['missing'] else 'UnknownAudioFatal' if name=='unknown' else 'LoadedMusicPrefix'
                assert n['outcome']==expected,(name,n)
                assert n['music']==('resolved-audio' if name in ('known','nul-prefix') else 0 if name=='empty' else 0xa5a5a5a5),(name,n)
                assert n['loaded_flag']==(0xa7 if c['duplicate'] else 1),(name,n)
                rows.append(dict(build=build,precision=pc,name=name,input=c,native=n))
    h.P=ORIGINAL
    excluded={0x5b5620,0x405b80,0x5b54d0,0x416370,0x410c40}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    out=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),
             machine_sha256=h.H(Path(h.__file__).read_bytes()),rows=rows,pins=pins)
    p=HERE/'native-nation-music.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'Nation music source prefix replay differs'
    print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=h.H(p.read_bytes()))))
if __name__=='__main__':main()
