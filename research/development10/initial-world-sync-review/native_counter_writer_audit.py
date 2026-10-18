"""Independent bounded counter writer audit; no whole load/constructor claim.

Execute only the exact Area/Camera fresh InitCreate counter intervals and the
separate generation leaves. Pin saved derived constructors/Init bodies and
queue-item constructor/load windows separately as inspected static evidence.
"""
from pathlib import Path
import sys, json, hashlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as n
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
from unicorn.x86_const import UC_X86_REG_ECX, UC_X86_REG_ESP

WINDOWS = [
    ('saved_area_constructor',0x4db6f0,0x4db79c),
    ('saved_camera_constructor',0x4dbf40,0x4dbf8d),
    ('saved_area_init_null',0x4db200,0x4db243),
    ('saved_camera_init_null',0x4dbcf0,0x4dbd11),
    ('fresh_area_initcreate_prefix',0x4db890,0x4db8e4),
    ('fresh_camera_initcreate_prefix',0x4dbfb0,0x4dc004),
    ('queue_load_factory_prefix',0x58a470,0x58a507),
    ('queue_generic_constructor',0x589920,0x5899d0),
    ('camera_generation_leaf',0x4f0340,0x4f0347),
    ('area_generation_leaf',0x4f0350,0x4f0357),
]

def main():
    windows=[]; decoder=Cs(CS_ARCH_X86,CS_MODE_32)
    for name,start,end in WINDOWS:
        raw=n.original.range(start,end-start)
        assert raw==n.modified.range(start,end-start)
        windows.append(dict(name=name,start=hex(start),end_exclusive=hex(end),bytes=raw.hex(),
            instructions=[dict(va=hex(i.address),mnemonic=i.mnemonic,operands=i.op_str)
                for i in decoder.disasm(raw,start)]))
    rows=[]; pins={}; original=n.original
    for image in ('original','v14'):
        n.original=original if image=='original' else n.modified
        for pc in n.PRECISIONS:
            for before in (0,0x12345678,0xffffffff):
                for name,start,end,offset in [('area',0x4db8c9,0x4db8e4,0x194),
                        ('camera',0x4dbfe9,0x4dc004,0x184)]:
                    m=n.Machine(pc); world=m.alloc(0x300)
                    m.w(0xb36534,world);m.w(world+offset,before)
                    m.u.reg_write(UC_X86_REG_ESP,n.STACK)
                    m.u.emu_start(start,end,count=100)
                    after=m.r(world+offset); assert after==(before+1)&0xffffffff
                    pins.update({a:b for a,b in m.visited.items() if a!=end})
                    rows.append(dict(image=image,precision=pc,operation=name+'_fresh_increment',before=before,after=after))
                for name,address,offset in [('camera',0x4f0340,0x180),('area',0x4f0350,0x190)]:
                    m=n.Machine(pc);world=m.alloc(0x300);m.w(world+offset,before)
                    m.call(address,world,[])
                    after=m.r(world+offset);assert after==(before+1)&0xffffffff
                    pins.update({a:b for a,b in m.visited.items() if a!=n.STOP})
                    rows.append(dict(image=image,precision=pc,operation=name+'_generation',before=before,after=after))
    n.original=original
    for a,b in pins.items():assert original.range(a,len(b))==n.modified.range(a,len(b))==b
    doc=dict(scope=__doc__,cases=rows,static_windows=windows,
        executed_pins=[dict(va=hex(a),bytes=b.hex())for a,b in sorted(pins.items())])
    path=HERE/'native-counter-writer-audit.json'
    if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
    else:assert json.loads(path.read_text())==doc
    print(json.dumps(dict(rows=len(rows),pins=len(pins),static_windows=len(windows),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
