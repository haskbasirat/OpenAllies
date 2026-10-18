"""Execute original Regiment constructor526260, state list521C30 and Load528C20.

All read/version gates, sorted engage insertion51FE60, linked lists, path52D2C0,
join vector growth522E60 and formation-index reader4C2D90 execute originally.
Seams inherited: stream byte source, CString storage/read, allocator/free and
named definition registry lookup. Stream mode+14 explicitly returnsfalse to
select native saved-input mode. Worldtime and actual formationtable are supplied
inputs, not reader seams. Zero ordinary memory accesses fault. Original andv14
instructions are checked equal. No FPU arithmetic occurs in this Load.
"""
from pathlib import Path
import sys,json,struct,random,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/resume-20261016-mind-commands'))
from native_commands import Machine as Base,P,V,VISITED,H,W,text,command,Outcome
from unicorn.x86_const import *
STATES=[(0x34,'cai_sleep'),(0x54,'cai_rampage'),(0x68,'cai_move'),(0xc0,'cai_kill'),
 (0xd4,'cai_bombard'),(0xf8,'cai_siege'),(0x120,'cai_chasing'),(0x134,'cai_construct'),
 (0x160,'cai_repair'),(0x17c,'cai_guard'),(0x1a8,'cai_regroup'),(0x1b8,'cai_reform'),
 (0x1cc,'cai_engage'),(0x1f0,'cai_retreat'),(0x20c,'cai_rout'),(0x220,'cai_explore'),
 (0x24c,'cai_join'),(0x270,'cai_exhausted'),(0x280,'cai_unpack'),(0x2a4,'cai_pack')]

class Machine(Base):
    def __init__(self,payload,pc,worldtime):
        super().__init__(payload,pc)
        self.u.mem_write(0x2e00480,b'\x31\xc0\xc3')
        self.write(self.read(self.stream)+0x14,0x2e00480)
        self.write(self.world+0xe0,worldtime)
        self.formations=[self.alloc(8) for _ in range(3)]
    def hook(self,u,pc,size,arg):
        if pc==0x858ad0:raise Outcome('formation_range')
        super().hook(u,pc,size,arg)
    def regiment(self):
        for i,(_,name) in enumerate(STATES):self.write(0xb3cf8c+i*24,self.string(name.encode('utf-16-le')))
        owner=self.alloc(0x200);self.write(owner+4,self.alloc(0x800))
        organization=self.alloc(0x200);self.write(owner+0x7c,organization)
        layout=self.alloc(0x50);self.write(organization+0x18,layout)
        table=self.alloc(12)
        for i,p in enumerate(self.formations):self.write(table+4*i,p)
        self.write(layout+0x34,table);self.write(layout+0x38,3)
        obj=self.alloc(0x2c4);self.u.mem_write(obj,b'\xcd'*0x2c4)
        self.call(0x526260,obj,[owner]);before=bytes(self.u.mem_read(obj,0x2c4)).hex()
        try:self.call(0x528c20,obj,[self.world]);outcome='loaded'
        except Outcome as e:outcome=str(e)
        out={'outcome':outcome,'consumed':self.r.at}
        if outcome!='loaded':return out
        image=bytes(self.u.mem_read(obj,0x2c4))
        def linked(off,stride=4,nxt=4):
            values=[];p=self.read(obj+off)
            while p:values.append([self.read(p+i*4) for i in range(stride)]);p=self.read(p+nxt)
            return values
        path=linked(0xa0,8,0x24)
        tracked=[x[0] for x in linked(0x2bc,1)]
        engaged=[{'gtid':x[0],'priority':struct.unpack('<i',struct.pack('<I',x[3]))[0]} for x in linked(0x1dc,4,8)]
        joined=[self.read(self.read(obj+0x264)+4*i) for i in range(self.read(obj+0x268))]
        stack=[x[0]-obj for x in linked(0x10,1)]
        pending=[self.fields(x[0]) for x in linked(0x18,1)]
        current=self.read(obj+0x20);ref=self.read(obj+0x148);formation=self.read(obj+0xb8)
        ptr=self.read(obj+0xe8);units=self.read(ptr-12)
        bombard=[int.from_bytes(self.u.mem_read(ptr+i*2,2),'little') for i in range(units)]
        out.update({'before':before,'image':image.hex(),'path':path,'tracked':tracked,'engage':engaged,
           'join':joined,'stack':stack,'pending':pending,'current':self.fields(current) if current else None,
           'construct_definition':self.ref_names[ref]['units'] if ref else None,
           'formation_index':self.formations.index(formation) if formation else None,'bombard_text':bombard})
        return out

