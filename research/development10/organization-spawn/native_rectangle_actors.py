"""Closed empty552EE0 /558040(mode0)/558580 scan on real native tile lists.
Original map insertion, circle descriptor, iterator, exclusion, B4 and cleanup
execute unchanged. Only malloc/free are seamed. No circle intersection filter.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h

def shape(kind,radius,hx=1.25,hy=.75):
    return dict(kind=kind,radius_bits=h.f2b(radius),bounds=[h.f2b(x) for x in [-hx,-hy,hx,hy]])

def run(precision,scenario):
    m=h.Machine(precision)
    m.seams[0x6f7810]=lambda mm:mm.ret(0,mm.alloc(mm.arg()))
    m.seams[0x6f7430]=lambda mm:mm.ret()
    grid=dict(dimensions=[8,7],coordinate_shift=1,cell_size_bits=h.f2b(2.))
    state=m.alloc(0x100);tiles=m.alloc(56*0x30);ids=m.alloc(0x60004)
    world=m.alloc(0x100);points=m.alloc(0x100);point=m.alloc(0x20)
    m.w32(0xb36534,world);m.w32(world+0x30,state);m.w32(0xb364c0,ids)
    m.w32(0xb39990,points);m.w32(points+0x64,point);m.w32(point+4,0,h.f2b(-1.))
    extent=[h.f2b(17. if scenario&1 else 16.),h.f2b(15. if scenario&1 else 14.)]
    m.w32(state+4,1,h.f2b(2.),*extent);m.w32(state+0x1c,8,7);m.w32(state+0x28,tiles)
    #Mode0/mask0 must not yield any static tile even with poison flags.
    flags=[0x300 if i%7==scenario%7 else 0x200 for i in range(56)]
    for i,f in enumerate(flags):m.w32(tiles+i*0x30+0x20,f)
    members=[];pointers={}
    for i in range(7):
        key=i+1;actor=m.alloc(0x200);definition=m.alloc(0x500);vtable=m.alloc(0x180)
        m.u.mem_write(vtable,h.P.range(0xa273f0,0x180))
        category=not(scenario&4 and i in [1,5]);m.w32(vtable+0xb4,0x6a71d0 if category else 0x6a71f0)
        flags_word=0x20000400 if i%3 else 0x400
        position=[[1.,1.],[2.,2.],[3.,3.],[5.,5.],[6.5,5.],[9.,9.],[13.,11.]][i]
        facing=[0,h.f2b(-1.)];reference=((i+2)<<16)|key
        blocking=shape(1 if i%2 else 3,1.25);soft=shape(3,3.,2.5,2.5)
        m.w32(actor,vtable,definition,flags_word);m.w32(actor+0x14,reference)
        m.w32(actor+0x20,*[h.f2b(x) for x in position],h.f2b(2.),*facing,0)
        m.w32(definition+0x1c,flags_word)
        for offset,s in [(0x38,blocking),(0x50,soft)]:m.w32(definition+offset,s['kind'],s['radius_bits'],*s['bounds'])
        m.u.mem_write(ids+4+key*2,(i+2).to_bytes(2,'little'));m.w32(ids+0x20004+key*4,actor)
        pose_soft=m.alloc(0x18);pose_block=m.alloc(0x18)
        m.w32(pose_soft,definition+0x50,*facing,*[h.f2b(x) for x in position],0)
        m.w32(pose_block,definition+0x38,*facing,*[h.f2b(x) for x in position],0)
        m.call(0x5536b0,ecx=state,args=(actor,pose_soft,pose_block))
        pointers[key]=actor
        members.append(dict(key=key,reference=reference,actor=dict(definition=key+10,flags=flags_word,
            position=[h.f2b(x) for x in position],facing=facing),height_bits=h.f2b(2.),facing_z_bits=0,
            cached_tile=[0,0],blocking=blocking,soft=soft,target=soft,actor_category=category,wall_present=False))
    premarked=[3] if scenario&8 else []
    for key in premarked:m.w32(pointers[key]+0x10,pointers[key])
    descriptor=m.alloc(0x60);iterator=m.alloc(0xa0)
    xy=[[-1.,5.],[0.,0.],[5.,5.],[6.,6.],[15.9,13.9],[30.,30.]][scenario%6]
    radius=[0.,.00001,1.,2.,5.][scenario%5]
    position=[h.f2b(v) for v in xy]
    m.call(0x552ee0,ecx=descriptor,args=(*position,h.f2b(radius)))
    m.call(0x5581f0,ecx=iterator)
    m.call(0x558040,ecx=iterator,args=(descriptor,0))
    found=[]
    for _ in range(100):
        actor=m.call(0x558580,ecx=iterator)
        if not actor:break
        found.append(m.r32(actor+0x14))
    else:raise AssertionError('iterator failed to finish')
    m.call(0x557fd0,ecx=iterator)
    links={str(k):(k if m.r32(p+0x10)==p else None) for k,p in pointers.items()}
    assert all(m.r32(p+0x10) in (0,p) for p in pointers.values())
    return dict(input=dict(grid=grid,extent_bits=extent,flags=flags,members=members,premarked=premarked,
                           position=position,radius=h.f2b(radius),source_facing=[0,h.f2b(-1.)]),
                native=dict(references=found,links=links))

def main():
    original=h.P;rows=[]
    try:
        for label,image in [('original',original),('v14',h.V)]:
            h.P=image
            for precision in h.PRECISION:
                for scenario in range(24):rows.append(dict(image=label,precision=precision,scenario=scenario,**run(precision,scenario)))
    finally:h.P=original
    half=len(rows)//2
    assert [{k:v for k,v in r.items()if k!='image'}for r in rows[:half]]==[{k:v for k,v in r.items()if k!='image'}for r in rows[half:]]
    doc=dict(scope=__doc__,pairs=half,executions=len(rows),rows=rows,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
        producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        executed_pins=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
    path=HERE/'native-rectangle-actors.json';payload=(json.dumps(doc,indent=1)+'\n').encode()
    if '--check'in sys.argv:assert path.read_bytes()==payload
    else:
        assert not path.exists(),'use --check'
        path.write_bytes(payload)
    print(json.dumps(dict(pairs=half,executions=len(rows),pins=len(h.VISITED),sha256=hashlib.sha256(payload).hexdigest(),result='PASS')))
if __name__=='__main__':main()
