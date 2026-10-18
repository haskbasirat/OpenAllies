"""Original/v14 Settlement shared-shape prefix; no fit/trig/completion claim."""
from pathlib import Path
import json, random, struct, sys
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
from native_machine import Machine, original, modified, sha, STACK, PRECISIONS
from unicorn.x86_const import *
HERE = Path(__file__).resolve().parent

def remap(m, image):
    old=(original.image_size+0xfff)&~0xfff; new=(image.image_size+0xfff)&~0xfff
    if new > old: m.u.mem_map(original.base+old,new-old)
    for _,rva,size,off in image.sections: m.u.mem_write(image.base+rva,image.b[off:off+size])

def execute(m, start, end, registers):
    m.u.reg_write(UC_X86_REG_ESP, STACK)
    m.u.reg_write(UC_X86_REG_FPCW, PRECISIONS[m.precision])
    for reg,value in registers.items(): m.u.reg_write(reg,value)
    errors=[]
    def fail(z): errors.append(z.arg(1)); z.u.emu_stop()
    m.seam(0x410c40, fail)
    m.u.emu_start(start,end,count=10000)
    assert not m.null_accesses, [(hex(a),hex(b),c) for a,b,c in m.null_accesses]
    assert m.u.reg_read(UC_X86_REG_EIP)==end or errors
    m.verify_v14()
    return None if not errors else hex(errors[0])

def shapes(index):
    return [[0x10000000+index*100+k for k in range(6)],
            [0x20000000+index*100+k for k in range(6)]]

