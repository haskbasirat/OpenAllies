"""Original GMap serialized tile loop and static passability; suffix load excluded."""
from pathlib import Path
import hashlib, json, random, struct, sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(OUT))
from original_height_binding import BindingMachine, original, modified
from unicorn.x86_const import *
W=lambda *v:struct.pack('<'+'I'*len(v),*v)
H=lambda b:hashlib.sha256(b).hexdigest()
S=lambda s:struct.pack('<H',len(s))+s.encode('utf-16-le')

class TerrainMachine(BindingMachine):
    EMPTY=0x2008000; ALLOCATOR=0x2009000; STRINGTABLE=0x2009100
    STRINGNEW=0x200f200; STRINGFREE=0x200f210; REGISTRY=0x2009200
    def __init__(self):
        super().__init__()
        self.u.mem_write(self.EMPTY-16,b'\0'*32)
        self.w(self.EMPTY-16,self.ALLOCATOR);self.w(self.EMPTY-4,1)
        self.w(self.ALLOCATOR,self.STRINGTABLE)
        self.w(self.STRINGTABLE+12,self.STRINGNEW);self.w(self.STRINGTABLE+4,self.STRINGFREE)
        self.references={};self.seams=[];self.biome=0
    def alloc(self,n):
        p=self.heap;self.heap+=(n+15)&~15;assert self.heap<0x5000000
        if n:self.u.mem_write(p,b'\0'*n)
        return p
    def string(self,n):
        p=self.alloc(n*2+18)+16
        self.w(p-16,self.ALLOCATOR);self.w(p-12,n);self.w(p-8,n);self.w(p-4,1)
        return p
    def arg(self,n=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+n*4)
    def take_string(self):
        n=struct.unpack_from('<H',self.data,self.at)[0];self.at+=2
        b=self.data[self.at:self.at+2*n];assert len(b)==2*n;self.at+=2*n
        return b.decode('utf-16-le')
    def hook(self,u,pc,size,user):
        ecx=u.reg_read(UC_X86_REG_ECX)
        if pc==0x557d65:
            # Native has already incremented its missing counter. Formatting
            # the first two diagnostic messages consumes no input/world state.
            self.seams.append('0x557d65..0x557e33 missing-variation diagnostics')
            u.reg_write(UC_X86_REG_EIP,0x557e33);return
        if pc in (0x513700,0x557620,0x557680,0x856370,0x401330,0x405b80,0x48d500,0x59e520,self.STRINGNEW,self.STRINGFREE):
            self.seams.append(hex(pc))
            if pc==0x513700:
                self.biome_name=self.take_string();self.w(self.arg(1),self.biome);self.ret(0,self.arg());return
            if pc==0x557620:
                # Empty palette allocation shell. The original557960 read loop,
                # length/count handling and resolved-index publication execute.
                self.w(ecx,self.REGISTRY);self.w(ecx+4,0);self.w(ecx+8,0);self.w(ecx+12,0);self.ret(0,ecx);return
            if pc==0x557680:
                n=self.arg();self.w(ecx,self.alloc(n*8));self.w(ecx+8,n);self.ret(4);return
            if pc==0x856370:self.ret(0,self.ALLOCATOR);return
            if pc==self.STRINGNEW:self.ret(0,self.string(0)-16);return
            if pc==self.STRINGFREE:self.ret(4);return
            if pc==0x401330:
                n=self.arg();self.w(ecx,self.string(n));self.ret(4,ecx);return
            if pc==0x405b80:
                self.w(ecx,self.r(self.arg()));self.ret(4,ecx);return
            if pc==0x48d500:
                p=self.arg();n=self.r(p-12);name=bytes(u.mem_read(p,n*2)).decode('utf-16-le')
                assert name in self.references,name;self.ret(4,self.references[name]);return
            if pc==0x59e520:self.ret(0,0);return
        if pc==0x4f0110:raise ValueError('native terrain format rejected')
        super().hook(u,pc,size,user)
    def layout(self,bits,resolution):
        self.reset(0);self.w(self.WORLD+0x3c,bits[0]);self.w(self.WORLD+0x40,bits[1])
        self.w(self.SVARS+4,self.VALUES+24);self.w(self.VALUES+24,resolution)
        self.u.reg_write(UC_X86_REG_ESI,self.STATE)
        self.u.emu_start(0x5565e1,0x556668,count=100000)
        assert self.u.reg_read(UC_X86_REG_EIP)==0x556668
        return {'extent_i32':[self.r(self.STATE+20),self.r(self.STATE+24)],
            'width':self.r(self.STATE+28),'height':self.r(self.STATE+32),
            'coordinate_shift':self.r(self.STATE+4),'cell_size_bits':self.r(self.STATE+8)}
    def load(self,data,width,height,resolution,definitions,initial=None):
        self.reset(0);self.data=data;self.at=0;self.reads=[];self.seams=[]
        self.references={};self.biome=self.alloc(0x100);bio_data=self.alloc(0x100);self.w(self.biome+0x78,bio_data)
        for index,d in enumerate(definitions):
            if d.get('unresolved'):
                self.references[d['id']]=0
                continue
            variation=self.alloc(0x40);definition=self.alloc(0x80)
            self.w(variation+0x1c,definition);self.w(definition+0x1c,d['native_type']);self.w(definition+0x4c,d['passability_bits'])
            self.references[d['id']]=variation
            if index==0:self.w(bio_data+0x30,variation)
        initial_pointer=None
        if initial is not None:
            initial_pointer=self.alloc(0x40);definition=self.alloc(0x80)
            self.w(initial_pointer+28,definition);self.w(definition+28,initial['native_type']);self.w(definition+76,initial['passability_bits'])
            self.w(bio_data+0x30,initial_pointer)
        count=width*height;tiles=self.alloc(count*48);regions=self.alloc(256*4)
        for i in range(256):self.w(regions+i*4,self.alloc(8))
        self.w(self.STATE+4,resolution);self.w(self.STATE+28,width);self.w(self.STATE+32,height)
        self.w(self.STATE+40,tiles);self.w(self.STATE+0x50,regions)
        self.w(self.STACK+4,self.STREAM)
        self.u.emu_start(0x557ad0,0x557f0b,count=1000000)
        assert self.u.reg_read(UC_X86_REG_EIP)==0x557f0b
        records=[];inverse={ptr:i for i,ptr in enumerate(self.references.values()) if ptr}
        if initial_pointer is not None:inverse[initial_pointer]=None
        for i in range(count):
            p=tiles+i*48
            records.append({'variation_index':inverse[self.r(p)],'flags20':self.r(p+32),'byte_2c':self.u.mem_read(p+44,1)[0],
                'runtime_region_word':self.r(p+20)})
        return {'consumed':self.at,'cells':records,'missing_count':self.r(self.u.reg_read(UC_X86_REG_ESP)+0x20),'read_calls':self.reads,'seams':sorted(set(self.seams))}
    def flags(self,flags,passability):
        self.reset(0);variation=self.alloc(0x40);definition=self.alloc(0x80)
        self.w(self.STATE,variation);self.w(variation+28,definition);self.w(definition+76,passability);self.w(self.STATE+32,flags)
        self.u.emu_start(0x54ee20,self.STOP,count=100)
        assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
        return self.r(self.STATE+32)

