"""Bounded real TerrainData+5C constructor/type-loader slices, embedded GMap+5C
constructor identity and actual50C310 notification gates. Metadata getters and
positive cache-builder/attached-view children are named transport seams; no
full source loader, map ctor, RMG/editor cache construction or renderer claim.
"""
from pathlib import Path
import sys,json,itertools,struct
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h
from unicorn.x86_const import *
P,V=h.P,h.V
class Halt(Exception):pass

def segment(m,start,stops):
    def stop(machine):raise Halt()
    for a in stops:m.probes[a]=stop
    m.u.reg_write(UC_X86_REG_ESP,h.STACK);m.w(h.STACK,h.STOP)
    try:m.u.emu_start(start,h.STOP,count=2000000)
    except Halt:pass
    assert m.u.reg_read(UC_X86_REG_EIP)in stops
    for a in stops:m.probes.pop(a,None)
    assert not m.null_accesses

def row(pc,native_type,bits):
    m=h.Machine(b'',pc);obj=m.alloc(0xa0,0xa5)
    m.u.reg_write(UC_X86_REG_ESI,obj)
    segment(m,0x4b7e80,[0x4b7efa]);ctor=m.r(obj+0x5c);assert ctor==0
    m.w(obj+0x1c,native_type);node=m.alloc(4);m.u.reg_write(UC_X86_REG_EDI,node)
    events=[]
    def getter(mm):
        events.append(dict(this=mm.u.reg_read(UC_X86_REG_ECX)==node,name=mm.read_wide(mm.arg())))
        value=struct.unpack('<f',struct.pack('<I',bits))[0]
        mm.fld_stub(h.f64(value))
    m.seam(0x5b5130,getter)
    segment(m,0x4b8955,[0x4b8975,0x4b8bbc,0x4b8bf8])
    return dict(constructor=ctor,value=m.r(obj+0x5c),getters=events)

def notification(pc,phase,dirty,attached):
    m=h.Machine(b'',pc);world=m.alloc(0x134);map_=m.alloc(0xf0,0xa5);manager=m.alloc(8)
    m.w(0xb36534,world);m.w(world+0x30,map_);m.w(world+0xd8,phase);m.w(world+0x130,manager)
    #Actual parent ctor's LEA ECX,GMap+5C then CALL55AAF0.
    m.u.reg_write(UC_X86_REG_ESI,map_);m.u.reg_write(UC_X86_REG_EBX,0)
    segment(m,0x5565d5,[0x5565e1]);assert m.b(map_+0xec)==0
    if dirty:
        #The actual builder suffix retains its field write; builder body is a
        #named seam, not a claimed completed RMG/editor build.
        m.seam(0x55d1f0,lambda mm:mm.ret());m.call(0x55d790,map_+0x5c,[])
    m.w(manager+4,0x1234 if attached else 0)
    events=[]
    m.probes[0x55a550]=lambda mm:events.append('55A550')
    def deliver(mm):events.append(dict(child='43B240',raw_args=[mm.arg(i)for i in range(4)],changed_low_byte=mm.arg(3)&255,view=mm.u.reg_read(UC_X86_REG_ECX)));mm.ret(16)
    m.seam(0x43b240,deliver)
    m.call(0x50c310,manager,[0x3f800000,0x40000000,0x40400000])
    assert not m.null_accesses
    return dict(ctor_dirty=0,dirty_after=m.b(map_+0xec),events=events)

def main():
    h.VISITED.clear();rows=[]
    for pc,t,bits in itertools.product(h.CW,[0,1,2],[0,0x3e800000,0x404ae148,0xc0000000]):
        pair=[]
        for pe in [P,V]:h.P=pe;pair.append(row(pc,t,bits))
        assert pair[0]==pair[1];assert pair[0]['value']==(bits if t else 0)
        rows.append(dict(precision=pc,type=t,source_value=bits,native=pair[0]))
    notifications=[]
    for pc,phase,dirty,attached in itertools.product(h.CW,[0,6],[False,True],[False,True]):
        pair=[]
        for pe in [P,V]:h.P=pe;pair.append(notification(pc,phase,dirty,attached))
        assert pair[0]==pair[1]
        notifications.append(dict(precision=pc,phase=phase,dirty=dirty,attached=attached,native=pair[0]))
    h.P=P
    stopped={0x4b7efa,0x4b8975,0x4b8bbc,0x4b8bf8,0x5565e1}
    seams={0x5b5130,0x6f7810,0x6f7430,0x55d1f0,0x43b240}
    pins={hex(a):b for a,b in sorted(h.VISITED.items())if a not in stopped|seams}
    out=dict(schema=1,scope=__doc__,rows=rows,notifications=notifications,executions=2*(len(rows)+len(notifications)),pins=pins,
      boundary_pins={hex(a):h.VISITED[a]for a in sorted(stopped)if a in h.VISITED},
      seams=['5B5130 typed max_slope getter returns explicit binary32 source value through real FLD/RET4 stub','6F7810/6F7430 allocator/free transport','55D1F0 positive cache-builder body transported; only real55D790 flag suffix claimed','43B240 attached view child records args then returns; no renderer completion claim'],
      producer_sha256=h.H(Path(__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()))
    path=HERE/'native-brush-metadata.json'
    if '--write'in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert out==json.loads(path.read_text())
    print(json.dumps(dict(rows=len(rows),notifications=len(notifications),executions=out['executions'],pins=len(pins),sha256=h.H(path.read_bytes()))))
if __name__=='__main__':main()

