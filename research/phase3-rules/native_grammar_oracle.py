"""Bounded original-x86 header scan, EOF stack drain and ordinary child-merge decisions."""
from pathlib import Path
import hashlib, json, struct, sys
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
sys.path.insert(0, str(ROOT.parent / 'reconstruction/tools/python-packages'))
from audit_baseline import PE
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

native = PE(ROOT.parent / 'AA.exe')
v14 = PE(ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
H = lambda b: hashlib.sha256(b).hexdigest()
assert H(native.b) == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
assert H(v14.b) == '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'


class Machine:
    def __init__(self):
        self.u = Uc(UC_ARCH_X86, UC_MODE_32)
        self.u.mem_map(native.base, (native.image_size + 4095) & ~4095)
        for vs, rva, size, offset in native.sections:
            self.u.mem_write(native.base + rva, native.b[offset:offset+size])
        self.u.mem_map(0, 4096)
        self.u.mem_map(0x2000000, 0x20000)
        self.u.mem_map(0x3000000, 0x20000)
        self.sp, self.stop = 0x3010000, 0x301f000
        self.callbacks = {}
        self.u.hook_add(UC_HOOK_CODE, self.hook)

    def w(self, at, value): self.u.mem_write(at, struct.pack('<I', value))
    def r(self, at): return struct.unpack('<I', self.u.mem_read(at, 4))[0]
    def arg(self, n=0): return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+4*n)
    def ret(self, value=0, pop=0):
        sp = self.u.reg_read(UC_X86_REG_ESP)
        self.u.reg_write(UC_X86_REG_EAX, value)
        self.u.reg_write(UC_X86_REG_EIP, self.r(sp))
        self.u.reg_write(UC_X86_REG_ESP, sp+4+pop)
    def hook(self, u, pc, size, user):
        if pc in self.callbacks: self.callbacks[pc]()
    def call(self, pc, ecx, args, popped):
        self.w(self.sp, self.stop)
        for i, value in enumerate(args): self.w(self.sp+4+4*i, value)
        self.u.reg_write(UC_X86_REG_ESP, self.sp)
        self.u.reg_write(UC_X86_REG_ECX, ecx)
        self.u.emu_start(pc, self.stop, count=500000)
        assert self.u.reg_read(UC_X86_REG_EIP) == self.stop
        assert self.u.reg_read(UC_X86_REG_ESP) == self.sp+4+popped


def header(text):
    m = Machine(); parser, raw, manager, table, buffer, out = [0x2000000+x for x in (0,0x1000,0x2000,0x2100,0x4000,0x8000)]
    m.u.mem_write(raw, text.encode('utf-16-le')+b'\0\0');m.w(raw-12,len(text));m.w(parser+8,raw)
    m.w(manager,table);m.w(table+12,0x301f010)
    m.w(buffer+8,4096);m.w(buffer+12,1)
    m.callbacks[0x856370] = lambda: m.ret(manager)
    m.callbacks[0x301f010] = lambda: m.ret(buffer)
    m.callbacks[0x4749c0] = lambda: m.ret() # CString trimming boundary, scan bytes are captured unchanged.
    def retain(): m.w(buffer+12,m.r(buffer+12)+1);m.ret(buffer)
    m.callbacks[0x4012c0] = retain
    m.call(0x5bde30, parser, [out], 4)
    result = m.u.mem_read(m.r(out),m.r(buffer+4)*2).decode('utf-16-le')
    expected = text.split(']',1)[0]
    assert result == expected and m.r(parser+12) == len(expected)
    return {'input_line_after_open_bracket':text,'scanned_header':result,'consumed':m.r(parser+12)}


def close_stack(depth, start):
    m=Machine(); parser=0x2000000;table=0x2001000;cap=5
    m.w(parser+0x48,table);m.w(parser+0x4c,cap);m.w(parser+0x50,start);m.w(parser+0x54,depth)
    expected=[]
    for i in range(depth):
        frame=0x2002000+i*0x40;node=0x2003000+i*0x100
        m.w(table+4*((start+i)%cap),frame);m.w(frame,node);m.w(frame+12,i+3)
        expected.insert(0,node)
    observed=[]
    def write(u, access, address, size, value, user):
        if address==parser: observed.append(value)
    m.u.hook_add(UC_HOOK_MEM_WRITE,write)
    m.call(0x5baeb0,parser,[],0)
    assert observed==expected and m.r(parser+0x54)==0
    if depth: assert m.r(parser+0x50)==0
    return {'depth':depth,'ring_start':start,'restored_parent_offsets':[x-0x2003000 for x in observed],'remaining':0}


