"""Actual4CFEA0/554CC0/554330/54EF10 with actual ordinary getters.

Real leaf515620 executes for flags0/1/2/64 and type2 with no Detection. Point words are explicit
controlled input, not startup provenance. No iterator/registration body seams.
"""
from pathlib import Path
import hashlib
import itertools
import json
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE

H=lambda b:hashlib.sha256(b).hexdigest()


def run(c,pc,image):
    m=h.Machine(pc)
    if image is h.V:
        old=(h.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
        if new>old:m.u.mem_map(image.base+old,new-old)
        m.u.mem_write(image.base,image.b[:4096])
        for vs,r,n,o in image.sections:
            if n:m.u.mem_write(image.base+r,image.b[o:o+min(n,max(vs,n))])
    obj,definition,world,map_object,session=[m.alloc(n)for n in(0x300,0x600,0x100,0x100,0x100)]
    tiles=m.alloc(8*6*48)
    m.w32(0xb36534,world);m.w32(world+0x30,map_object);m.w32(world+0x48,c['world48'])
    m.w32(0xb36550,session);m.w32(session+0xe4,c['application_e4'])
    m.w32(map_object+4,2,h.f2b(4),h.f2b(32),h.f2b(24))
    m.w32(map_object+0x1c,8,6);m.w32(map_object+0x28,tiles)
    m.w32(0xb3a8c4,*c['point'])
    m.w32(obj,0xa273f0,definition,c['flags']);m.w32(definition,0xa05540)
    m.w32(definition+0x38,*c['shape'])
    m.w32(obj+0x20,*c['position'],0,*c['facing'],0);m.w32(obj+0x44,*c['cached'])
    leaves={};reverse={}
    for t in range(48):
        leaf=m.alloc(20);leaves[t]=leaf;reverse[leaf]=t
        m.w32(leaf,0xa323e8,1,obj,c['leaf_flags'],0)
        m.w32(tiles+t*48+4,leaf)
    tile_calls=[];leaf_calls=[]
    def observe(u,address,size,data):
        if address==0x54ef10:
            assert m.arg()==obj
            tile_calls.append((m.ecx()-tiles)//48)
        if address==0x515620:
            assert m.arg()==obj
            leaf_calls.append(reverse[m.ecx()])
    m.u.hook_add(UC_HOOK_CODE,observe)
    m.call(0x4cfea0,obj)
    assert m.fpu_top()==0
    enabled=c['application_e4']!=3 or c['world48']!=0
    assert leaf_calls==(tile_calls if enabled else [])
    return {'tile_calls':tile_calls,'leaf_calls':leaf_calls,'flags_after':m.r32(obj+8)}


def inputs():
    shapes=[
        ([0,0,0,0,0,0],0x20000000,[0,0],[0,0],[0,0xbf800000]),
        ([0,0,0,0,0,0],0x20000000,[4,2],[h.f2b(10.3),h.f2b(12.8)],[0,0xbf800000]),
        ([1,h.f2b(3.2),0,0,0,0],0x20000400,[2,3],[h.f2b(10.3),h.f2b(12.8)],[0,0xbf800000]),
        ([3,h.f2b(5),h.f2b(-4),h.f2b(-3),h.f2b(4),h.f2b(3)],0x20000400,[2,3],[h.f2b(10.3),h.f2b(12.8)],[h.f2b(.6),h.f2b(.8)]),
        ([1,h.f2b(3.2),0,0,0,0],0x20000000,[-2,99],[h.f2b(-1),h.f2b(10)],[0,0xbf800000]),
    ]
    for shape,flags,cached,position,facing in shapes:
        for app,world,leaf in itertools.product([0,3],[0,1],[0,1,2,4,64]):
            yield dict(shape=shape,flags=flags,cached=cached,position=position,facing=facing,
                       application_e4=app,world48=world,leaf_flags=leaf,point=[0]*6)


def main():
    rows=[]
    for pc in h.PRECISION:
        for c in inputs():
            a=run(c,pc,h.P);b=run(c,pc,h.V);assert a==b
            rows.append({'precision':pc,'input':c,'output':a})
    pins=[{'va':hex(a),'bytes':raw}for a,raw in sorted(h.VISITED.items())]
    assert all(int(p['va'],16)not in h.ALLOCATORS for p in pins)
    doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),
         'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'cases':rows,
         'executed_address_byte_pins':pins,'seams':{}}
    raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-grid-registration.json'
    if '--write'in sys.argv:path.write_bytes(raw)
    else:assert json.loads(path.read_text())==doc
    print(json.dumps({'rows':len(rows),'fresh_original_v14_machines':len(rows)*2,
                      'executed_pins':len(pins),'sha256':H(raw),'immutable':'--write'not in sys.argv}))


if __name__=='__main__':main()