def payload(v,seed,rich=False,baseversion=3):
    rng=random.Random(seed)
    out=bytearray(W(v))
    def word(n=1):out.extend(W(*[rng.getrandbits(32) for _ in range(n)]))
    def byte():out.append(rng.randrange(256))
    if not v:return bytes(out)
    out.extend(W(1,7,baseversion));word(2)
    out.extend(W(2 if rich else 0))
    if rich:out.extend(command(0,seed)+command(3,seed+1))
    if baseversion>=1:
        word();names=[name for _,name in STATES] if rich else ['cai_sleep']
        out.extend(W(len(names)))
        for name in names:out.extend(text(name))
    if baseversion>=3:
        out.append(2 if rich else 0)
        if rich:out.extend(command(3,seed+2))
    word(2)
    if v<2:return bytes(out)
    out.extend(W(3 if rich else 0))
    if rich:word(3)
    word()
    if v>=9:byte();word()
    if v>=12:byte()
    word();byte();word(8)
    if v>=15:word(2)
    out.extend(W(0,3 if rich else 0))
    if rich:
        for i in range(3):word(4);out.extend(W(i));word(4)
    word()
    if v>=3:out.extend(W(2 if rich else 0xffffffff))
    word(3)
    if v<8:byte()
    word()
    if v>=6:out.extend(text('bombard_source' if rich else None));word(2)
    if v>=4:
        word(2)
        if v<5 or v>=13:word()
    word(3);byte();out.extend(text('bunker' if rich else None));word(2)
    if v>=11:word()
    if v>=20:word(2)
    word();byte();word()
    if v>=10:byte()
    word(2);byte();word(9)
    out.extend(W(4 if rich else 0))
    if rich:
        for priority,gtid in [(5,10),(-2,11),(5,12),(0,13)]:out.extend(W(priority,gtid))
    if v>=18:word()
    if v>=19:byte()
    if v>=5:byte()
    word(5);word(2);word(6)
    if v>=17:word(3)
    word(2);byte()
    if v>=7:
        byte();out.extend(W(4 if rich else 0))
        if rich:word(4)
    word()
    if v>=14:
        word(3)
        if v>=21:word(2)
        if v>=16:word()
        word()
    return bytes(out)

def main():
    rows=[]
    for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
        for v in range(22):
            for rich in [False,True]:
                for baseversion in [0,3]:
                    data=payload(v,0x528c20+v,rich,baseversion);worldtime=0x41480000+v
                    m=Machine(data+b'AFTER',cw,worldtime);native=m.regiment()
                    assert native['consumed']==len(data),(v,rich,baseversion,native['consumed'],len(data))
                    rows.append({'version':v,'precision':pc,'payload':data.hex(),'world_time':worldtime,'native':native})
    for v in [22,0xffffffff]:
        data=W(v);native=Machine(data,0x27f,0).regiment()
        rows.append({'version':v,'precision':'Double53','payload':data.hex(),'world_time':0,'native':native})
    doc={'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),
         'cases':rows,'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(VISITED.items())]}
    p=HERE/'native-regiment.json';p.write_text(json.dumps(doc,indent=1)+'\n');print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
