"""Actual464DB3 required integer getter/store prefix. Lookup5B4750 returns
an explicitly supplied typed field. Actual5B4ED0 and integer vf10 execute;
missing lookup stops at the native exception-building branch5B4F03, while
negative integers reach required fatal410C40. No whole ActorDataLoad claim.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import *
ORIGINAL,V14=h.P,h.V
class Machine(h.Machine):
    def __init__(self,c,pc):
        super().__init__(b'',pc);self.c=c;self.events=[]
        self.actor=self.alloc(0x600,0xa5);self.node=self.alloc(0x58);self.field=self.alloc(0x14)
        self.w(self.field,0xa487c8);self.w(self.field+0x10,c['value'])
        actor_name=self.alloc(28);self.u.mem_write(actor_name,'actor_fixture\0'.encode('utf-16-le'))
        self.w(self.actor+8,actor_name);self.w(self.actor+0x1b4,0x12345678)
        def lookup(m):
            assert m.u.reg_read(UC_X86_REG_ECX)==m.node
            assert m.read_wide(m.arg(0))=='starting_location_priority'
            m.events.append('lookup');m.ret(4,m.field if c['present'] else 0)
        self.seam(0x5b4750,lookup)
        def missing(m):m.outcome='MissingFieldExceptionBranch';raise h.Throw()
        def negative(m):
            assert m.read_wide(m.arg(0))=="starting_location_priority for actor '%s' needs to be positive"
            m.outcome='NegativePriorityFatal';raise h.Throw()
        def complete(m):m.outcome='LoadedPriorityPrefix';raise h.Throw()
        self.probes[0x5b4f03]=missing;self.seam(0x410c40,negative);self.probes[0x464de0]=complete
    def run(self):
        self.u.reg_write(UC_X86_REG_ESP,h.STACK);self.u.reg_write(UC_X86_REG_EBP,self.actor)
        self.u.reg_write(UC_X86_REG_EDI,self.node);self.u.reg_write(UC_X86_REG_EAX,0xa7)
        try:self.u.emu_start(0x464db3,h.STOP,count=100000)
        except h.Throw:pass
        return dict(outcome=self.outcome,priority_word=self.r(self.actor+0x1b4),previous_bool=self.b(self.actor+0x1b3),events=self.events)
def main():
    rows=[]
    cases=[dict(present=True,value=v) for v in [0,1,9,2147483647,0xffffffff,0x80000000]]+[dict(present=False,value=0)]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for c in cases:rows.append(dict(build=build,precision=pc,input=c,native=Machine(c,pc).run()))
    h.P=ORIGINAL
    excluded={0x5b4750,0x410c40,0x5b4f03,0x464de0}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,b in pins.items():assert ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==V14.range(int(a,16),len(bytes.fromhex(b)))
    assert struct.unpack('<I',ORIGINAL.range(0xa487c8+0x10,4))[0]==0x58cb30
    assert ORIGINAL.range(0xa487c8,0x14)==V14.range(0xa487c8,0x14)
    out=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()),rows=rows,pins=pins,
       typed_integer_vtable=dict(address=0xa487c8,bytes=ORIGINAL.range(0xa487c8,0x14).hex()),
       seams=['5B4750 source-node lookup supplied with exact key and typed integer field; native lookup inheritance/parser not re-executed here',
              'missing field cutoff5B4F03 before exception construction; no fallback/default',
              '410C40 required fatal callback stops after actual negative store; no exception/unwind success claim',
              '464DE0 stops before next stance loader; not completed ActorDataLoad'])
    p=HERE/'native-starting-priority.json'
    if '--write' in sys.argv:p.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(p.read_text()),'native priority replay differs'
    print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=h.H(p.read_bytes()))))
if __name__=='__main__':main()
