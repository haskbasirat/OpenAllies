"""Native MISC_DATA body4FD017..4FD3B3 and reached ctor field slices.
The surrounding GWorld::Load/chunk-seek/lifecycle is outside this harness.
Native lookup registries are explicit dictionary leaves, never source receipts.
"""
from pathlib import Path
import sys,json,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import *
ORIGINAL,V14=h.P,h.V
def w(x):return struct.pack('<I',x&0xffffffff)
def s(x):
    raw=x.encode('utf-16-le',errors='surrogatepass') if isinstance(x,str) else b''.join(struct.pack('<H',i) for i in x)
    return struct.pack('<H',len(raw)//2)+raw
def payload(v,weather='rain',presence=1,count=3,used_count=3):
    b=b''
    if v>=12:
        b+=w(count)
        if count>0:
            for name,value in [('alpha',11),('missing',22),('ALIAS',33)][:count]:b+=s(name)+w(value)
    if v>=13:b+=w(0x7fc01234)
    if v>=14:b+=s(weather)
    if v>=24:
        b+=w(used_count)
        if used_count>0:
            for text in ['first',[0xd800,0x41],'last'][:used_count]:b+=s(text)
    if v>=28:
        b+=bytes([presence])
        if presence:b+=w(0x12345678)+w(0x87654321)
    return b
def cases():
    rows=[]
    def add(name,v,b,**kw):rows.append(dict(name=name,version=v,payload=b.hex(),speed=0x3f800001,seed=True,**kw))
    for v in [0,11,12,13,14,17,18,23,24,27,28]:add(f'v{v}',v,payload(v))
    b=payload(28)
    for end in range(len(b)):add(f'truncated-{end}',28,b[:end])
    add('empty-weather',28,payload(28,weather=''))
    add('unknown-weather',28,payload(28,weather='missing'))
    add('no-new-pair',28,payload(28,presence=0))
    add('negative-counts',28,payload(28,count=-3,used_count=-2))
    add('name-nul-tail',28,payload(28).replace(s('alpha'),s('alpha\0ignored')))
    add('weather-nul-tail',28,payload(28,weather='rain\0ignored'))
    add('weather-leading-nul',28,payload(28,weather='\0rain'))
    return rows
class MiscMachine(h.Machine):
    def __init__(self,c,pc):
        self.initialized=set();super().__init__(bytes.fromhex(c['payload']),pc)
        self.c=c;self.events=[];self.defs={101:'alpha',202:'rain'}
        self.ptrs={k:self.alloc(0x20) for k in self.defs}
        self.world=self.alloc(0x300,0xa5)
        game=self.alloc(0x700);mgr=self.alloc(0x50);self.w(game+0x59c,mgr);self.w(0xb36530,game)
        settings=self.alloc(0x200);speed=self.alloc(4);self.w(speed,c['speed']);self.w(settings+0x114,speed);self.w(0xb39ab0,settings)
        def lookup(m):
            ids=m.read_wide(m.arg(0));m.events.append(['name_list',ids]);token={'alpha':101,'alias':101}.get(ids.lower())
            m.ret(4,m.ptrs[token] if token else 0)
        self.seam(0x463fa0,lookup)
        def weather(m):
            ids=m.read_wide(m.arg(0));m.events.append(['weather',ids]);token={'rain':202}.get(ids.lower())
            m.ret(4,m.ptrs[token] if token else 0)
        self.seam(0x48d500,weather)
        def warning(m):
            fmt=m.cstring_text(m.arg(1));assert fmt=="Saved name list no longer exists '%s'"
            m.events.append(['warn',m.read_wide(m.arg(2))]);m.ret(0)
        self.seam(0x5a1040,warning)
        self.probes[0x4fd3b3]=lambda m:m.u.reg_write(UC_X86_REG_EIP,h.STOP)
    def _read(self,m):
        dst,n=m.arg(0),m.arg(1);data=m.take(n);m.u.mem_write(dst,data)
        self.initialized.update(range(dst,dst+n));m.ret(8,m.stream)
    def block(self,start,end):
        self.u.emu_start(start,end,count=2000000)
    def token(self,p):
        return next((t for t,a in self.ptrs.items() if a==p),0 if p==0 else None)
    def text(self,p):return list(struct.unpack('<'+'H'*self.r(p-12),bytes(self.u.mem_read(p,self.r(p-12)*2))))
    def state(self,ctor=False):
        p,n=self.r(self.world+0x268),self.r(self.world+0x26c)
        names=[[self.token(self.r(self.r(p+4*i))),self.r(self.r(p+4*i)+4)] for i in range(n)]
        weather=self.r(self.world+0x290)
        weather='Uninitialized' if weather==0xa5a5a5a5 else self.token(weather)
        texts=[];p=self.r(self.world+0x294)
        while p:texts.append(self.text(self.r(p)));p=self.r(p+4)
        pair=self.r(self.world+0x27c)
        vals=None if pair==0 else [self.r(pair+i) if pair+i in self.initialized else None for i in (0,4)]
        return dict(names=names,speed=self.r(self.world+0x278),weather=weather,texts=texts,pair=vals)
    def run(self):
        self.u.reg_write(UC_X86_REG_ESP,h.STACK);self.u.reg_write(UC_X86_REG_ESI,self.world)
        self.u.reg_write(UC_X86_REG_EBX,0);self.u.reg_write(UC_X86_REG_EDI,5)
        self.block(0x4f8018,0x4f8072)
        self.block(0x4f8125,0x4f813d)
        ctor=self.state(True)
        if self.c['seed']:
            names=self.alloc(0x100);entry=self.alloc(8);self.w(entry,self.ptrs[101]);self.w(entry+4,99)
            self.w(names,entry);self.w(self.world+0x268,names);self.w(self.world+0x26c,1);self.w(self.world+0x270,64)
            self.w(self.world+0x290,self.ptrs[202])
            node=self.alloc(8);self.w(node,self.cstring('existing'));self.w(node+4,0)
            self.w(self.world+0x294,node);self.w(self.world+0x298,node)
            pair=self.alloc(8);self.w(pair,7);self.w(pair+4,8);self.initialized.update(range(pair,pair+8));self.w(self.world+0x27c,pair)
        self.u.reg_write(UC_X86_REG_ESP,h.STACK)
        self.w(h.STACK+0x14,self.c['version']);self.w(h.STACK+0x20,self.world);self.w(h.STACK+0x80,self.ctx)
        self.u.reg_write(UC_X86_REG_EDI,self.world);self.u.reg_write(UC_X86_REG_EBP,self.stream)
        self.u.reg_write(UC_X86_REG_EBX,self.ctx);self.u.reg_write(UC_X86_REG_ESI,0)
        try:self.block(0x4fd017,h.STOP)
        except h.Throw:pass
        if self.outcome is None:assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP
        return dict(ctor=ctor,state=self.state(),outcome=self.outcome,consumed=self.at,reads=self.reads,events=self.events)
def main():
    rows=[]
    for build,image in [('original',ORIGINAL),('v14',V14)]:
        h.P=image
        for pc in h.CW:
            for c in cases():rows.append(dict(build=build,precision=pc,input=c,native=MiscMachine(c,pc).run()))
    h.P=ORIGINAL
    excluded={0x463fa0,0x48d500,0x5a1040,0x6f7810,0x6f7430,0x856370,0x401b00,0x856560,
              0x41d290,0x41dfb0,0x858a00,0x858ad0,0x4f0110,0x404ea0,0x404f30}
    pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
    for a,raw in pins.items():a=int(a,16);assert ORIGINAL.range(a,len(bytes.fromhex(raw)))==V14.range(a,len(bytes.fromhex(raw)))
    out=dict(schema=1,scope='MISC body and relevant constructor-field slices, explicit registry leaves',
       producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
       machine_sha256=hashlib.sha256(Path(h.__file__).read_bytes()).hexdigest(),rows=rows,pins=pins)
    path=HERE/'native-saved-misc.json';path.write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
