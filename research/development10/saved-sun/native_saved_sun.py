"""Execute ORIGINAL/v14 Sun ctor4EE0A0 and Load4EDBD0, all x87 PCs.
Only scheduling cancellation50B250 is a new host seam (explicit queue owner).
Stream/allocation/string/error seams are inherited, pinned and enumerated.
"""
from pathlib import Path
import sys, json, struct, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import UC_X86_REG_ECX
ORIGINAL, V14 = h.P, h.V

def w(v): return struct.pack('<I', v & 0xffffffff)
def payload(version=2, uv=1, uid=3):
    b = w(version) + w(uv)
    if uv >= 1: b += w(uid)
    # Distinct words, including exceptional bit patterns (no Load arithmetic).
    vals = [0x7fc01234, 0x80000000, 0x7f800001] + [0x3e000000+i*0x1001 for i in range(23)]
    b += b''.join(w(x) for x in vals)
    if version >= 1:
        b += b'\x81' + w(0x12345678) + w(0x87654321) + b'\xfe' + w(0xabcdef01) + w(0x10203040)
    return b

def cases():
    rows = []
    def add(name, b, **kw):
        rows.append(dict(name=name, payload=b.hex(), colors=[0x3f000001,0x80000000,0x7fc05678],
                         seed=True, table_word0=8, occupied=[7], **kw))
    for version in range(3):
        b = payload(version)
        add(f'v{version}', b)
        # Every prefix ending inside a native read; full byte-by-byte truncation
        # makes read atomicity and version1 terminal reset observable.
        for end in range(len(b)):
            add(f'v{version}-truncated-{end}', b[:end])
    for name, version, uv, uid in [('bad-sun-version',3,1,3), ('negative-sun-version',-1,1,3),
        ('bad-updatable-version',2,2,3), ('negative-updatable-version',2,-1,3),
        ('updatable-v0',2,0,3), ('negative-id',2,1,-5),
        ('occupied',2,1,7), ('above-bound',2,1,9), ('native-id-equals-bound',2,1,8)]:
        add(name, payload(version,uv,uid))
    return rows

class SunMachine(h.Machine):
    def __init__(self, c, pc):
        super().__init__(bytes.fromhex(c['payload']), pc)
        self.c=c
        self.events=[]
        for i,x in enumerate(c['colors']): self.w(0xb2814c+4*i,x)
        world=self.alloc(0x400)
        self.w(0xb36534,world)
        table=self.alloc(16)
        self.slots=self.alloc(4*(c['table_word0']+1))
        self.w(table,c['table_word0']);self.w(table+4,self.slots);self.w(world+0x284,table)
        for i in c['occupied']: self.w(self.slots+4*i,0xdead0000+i)
        def cancel(m):
            assert m.u.reg_read(UC_X86_REG_ECX)==m.sun
            m.events.append(['cancel']);m.w(m.sun+4,0);m.ret(0)
        self.seam(0x50b250,cancel)
        self.probes[0x50bd26]=lambda m:m.events.append(['bound',c['table_word0']])
        self.probes[0x50bd2d]=lambda m:m.events.append(['occupied',m.u.reg_read(h.UC_X86_REG_EAX)])
        self.probes[0x50bd37]=lambda m:m.events.append(['register',m.u.reg_read(h.UC_X86_REG_EAX)])
    def state(self):
        return ([self.r(self.sun+i) for i in range(8,0x70,4)] +
                [self.b(self.sun+0x70),self.r(self.sun+0x74),self.r(self.sun+0x78),
                 self.b(self.sun+0x7c),self.r(self.sun+0x80),self.r(self.sun+0x84)])
    def run(self):
        self.sun=self.alloc(0x88,0xa5)
        self.call(0x4ee0a0,self.sun,[])
        ctor=self.state()
        assert self.r(self.sun)==0xa2a578 and self.r(self.sun+4)==0
        if self.c['seed']:
            for i in range(8,0x70,4): self.w(self.sun+i,0x5a000000+i)
            for i in (0x74,0x78,0x80,0x84): self.w(self.sun+i,0x5a000000+i)
            self.wb(self.sun+0x70,0x55);self.wb(self.sun+0x7c,0x66)
            self.w(self.sun+4,0xdeadbeef) # cancellation explicitly host-owned
        self.call(0x4edbd0,self.sun,[self.ctx])
        regs=[i for i in range(self.c['table_word0']+1) if self.r(self.slots+4*i)==self.sun]
        return dict(ctor=ctor,state=self.state(),outcome=self.outcome,consumed=self.at,
                    reads=self.reads,events=self.events,registered=regs,
                    scheduled_cleared=self.r(self.sun+4)==0)

def main():
    rows=[]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for c in cases(): rows.append(dict(build=build,precision=pc,input=c,native=SunMachine(c,pc).run()))
    h.P=ORIGINAL
    excluded={0x50b250,0x6f7810,0x6f7430,0x856370,0x401b00,0x856560,0x41d290,
              0x41dfb0,0x858a00,0x858ad0,0x4f0110,0x404ea0,0x404f30,0x5a1040}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,raw in pins.items():
        a=int(a,16);assert ORIGINAL.range(a,len(bytes.fromhex(raw)))==V14.range(a,len(bytes.fromhex(raw)))
    out=dict(schema=1,scope='Sun ctor and Load; cancellation/stream/exception leaves explicit',
             producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             machine_sha256=hashlib.sha256(Path(h.__file__).read_bytes()).hexdigest(),
             rows=rows,pins=pins)
    path=HERE/'native-saved-sun.json';path.write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps(dict(rows=len(rows),pins=len(pins),fixture=str(path),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