def main():
    rng=random.Random(0x557ad0);m=TerrainMachine();layouts=[];loads=[];flags=[];fallback=[]
    for res in [0,1,2,3,5]:
        for xy in [(32.,16.),(31.01,19.9),(100.,256.),(65535.,100.)]:
            bits=[struct.unpack('<I',struct.pack('<f',v))[0]for v in xy]
            layouts.append({'extent_bits':bits,'resolution':res,'native':m.layout(bits,res)})
    defs=[{'id':'land','native_type':0,'passability_bits':1},{'id':'water','native_type':2,'passability_bits':2},
          {'id':'ford','native_type':2,'passability_bits':1},{'id':'mountain','native_type':1,'passability_bits':0},
          {'id':'both','native_type':0,'passability_bits':3}]
    for version in [6,7]:
        for sample in range(16):
            width,height=(sample%4+1,sample%3+1);n=width*height;stride=6 if version==6 else 8
            records=[]
            for _ in range(n):
                idx=rng.randrange(len(defs));flag=rng.randrange(256);region=rng.randrange(65536);byte=rng.randrange(256)
                records.append(struct.pack('<BBHBB',idx,flag,region,byte,rng.randrange(256))if version==6 else struct.pack('<HHHBB',idx,region,byte|rng.randrange(256)<<8,flag,rng.randrange(256)))
            data=W(version,1)+S('testbiome')+W(len(defs))+b''.join(S(d['id'])for d in defs)+W(n,stride)+b''.join(records)+b'opaque suffix'
            result=m.load(data,width,height,1,defs)
            assert result['consumed']==len(data)-len(b'opaque suffix')
            loads.append({'version':version,'width':width,'height':height,'resolution':1,'definitions':defs,'input_hex':data.hex(),'native':result})
    for version in [6,7]:
        for passability in range(4):
            initial={'id':'explicit_biome_default','native_type':0,'passability_bits':passability}
            unresolved=defs+[{'id':'missing_a','unresolved':True},{'id':'missing_b','unresolved':True}]
            indices=[5,6,1,5,6,2,5,3,6];stride=6 if version==6 else 8
            raw=[]
            for i,index in enumerate(indices):
                raw.append(struct.pack('<BBHBB',index,i%2,i+100,i,55)if version==6 else struct.pack('<HHHBB',index,i+100,i,i%2,77))
            data=W(version,1)+S('testbiome')+W(len(unresolved))+b''.join(S(d['id'])for d in unresolved)+W(9,stride)+b''.join(raw)
            actual=m.load(data,3,3,1,unresolved,initial)
            assert actual['missing_count']==6
            fallback.append({'version':version,'definitions':unresolved,'initial':initial,'input_hex':data.hex(),'native':actual})
    for flag in list(range(256))+[0xffffffff,0xdeadbeee,0x100,0x200,0x400,0x800]:
        for p in range(4):flags.append({'flags20':flag,'passability_bits':p,'native':m.flags(flag,p)})
    windows=[]
    for pc,size in sorted(m.visited.items()):
        raw=original.range(pc,size);assert raw==modified.range(pc,size),hex(pc)
        windows.append({'va':hex(pc),'bytes':raw.hex()})
    out={'schema':1,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),
        'scope':'Original layout5565E1..556668, tile input557AD0..557F0B including native557960 palette read and552C50/54EE20. Allocation/string storage/reference lookup/biome reference transport are explicit seams. Stream mode59E520=false; aux suffix/region lookup and world publication are excluded.',
        'native_executed_windows':windows,'layouts':layouts,'loads':loads,'flags':flags,'fallback':fallback}
    path=OUT/'native-terrain.json';path.write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps({'layouts':len(layouts),'loads':len(loads),'flags':len(flags),'fallback':len(fallback),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
