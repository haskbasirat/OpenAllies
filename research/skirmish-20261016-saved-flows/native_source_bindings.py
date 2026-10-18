"""Bounded source binding proof: Resource manager address alias and integer SVar overwrite.

The alias windows execute original instructions, not a simulated constructor.
The Settings tests execute native typed-int field insertion and full4B3C10;
string allocation, successful allocation and debug formatting are explicit leaves.
"""
from pathlib import Path
import sys, json
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT/'research/phase8-ai'))
from native_machine import Machine as Protected, original, modified, sha, STACK, PRECISIONS
sys.path.insert(0, str(ROOT/'research/phase7-definitions'))
from probe_lists import Machine as Parser
from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

class SettingsMachine(Parser):
    def __init__(self, image, precision):
        super().__init__()
        original_end=(original.image_size+0xfff)&~0xfff
        image_end=(image.image_size+0xfff)&~0xfff
        if image_end>original_end:self.u.mem_map(original.base+original_end,image_end-original_end)
        for _, rva, size, offset in image.sections:
            self.u.mem_write(image.base+rva, image.b[offset:offset+size])
        # Restore the harness' TLS locale after remapping the readonly PE.
        self.w(0x2255064,self.r(0xb35444))
        self.u.reg_write(UC_X86_REG_FPCW, PRECISIONS[precision])
        self.null_accesses = []
        self.u.hook_add(UC_HOOK_MEM_READ | UC_HOOK_MEM_WRITE, self.guard, begin=0, end=0xfff)
    def guard(self,u,access,address,size,value,z):
        pc=u.reg_read(UC_X86_REG_EIP)
        raw=bytes(u.mem_read(pc,4))
        prefix=[]
        for b in raw:
            if b not in (0x26,0x2e,0x36,0x3e,0x64,0x65,0x66,0x67,0xf0,0xf2,0xf3):break
            prefix.append(b)
        if address+size>4 or 0x64 not in prefix:
            self.null_accesses.append([pc,address,size]);u.emu_stop()
    def hook(self,u,pc,n,z):
        if pc in [0x404f60,0x404ed0,0x404ea0]: self.ret()
        elif pc==0x5b3b10:
            out=self.arg();self.w(out,self.string(list(map(ord,'fixture'))));self.ret(out,pop=4)
        else: super().hook(u,pc,n,z)
    def make(self, value):
        node=self.node('SVars');parser=self.alloc(0x80);token=self.alloc(0x30)
        self.w(parser,node);self.w(parser+0x40,self.string(list(map(ord,'EconomyLimitedResourceMax'))));self.w(parser+0x38,2)
        self.w(token+0x18,self.string(list(map(ord,str(value)))));self.w(token+0x10,7)
        self.invoke(0x5bbe70,parser,[token]);return node
    def sequence(self, values):
        settings=self.alloc(0x80);self.invoke(0x4b2ce0,settings,[]);rows=[];first=None
        for value in values:
            self.invoke(0x4b3c10,settings,[self.make(value)])
            count=self.r(settings+12);var=self.r(self.r(settings+8))
            if first is None:first=var
            rows.append({'value':self.r(var),'kind':self.r(var+36),'flags':self.r(var+40),'count':count,'same_identity':first==var})
        assert not self.null_accesses,self.null_accesses
        return rows

def main():
    aliases=[];settings=[];pins={}
    for label,image in [('original',original),('v14',modified)]:
        for pc in PRECISIONS:
            for count in [0,1,7,17]:
                m=Protected(pc)
                original_end=(original.image_size+0xfff)&~0xfff
                image_end=(image.image_size+0xfff)&~0xfff
                if image_end>original_end:m.u.mem_map(original.base+original_end,image_end-original_end)
                for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
                data=m.alloc(0x100)
                m.u.reg_write(UC_X86_REG_ESI,data);m.u.reg_write(UC_X86_REG_ESP,STACK)
                m.u.emu_start(0x48a901,0x48a917,count=20)
                m.u.reg_write(UC_X86_REG_EBX,0)
                m.u.emu_start(0x48a96c,0x47f670,count=20)
                manager=m.ecx();assert manager==data+0x10
                # Enter the real constructor's identity-store window with its
                # native ESI=this established by47F68D. No allocation is claimed.
                m.u.reg_write(UC_X86_REG_ESI,manager)
                m.u.emu_start(0x47f699,0x47f6b4,count=20)
                assert m.r(0xb3a660)==m.r(0xb36530)+0x10
                m.w(data+0x38,count)
                assert m.r(m.r(0xb3a660)+0x28)==count
                assert not m.null_accesses
                m.verify_v14();pins.update(m.visited)
                aliases.append({'image':label,'precision':pc,'count':count,'manager_relative_to_game_data':manager-data,'count_word_relative_to_game_data':0x38})
            for values in [[150,20],[150,100],[150,150,100],[150,0],[150,-7],[150,2147483647],[150,-2147483648]]:
                m=SettingsMachine(image,pc);out=m.sequence(values)
                assert [r['value'] for r in out]==[v&0xffffffff for v in values]
                assert all(r['kind']==3 and r['flags']==0 and r['count']==1 and r['same_identity'] for r in out)
                for a,n in m.visited.items():
                    if original.base<=a<original.base+original.image_size:
                        b=image.range(a,n);assert b==original.range(a,n)==modified.range(a,n);pins[a]=b
                settings.append({'image':label,'precision':pc,'sequence':values,'states':out})
    for a,z in [(0x50880c,0x50884d),(0x4e3940,0x4e39cf),(0x4e48c0,0x4e48ca),(0x48a901,0x48a917),(0x48a96c,0x48a97f),(0x47f699,0x47f6b4)]:
        raw=original.range(a,z-a);assert raw==modified.range(a,z-a);pins[a]=raw
    report={'scope':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'alias_cases':aliases,'settings_cases':settings,'pins':[{'address':hex(a),'bytes':b.hex()} for a,b in sorted(pins.items())],'limits':['Alias proof executes only identified constructor windows, not whole GameData initialization.','Settings supplies typed integer source declarations, successful allocator/stringpool and no-op logs; no wildcard OS traversal is claimed.','Current Settings lookup must still reject unknown WorldRule overrides and mismatching source metadata.']}
    path=Path(__file__).with_name('native-source-bindings.json');path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'alias_cases':len(aliases),'settings_cases':len(settings),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()