def main():
    rows=[]; wall_rows=[]; radius_rows=[]; pins={}
    rng=random.Random(0x4640f0)
    layouts=[([0,1,2],[3,4,5]),([0,1,2],[1,2,0]),([0,1,2],[2,0,1]),
             ([0,0,1],[1,2,0]),([0,1,2],[0,1,2]),([0,1,2],[1,1,1])]
    layouts += [([rng.randrange(6) for _ in range(3)], [rng.randrange(6) for _ in range(3)]) for _ in range(24)]
    for image_name,image in [('original',original),('v14',modified)]:
      for pc in PRECISIONS:
        for index,(source,target) in enumerate(layouts):
          for mode in ['level','placeholder']:
            m=Machine(pc); remap(m,image)
            actors=[m.alloc(0x500) for _ in range(6)]
            for i,a in enumerate(actors):
              for shape,offset in zip(shapes(i),[0x38,0x50]):
                for j,value in enumerate(shape):m.w(a+offset+4*j,value)
              m.w(a+0x174,1<<13)
            owner=m.alloc(0x500); first=m.alloc(0x30); current=m.alloc(0x30); levels=m.alloc(0x18)
            for i,a in enumerate(source): m.w(first+0x18+4*i,actors[a])
            for i,a in enumerate(target):m.w(current+0x18+4*i,actors[a]);m.w(owner+0x49c+4*i,actors[a])
            m.w(owner+0x464,levels);m.w(levels+8,first)
            record=levels+0xc;m.w(record+4,actors[5]);m.w(record+8,current)
            # One row compares the exact same WallSet pointer, so the native
            # Level identity branch must skip even if the fixture refs differ.
            same=(index==4 and mode=='level')
            if same:m.w(record+8,first)
            if mode=='level':
              err=execute(m,0x46450b,0x46469a,{UC_X86_REG_EBP:owner,UC_X86_REG_EBX:record})
            else:
              err=execute(m,0x4647b1,0x464909,{UC_X86_REG_EBP:owner})
            assert err is None
            actual=[]
            for a in actors:
              actual.append({'blocking':[m.r(a+0x38+4*j) for j in range(6)],
                             'soft':[m.r(a+0x50+4*j) for j in range(6)],
                             'settlement_wall':m.rb(a+0x1a8),
                             'owner':m.r(a+0x430)==owner})
            rows.append({'image':image_name,'precision':pc,'mode':mode,'source':source,'target':target,'same_wallset':same,'initial_shapes':[shapes(i) for i in range(6)],'result':actual})
            pins.update(m.visited)
        # Error prefix: a null/invalid/already owned center must fail before
        # center+430 changes; unknown WallSet fails AFTER the owner write.
        for case in ['null_center','not_structure','already_owned','null_wallset']:
            m=Machine(pc);remap(m,image);a=m.alloc(0x500);owner=m.alloc(0x500);record=m.alloc(0x18);old_owner=m.alloc(0x10)
            m.w(record+4,0 if case=='null_center' else a)
            m.w(a+0x174,0 if case=='not_structure' else 1<<13)
            m.w(a+0x430,old_owner if case=='already_owned' else 0)
            err=execute(m,0x46450b,0x46469a,{UC_X86_REG_EBP:owner,UC_X86_REG_EBX:record})
            assert err is not None
            rows.append({'image':image_name,'precision':pc,'mode':'error','case':case,'error_address':err,'owner_written':m.r(a+0x430)==owner,'old_owner_preserved':m.r(a+0x430)==old_owner})
            pins.update(m.visited)
        # WallSet reference/validation suffix, with ONLY Thing Find as seam.
        for absent in [-1,0,1,2]:
          for missing_wall in [-1,0,1,2]:
            for corner_targetable in [False,True]:
              m=Machine(pc);remap(m,image);wall=m.alloc(0x30);actors=[m.alloc(0x500) for _ in range(3)]
              game=m.alloc(0x20);m.w(0xb36530,game);m.w(game+0xc,m.alloc(0x20))
              refs=[m.alloc(4) for _ in range(3)];calls=[]
              for i,a in enumerate(actors):m.w(a+0x174,0 if i==missing_wall else 1<<12)
              m.w(actors[1]+0x1c,(1<<14) if corner_targetable else 0)
              def lookup(z):
                i=refs.index(z.arg(1));assert z.arg(2)==0x2b;calls.append(i);z.ret(0 if i==absent else actors[i],pop=8)
              m.seam(0x4bdba0,lookup)
              # At47DFB9 the first two arguments were already pushed.
              m.w(STACK,refs[0]);m.w(STACK+4,0x2b)
              err=execute(m,0x47dfb9,0x47e06d,{UC_X86_REG_ESI:wall,UC_X86_REG_EBX:refs[0],UC_X86_REG_EBP:refs[1],UC_X86_REG_EDI:refs[2]})
              assert calls==[0,1,2]
              wall_rows.append({'image':image_name,'precision':pc,'absent':absent,'missing_wall':missing_wall,'corner_targetable':corner_targetable,'error_address':err,'stored_references':[None if m.r(wall+0x18+4*i)==0 else actors.index(m.r(wall+0x18+4*i)) for i in range(3)]})
              pins.update(m.visited)
        for diameter in [0,0x80000000,0x3f800000,0x40000001,0x42480000,0x00000001,0x00800001,0x7f7fffff,0xbf800000]:
            m=Machine(pc);remap(m,image);owner=m.alloc(0x500);arg=m.alloc(4);m.w(arg,diameter)
            stub=m.alloc(16);m.u.mem_write(stub,b'\xd9\x05'+struct.pack('<I',arg)+b'\xe9'+struct.pack('<i',0x46491c-(stub+11)))
            err=execute(m,stub,0x46492f,{UC_X86_REG_EBP:owner})
            assert err is None
            radius_rows.append({'image':image_name,'precision':pc,'diameter':diameter,'initial_radius':m.r(owner+0x478)})
            pins.update(m.visited)
    out={'schema':1,'passed':True,'scope':'Bounded native instruction prefixes only. No460940, trigonometry, CRT fitting, parser, allocator, complete4640F0, completed478 or1E4 proof.','original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'prefix_cases':rows,'wall_set_cases':wall_rows,'initial_radius_cases':radius_rows,'instructions':[{'address':hex(a),'bytes':b.hex()} for a,b in sorted(pins.items())]}
    (HERE/'native-prefix.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
    print('passed',len(rows),'shared-prefix',len(wall_rows),'WallSet',len(radius_rows),'initial-radius',len(pins),'pins')

if __name__=='__main__':main()
