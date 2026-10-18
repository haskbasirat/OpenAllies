"""Real524910 secondary-Updatable Update and529D00 base. Actual member-count
iterator57A730/4364C0, current-state getter529F70, home4D11A0, and arrival
destructor763B90 execute. State/owner virtuals, timed/Engage/Think services,
scheduling and allocation are explicit seams, with callback mutations.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
def cases():
    return [dict(index=i,state10=i%3!=0,owner=i%4!=0,owner239=[0,1,128,255][i%4],
        element=i%5!=0,attached=i%2==0,home=i%7==0,
        flags=[0,1,64,65,1024][i%5],time_flags=[0,1,64,65,1024][(i//5)%5],
        orphan_destroyed=i%3==0,members=[0,1,3][i%3],
        head_before=i%2==0,head_after=i%3==0,think_changes_actor=i%2==0,
        think_flags=[0,1,64,65][(i//3)%4],arrivals=i%4)for i in range(120)]
def run(m,c):
    m.next=h.HEAP+0x1000;calls=[]
    mind=m.alloc(0x300);actor=m.alloc(0x180);other=m.alloc(0x180);kingdom=m.alloc(0x260)
    primary=m.alloc(0x1b0);updatable=m.alloc(0x28);av=m.alloc(0x120)
    org=m.alloc(0x40);members=m.alloc(16);element=m.alloc(0x30)
    registry=m.alloc(0x60020);m.w32(0xb364c0,registry)
    m.u.mem_write(registry+6,(1).to_bytes(2,'little'));m.w32(registry+0x20008,other)
    stub=h.STUBS+0x200
    m.w32(mind,primary,actor,updatable,0xabc if c['head_before']else 0)
    m.w32(mind+0x28,0x10001 if c['home']else 0)
    m.w32(primary+0x1a8,stub+0x20);m.w32(updatable+4,stub+0x40)
    for obj in [actor,other]:
        m.w32(obj,av);m.w32(obj+0x70,mind)
        m.w32(obj+0x80,element if c['element']else 0);m.w32(obj+0x7c,org)
    m.w32(actor+8,c['flags']);m.w32(other+8,c['think_flags'])
    m.w32(element+0x10,org if c['attached']else 0)
    m.w32(av+0x34,stub);m.w32(av+0x108,stub+0x10);m.w8(kingdom+0x239,c['owner239'])
    m.w32(org+0x28,members,4)
    for i in range(c['members']):m.w32(members+i*4,other)
    states=[m.alloc(16),m.alloc(16)];sv=m.alloc(0x20);m.w32(sv+0xc,stub+0x30)
    for state in states:m.w32(state,sv)
    top=m.alloc(8);m.w32(top,states[0],0);m.w32(mind+0x10,top)
    arrival_nodes=[m.alloc(8)for _ in range(c['arrivals'])]
    for i,n in enumerate(arrival_nodes):m.w32(n,0x10010+i,arrival_nodes[i+1]if i+1<len(arrival_nodes)else 0)
    m.w32(mind+0x2bc,arrival_nodes[0]if arrival_nodes else 0,arrival_nodes[-1]if arrival_nodes else 0)
    def has(mm):assert mm.arg()==10;calls.append(['state10']);mm.ret(4,int(c['state10']))
    m.seams[stub]=has
    def owner(mm):calls.append(['owner108']);mm.ret(0,kingdom if c['owner']else 0)
    m.seams[stub+0x10]=owner
    def orphan(mm):
        assert mm.ecx()==mind;calls.append(['orphan'])
        if c['orphan_destroyed']:mm.w32(actor+8,mm.r32(actor+8)|1)
        mm.ret()
    m.seams[stub+0x20]=orphan
    def timed(mm):assert mm.ecx()==actor;calls.append(['timed']);mm.w32(actor+8,c['time_flags']);mm.ret()
    m.seams[0x4d37e0]=timed
    def engage(mm):assert mm.ecx()==mind+0x1cc;calls.append(['engage']);mm.w32(top,states[1]);mm.ret()
    m.seams[0x523ea0]=engage
    def think(mm):
        calls.append(['think',states.index(mm.ecx())]);mm.w32(mind+0xc,0xbeef if c['head_after']else 0)
        if c['think_changes_actor']:mm.w32(mind+4,other)
        else:mm.w32(actor+8,c['think_flags'])
        mm.ret()
    m.seams[stub+0x30]=think
    def schedule(mm):
        assert mm.ecx()==mind+8;calls.append(['schedule',mm.arg(),mm.arg(1)]);mm.ret(8)
    m.seams[stub+0x40]=schedule
    freed=[]
    def free(mm):freed.append(arrival_nodes.index(mm.arg()));mm.ret()
    m.seams[0x6f7430]=free
    m.call(0x524910,ecx=mind+8,args=(0x9abc,))
    return dict(input=c,calls=calls,flags=m.r32(actor+8),
        actor_changed=m.r32(mind+4)==other,freed=freed,
        arrival_head_null=m.r32(mind+0x2bc)==0,arrival_tail_null=m.r32(mind+0x2c0)==0)
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
    assert [{k:v for k,v in r.items()if k!='image'}for r in rows[:half]]==[{k:v for k,v in r.items()if k!='image'}for r in rows[half:]]
    doc=dict(scope=__doc__,pairs=half,executions=len(rows),rows=rows,
        original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
        producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        seams=[hex(a)for a in [0x4d37e0,0x523ea0,0x6f7430]],
        observed_pins=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
    p=HERE/'native-regiment-update.json';payload=(json.dumps(doc,indent=1)+'\n').encode()
    if '--check'in sys.argv:assert p.read_bytes()==payload
    else:assert not p.exists();p.write_bytes(payload)
    print(json.dumps(dict(result='PASS',pairs=half,executions=len(rows),observed_pins=len(h.VISITED),sha256=hashlib.sha256(payload).hexdigest())))
if __name__=='__main__':main()

