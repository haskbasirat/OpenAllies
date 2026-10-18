"""Execute untouched 4EBE50 -> 4EE700/4EFA40/4EF6A0(NULL)/4F0460.

Actual Object/Actor/Kingdom vtables and +4/+C/+8C leaf bodies execute. Spatial,
placement, snap, blocking, footprint, list and visibility functions are explicit
mandatory-service seams, never claimed implementations. Failure probes stop at
those services (not native exception/unwind proof). Both PE images are mapped
separately; all executed native instructions must be byte-identical.
"""
from pathlib import Path
import sys, json, struct, hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'))
import native_machine as nm
from unicorn import UC_HOOK_CODE, UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
H=lambda b:hashlib.sha256(b).hexdigest()
class Dependency(Exception):pass
def run(c,image,pc):
    m=nm.Machine(pc)
    old=(nm.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
    if new>old:m.u.mem_map(image.base+old,new-old)
    for _,rva,size,off in image.sections:m.u.mem_write(image.base+rva,image.b[off:off+size])
    obj=m.alloc(0x200);definition=m.alloc(0x600);world=m.alloc(0x800);terrain=m.alloc(0x100)
    config=m.alloc(0x200) if c['mode'] is not None else 0
    m.w(obj,0xa2a2a0 if c.get('plain_object') else (0xa299d8 if c['kingdom'] else 0xa273f0));m.w(obj+4,definition)
    m.w(0xb36534,world);m.w(world+0x30,terrain);m.w(world+0x28,config)
    if config:m.w(config+0xa8,c['mode'])
    m.w(terrain+4,c['shift']);m.w(terrain+8,c['tiles'][0])
    m.w(definition+0x24,c['height']);m.w(definition+8,m.cstring('fixture_object'))
    m.w(definition+0x10,m.cstring('Fixture Object'))
    def setshape(s):
        for off,b in zip([0,4,0x10,0x14],s):m.w(definition+0x38+off,b)
    setshape(c['shapes'][0])
    for off,b in [(8,c['flags']),(0x38,0x12345678),(0x3c,c['scale']),(0x44,0x99887766),
                  (0x48,0x55667788),(0x4c,0xa1b2c3d4),(0x58,c['instances']),(0x5c,0x55667700)]:m.w(obj+off,b)
    for i,b in enumerate(c['position']):m.w(obj+0x20+i*4,b)
    events=[];stores=[];tile_at=0;shape_at=0
    def position():return [m.r(obj+0x20+i*4) for i in range(3)]
    def fail(name):
        if c.get('failure')==name:raise Dependency(name)
    def ev(name,*args):events.append([name,*args]);fail(name)
    def ecx():return m.u.reg_read(UC_X86_REG_ECX)
    def seam(address,callback):m.seam(address,callback)
    def fpreturn(bits):
        code=m.alloc(32);data=code+16;m.u.mem_write(data,struct.pack('<Q',bits))
        m.u.mem_write(code,b'\xdd\x05'+struct.pack('<I',data)+b'\xc2\x08\x00')
        m.u.reg_write(UC_X86_REG_EIP,code)
    def snap(_):
        assert ecx()==definition+0x38 and m.arg(0)==obj+0x20
        ev('snap',m.arg(1),position())
        for i,b in enumerate(c['snapped']):m.w(obj+0x20+i*4,b)
        m.ret(8)
    def placement(_):
        assert ecx()==world and [m.arg(i) for i in range(5)]==[obj,0,0,0,0]
        ev('placement',position());m.ret(20,c['placement'])
    def diagnostic(_):ev('placement_failure',c['placement'],position());m.ret(0)
    def warning(_):ev('map_warning',position());m.ret(0)
    def index(_):
        assert ecx()==obj
        ev('index')
        if 'index_flags' in c:m.w(obj+8,c['index_flags'])
        m.ret()
    def terrain_height(_):
        assert ecx()==world and m.arg(0)==obj+0x20
        ev('height',m.arg(1),position());fpreturn(c['terrain_f64'])
    def footprint(_):
        ev('footprint')
        if 'footprint_position' in c:
            for i,b in enumerate(c['footprint_position']):m.w(obj+0x20+i*4,b)
        m.ret()
    def worldlist(_):
        assert ecx()==world and m.arg(0)==obj
        ev('list')
        if 'list_instances' in c:m.w(obj+0x58,c['list_instances'])
        m.ret(4)
    def retain(_):ev('retain');m.ret(0,c['retain'])
    def clear(_):
        ev('clear');m.w(obj+0x58,0);m.w(obj+0x5c,0);m.ret()
    seam(0x46ee00,snap);seam(0x4f4590,placement);seam(0x404f60,diagnostic)
    seam(0x5a1040,warning);seam(0x4ee4c0,index);seam(0x4f0e50,terrain_height)
    seam(0x4eb6e0,lambda _: (ev('blocking'),m.ret()))
    seam(0x4ea150,footprint);seam(0x4f1f70,worldlist);seam(0x4eb490,retain)
    seam(0x4ebb80,clear);seam(0x4ebc70,lambda _:(ev('visibility'),m.ret()))
    seam(0x404ea0,lambda _:(ev('thickness_warning'),m.ret()))
    # Native 401560's CString ref transport requires only reference storage.
    def copy_ref(_):
        header=m.arg(0);m.w(header+12,m.r(header+12)+1);m.ret(0,header)
    seam(0x4012c0,copy_ref)
    def observe(u,addr,size,_):
        nonlocal shape_at
        if addr==0x4d1e40:ev('radius')
        if addr==0x4ea010 and m.r(m.esp())!=0x4d1e45:
            s=c['shapes'][min(shape_at,len(c['shapes'])-1)];shape_at+=1;setshape(s);ev('shape',s)
        if addr==0x4ea3d0:ev('has_instances',m.r(obj+0x58))
        if addr==0x4f0460:ev('mode',c['mode'])
    def reads(u,access,address,size,value,_):
        nonlocal tile_at
        if address==terrain+8:
            b=c['tiles'][min(tile_at,len(c['tiles'])-1)];tile_at+=1;m.w(address,b);ev('tile',b)
        if address==terrain+4:
            ev('shift',c['shift'])
            if 'shift_position' in c:
                for i,b in enumerate(c['shift_position']):m.w(obj+0x20+i*4,b)
    def writes(u,access,address,size,value,_):
        if obj<=address<obj+0x200:stores.append([address-obj,size,value&((1<<(size*8))-1)])
    m.u.hook_add(UC_HOOK_CODE,observe);m.u.hook_add(UC_HOOK_MEM_READ,reads)
    m.u.hook_add(UC_HOOK_MEM_WRITE,writes)
    result=None;outcome='ok'
    try:result=bool(m.call(0x4ebe50,obj,[0],4)&255)
    except Dependency as error:outcome=str(error)
    assert m.outcome is None,m.outcome
    assert not m.null_accesses,m.null_accesses
    fields=[8,0x20,0x24,0x28,0x38,0x3c,0x44,0x48,0x4c,0x58,0x5c]
    return {'outcome':outcome,'result':result,'words':[[o,m.r(obj+o)] for o in fields],
            'events':events,'stores':stores}
def cases():
    base={'kingdom':False,'flags':0,'mode':0,'shift':2,'tiles':[0x40800000],
          'height':0x3f800001,'scale':0x3fc00001,'shapes':[[2,0x40800000,0x3e800000,0x3f000000]],
          'position':[0xc1233333,0x417ccccd,0x40400000],'snapped':[0xc1000000,0x41800000],
          'placement':0,'terrain_f64':0x3ff0000010000000,'instances':0,'retain':False}
    yield dict(base,label='base')
    for flags in [0,0x20010400,1<<12]:
        yield dict(base,label='plain_object_'+hex(flags),plain_object=True,flags=flags,instances=3)
    for flags in [1<<12,1<<21,(1<<12)|(1<<21),1<<16,0x20000400,0x20010400,0x20000000,0x400]:
        yield dict(base,label='flags_'+hex(flags),flags=flags)
    for n in [0,1,2,0xffffffff,0x80000000,0x7fffffff]:
        for retain in [False,True]:yield dict(base,label=f'instances_{n}_{retain}',instances=n,retain=retain,kingdom=True)
    for code in [1,2,3,4,5,6,7,8,0xffffffff]:
        for mode in [None,0,1,2]:yield dict(base,label=f'placement_{code}_{mode}',placement=code,mode=mode)
    for shift in [0,31,32,33,255]:yield dict(base,label=f'shift_{shift}',shift=shift)
    yield dict(base,label='shift_changes_position_after_capture',shift_position=[0x42800000,0x43000000,0x43400000])
    yield dict(base,label='footprint_changes_position',footprint_position=[0x42800000,0x43000000,0x43400000])
    for flags in [0,1<<12,1<<21,(1<<12)|(1<<21)]:yield dict(base,label=f'index_flags_{flags}',index_flags=flags,flags=1<<12)
    yield dict(base,label='list_creates_instances',list_instances=3)
    yield dict(base,label='list_removes_instances',list_instances=0,instances=3)
    for tiles in [[0x41000000,0x40800000],[0x40800000,0x42800000],[0x40ffffff],[0xc0800000]]:
        yield dict(base,label='tiles_'+str(tiles),flags=0x20010400,tiles=tiles)
    for shapes in [[[1,0x40800000,0,0]],[[3,0x40800000,0x40800000,0x3d800000]],
                   [[2,0x40800000,0,0],[1,0x3d800000,0x40800000,0x40800000]],
                   [[2,0x40800000,0,0],[2,0x40800000,0x40000000,0x40000000]]]:
        yield dict(base,label='shapes_'+str(shapes),flags=0x20000400,shapes=shapes)
    for bits in [0x8000000000000000,0x3ff0000030000000,0x0000000000000001]:
        yield dict(base,label='terrain_'+hex(bits),terrain_f64=bits)
    for name in ['radius','shape','thickness_warning','snap','placement','placement_failure','mode',
                 'map_warning','index','height','blocking','footprint','shift','list','has_instances',
                 'retain','clear','visibility']:
        yield dict(base,label='dependency_'+name,flags=0x20010400,instances=2,placement=1,mode=2,failure=name)
def main():
    rows=[]
    for pc in ['Single24','Double53','Extended64']:
        for c in cases():
            a=run(c,nm.P,pc);b=run(c,nm.V,pc);assert a==b,(pc,c['label'],a,b)
            rows.append({'precision':pc,'input':c,'output':a})
    cs=Cs(CS_ARCH_X86,CS_MODE_32)
    instructions=[{'address':hex(a),'bytes':b,'asm':'; '.join(i.mnemonic+' '+i.op_str for i in cs.disasm(bytes.fromhex(b),a))} for a,b in sorted(nm.VISITED.items())]
    vtables=[]
    for name,vt in [('Object',0xa2a2a0),('Actor',0xa273f0),('Kingdom',0xa299d8)]:
        entries=[]
        for offset in [4,12,0x8c,0xa8,0xb0,0xe8]:
            a=nm.P.range(vt+offset,4);b=nm.V.range(vt+offset,4);assert a==b
            entries.append({'offset':hex(offset),'target':hex(struct.unpack('<I',a)[0]),'bytes':a.hex()})
        vtables.append({'class':name,'address':hex(vt),'entries':entries})
    out={'schema':1,'scope':__doc__,'original_sha256':H(nm.P.b),'v14_sha256':H(nm.V.b),'vtables':vtables,
         'producer_sha256':H(Path(__file__).read_bytes()),'harness_sha256':H(Path(nm.__file__).read_bytes()),
         'native_executions':len(rows)*2,'cases':rows,'instructions':instructions}
    path=HERE/'native-object-init-null.json'
    if '--write' in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert json.loads(path.read_text())==out,'fixture changed'
    print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'instructions':len(instructions),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
