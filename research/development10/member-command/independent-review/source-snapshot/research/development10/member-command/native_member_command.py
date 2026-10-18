"""522890 formation command coordinator +actual4D9E20/4CA760/4CA2D0/529F10.
Geometry queries, zone membership, relocation, member activity/target and
4D9B60 command delivery are declared seams. Both immutable PEs/allPCs.
"""
from pathlib import Path
import hashlib,json,random,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
def cases():
    rng=random.Random(0x522890);out=[]
    for i in range(96):
        origin=[h.f2b(float(i%11)),h.f2b(float(i%7))]
        leader=[h.f2b(float(i%9)),h.f2b(float(i%5))]
        offset=[h.f2b([0.,.125,.5,-.125][i%4]),h.f2b([0.,.25,-.5][i%3])]
        base=leader if i%6 else origin
        dest=[h.f2b(h.b2f(base[a])+h.b2f(offset[a])) for a in range(2)]
        delta=[0.,.49999997,.5,.50000006,2.][i%5]
        out.append(dict(index=i,flag=[0,1,2,127,128,255][i%6],origin=origin,leader=leader,
            offset=offset,slot=i%3,member_position=[h.f2b(h.b2f(dest[0])+delta),dest[1]],
            target=[h.f2b(h.b2f(dest[0])+[0.,.5,2.][i%3]),dest[1]],
            zone=i%5!=0,destination_tile=i%7!=0,member_tile=i%4!=0,
            destination_inside=i%3!=0,member_inside=i%2!=0,active=i%2==0,
            sleep=[rng.getrandbits(32),rng.getrandbits(32)]))
    return out
def run(m,c):
    m.next=h.HEAP+0x1000;calls=[]
    mind=m.alloc(0x300);owner=m.alloc(0x180);org=m.alloc(0x100);member=m.alloc(0x180)
    leader=m.alloc(0x40);offsets=m.alloc(24);element=m.alloc(0x30);denizen=m.alloc(0x40)
    zones=m.alloc(24);zone=m.alloc(0x30);world=m.alloc(0x40);grid=m.alloc(0x40)
    data=m.alloc(0x2a0);registry=m.alloc(39*4)
    m.w32(0xb36530,data);m.w32(data+0x298,registry)
    defs=[m.alloc(0x30) for _ in range(39)]
    for i,d in enumerate(defs):m.w32(registry+i*4,d)
    m.w32(mind+4,owner);m.w32(mind+0x2b4,*c['sleep'])
    m.w32(owner+0x7c,org);m.w32(org+0x40,leader);m.w32(leader+0x20,*c['leader'])
    m.w32(org+0xbc,*c['origin']);m.w32(org+0x78,offsets);m.w32(offsets+c['slot']*8,*c['offset'])
    m.w32(owner+0x148,zones,6 if c['zone'] else 4);m.w32(zones+16,zone)
    m.w32(member+0x20,*c['member_position']);m.w32(member+0x80,element);m.w32(element+0x18,c['slot'])
    m.w32(member+0x70,denizen);vt=m.alloc(0x90);m.w32(denizen,vt);m.w32(vt+0x80,h.STUBS+0x200)
    m.w32(0xb36534,world);m.w32(world+0x30,grid)
    def relocate(mm):calls.append(['relocate',[mm.arg(),mm.arg(1)]]);mm.ret(8)
    m.seams[0x57c680]=relocate
    tiles=[m.alloc(0x30),m.alloc(0x30)];tile_count=[0]
    def tile(mm):
        i=tile_count[0];tile_count[0]+=1;assert i<2
        calls.append(['tile',[mm.arg(),mm.arg(1)]])
        mm.ret(8,tiles[i] if c['destination_tile' if i==0 else 'member_tile'] else 0)
    m.seams[0x5543f0]=tile
    def contains(mm):
        i=tiles.index(mm.ecx());assert mm.arg()==zone;calls.append(['contains',i])
        mm.ret(4,int(c['destination_inside' if i==0 else 'member_inside']))
    m.seams[0x54ef40]=contains
    def active(mm):assert mm.ecx()==member;calls.append(['active']);mm.ret(0,int(c['active']))
    m.seams[0x4cede0]=active
    def target(mm):
        assert mm.ecx()==denizen;out=mm.arg();mm.w32(out,*c['target']);calls.append(['target']);mm.ret(4,out)
    m.seams[h.STUBS+0x200]=target
    def process(mm):
        assert mm.arg()==member;p=mm.ecx();w=mm.r32(p,16)
        assert w[0]==0xa3cc38 and w[13]==0
        calls.append(['process',defs.index(w[2]),w[3],w[6:11]])
        mm.ret(4,1)
    m.seams[0x4d9b60]=process
    m.call(0x522890,ecx=mind,args=(member,c['flag']))
    # Independent distance leaf, including half-unit boundary at every PC.
    close=m.call(0x529f10,args=(*c['member_position'],*c['target']))&255
    return dict(input=c,calls=calls,close=bool(close))
def main():
    original=h.P;rows=[]
    try:
        for label,image in [('original',original),('v14',h.V)]:
            h.P=image
            for precision in h.PRECISION:
                m=h.Machine(precision)
                for c in cases():rows.append(dict(image=label,precision=precision,**run(m,c)))
    finally:h.P=original
    half=len(rows)//2
    assert [{k:v for k,v in r.items() if k!='image'} for r in rows[:half]]==[{k:v for k,v in r.items() if k!='image'} for r in rows[half:]]
    doc=dict(scope=__doc__,pairs=half,executions=len(rows),rows=rows,
        original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
        producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        seams=[hex(a)for a in [0x57c680,0x5543f0,0x54ef40,0x4cede0,h.STUBS+0x200,0x4d9b60]],
        observed_pins=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
    path=HERE/'native-member-command.json';payload=(json.dumps(doc,indent=1)+'\n').encode()
    if '--check'in sys.argv:assert path.read_bytes()==payload
    else:assert not path.exists();path.write_bytes(payload)
    print(json.dumps(dict(result='PASS',pairs=half,executions=len(rows),pins=len(h.VISITED),sha256=hashlib.sha256(payload).hexdigest())))
if __name__=='__main__':main()