def child(flags, destination_template, present):
    m=Machine();source=0x2000000;dest=0x2001000;existing=0x2002000;name=0x2003000
    m.u.mem_write(name,'BodyComponent\0'.encode('utf-16-le'));m.w(source,name);m.w(source+0x20,flags);m.w(dest+0x20,int(destination_template))
    events=[]
    m.callbacks[0x5b42c0]=lambda:m.ret(existing if present else 0,4)
    def fill(): events.append('fill_missing_fields_in_existing_child');m.ret(pop=8)
    def clone(): events.append('clone_missing_child');m.ret(pop=12)
    m.callbacks[0x5b57c0]=fill;m.callbacks[0x5b5fc0]=clone
    m.call(0x5b6190,0,[source,dest,0,0],16)
    expected=['fill_missing_fields_in_existing_child'] if present else (['clone_missing_child'] if destination_template or not flags&2 else [])
    assert events==expected
    return {'optional':bool(flags&2),'destination_is_template':destination_template,'child_present':present,'events':events}


def field_copy(destination_state):
    m=Machine();source=0x2000000;dest=0x2001000;table=0x2002000;entry=0x2003000;field=0x2004000;vtable=0x2005000;name=0x2006000;dest_entry=0x2007000
    m.w(source+0x40,table);m.w(source+0x44,1);m.w(table,entry);m.w(entry,field)
    m.w(field,vtable);m.w(field+4,name);m.w(vtable+0x50,0x301f020)
    m.u.mem_write(name,'health\0'.encode('utf-16-le'));m.w(name-4,100)
    m.w(dest_entry,0 if destination_state=='empty_entry' else field+0x100)
    events=[]
    def string_ref():
        m.w(m.u.reg_read(UC_X86_REG_ECX),m.arg());m.ret(pop=4)
    m.callbacks[0x401b00]=string_ref
    m.callbacks[0x497220]=lambda:m.ret(0 if destination_state=='missing' else dest_entry,4)
    def clone():events.append('clone_default');m.ret(field+0x200)
    def insert():events.append('insert_default');assert m.arg()==field+0x200;m.ret(pop=4)
    m.callbacks[0x301f020]=clone;m.callbacks[0x5b3f00]=insert
    m.call(0x5b57c0,source,[dest,0],8)
    assert events==([] if destination_state=='present' else ['clone_default','insert_default'])
    return {'destination_field':destination_state,'events':events}


def main():
    rows={'header':[header(s) for s in ['Thing Template=AABuilding]','Thing Template=AABuilding','Decal Template=DecalWidget','Template Derived Inherit=Base','Thing Template=Base] trailing']],
          'eof_stack':[close_stack(d,s) for d,s in [(0,0),(1,0),(3,0),(3,4)]],
          'unique_child':[child(f,t,p) for f in [0,2] for t in [False,True] for p in [False,True]],
          'field_copy':[field_copy(s) for s in ['missing','empty_entry','present']]}
    ranges={}
    for name,a,z in [('header',0x5bde30,0x5bdf7e),('eof',0x5baeb0,0x5bb050),('file_eof_call',0x5bd080,0x5bd290),('child_merge',0x5b6190,0x5b62c3),('field_merge',0x5b5952,0x5b5a4d),('field_casefold_lookup',0x497220,0x497287),('node_casefold_lookup',0x4247c0,0x424827)]:
        b=native.range(a,z-a);assert b==v14.range(a,z-a)
        ranges[name]={'start':hex(a),'end':hex(z),'sha256':H(b)}
    report={'schema':1,'passed':True,'original_sha256':H(native.b),'v14_sha256':H(v14.b),'cases':rows,'native_cases':sum(map(len,rows.values())),'ranges':ranges,'generator_sha256':H(Path(__file__).read_bytes()),
            'scope':['Header scans execute actual original function with allocation/retain/trim boundaries; block comments excluded from fixture.','EOF executes complete native stack drain for plain non-language nodes with no template-finalizer callback.','Child merge executes original decision function; name lookup and recursive copy/clone are boundary sinks.','Field copy executes original whole5B57C0 and hash iteration; destination lookup, string retain, type clone and insert are boundary seams. No expression/type evaluator claimed.']}
    p=Path(__file__).with_suffix('.json');p.write_text(json.dumps(report,indent=2)+'\n');print(p,H(p.read_bytes()),report['native_cases'])
if __name__=='__main__': main()
