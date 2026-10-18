"""57D950 descriptor and ordered child calls. CString manager/destruction,
owner virtual108, fresh4F9C10, recruit pairing4CEA60, add57CBA0 and notify578040
are explicit seams. Both immutable images/allPC. No whole-member claim.
"""
from pathlib import Path
import hashlib,json,random,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import UC_X86_REG_EIP

def cases():
    rng=random.Random(0x57d950)
    return [dict(index=i,slot=i%3,occupied=(i%7==0),result=(i%4!=0),
                 property=(0 if i%3==0 else 0x55aa+i),
                 allegiance=(0 if i%4==0 else 0x6000+i),
                 owner_null=(i%5==0),owner_gtid=0x10000+i,
                 # Callback mutates the same owner's GTID before its read.
                 updated_gtid=0x20000+i,
                 position=[rng.getrandbits(32) for _ in range(3)],
                 direction=[rng.getrandbits(32) for _ in range(2)],
                 byte50=[0,1,2,127,128,255][i%6],
                 byte51=[255,128,127,2,1,0][i%6]) for i in range(84)]

def run(m,c):
    calls=[]
    org=m.alloc(0x100); owner=m.alloc(0x200); members=m.alloc(12)
    groups=m.alloc(12); definitions=m.alloc(12); layout=m.alloc(0x50)
    props=m.alloc(12); manager=m.alloc(0x20); nil=m.alloc(0x30)
    vt=m.alloc(0x120); m.w32(owner,vt); m.w32(owner+0x14,c['owner_gtid'])
    stub=h.STUBS+0x200
    m.w32(vt+0x108,stub)
    m.w32(org+4,owner); m.w32(org+0x18,layout); m.w32(org+0x1c,definitions)
    m.w32(org+0x28,members); m.w32(org+0x34,props); m.w32(layout+0x28,groups)
    for i in range(3):
        m.w32(groups+4*i,2-i); m.w32(definitions+4*(2-i),0x7000+i)
    m.w32(members+4*c['slot'],0xdead if c['occupied'] else 0)
    m.w32(props+4*c['slot'],c['property'])
    m.w32(nil+0xc,1000); nilvt=m.alloc(0x10); m.w32(manager,nilvt)
    m.w32(nilvt+0xc,stub+0x10)
    m.seams[0x856370]=lambda mm:mm.ret(0,manager)
    m.seams[stub+0x10]=lambda mm:mm.ret(0,nil)
    def allegiance(mm):
        assert mm.ecx()==owner
        calls.append(['allegiance'])
        mm.w32(owner+0x14,c['updated_gtid'])
        if c['owner_null']: mm.w32(org+4,0)
        mm.ret(0,c['allegiance'])
    m.seams[stub]=allegiance
    def create(mm):
        p=mm.arg(); words=mm.r32(p,21)
        assert words[7]==nil+0x10
        words[7]=0
        calls.append(['create',words])
        mm.ret(4,0x1234 if c['result'] else 0)
    m.seams[0x4f9c10]=create
    def remove(mm):
        calls.append(['pair_recruit_item',mm.ecx(),mm.arg()]);mm.ret(4)
    m.seams[0x4cea60]=remove
    def add(mm):
        assert mm.ecx()==org
        calls.append(['add_member',mm.arg(),mm.arg(1)]);mm.ret(8)
    m.seams[0x57cba0]=add
    def notify(mm):
        assert mm.ecx()==org
        calls.append(['notify_member',mm.arg()]);mm.ret(4)
    m.seams[0x578040]=notify
    try:
        result=m.call(0x57d950,ecx=org,args=(c['slot'],*c['position'],*c['direction'],c['byte50'],c['byte51']))
    except Exception:
        print(json.dumps(dict(failed_case=c['index'],eip=hex(m.u.reg_read(UC_X86_REG_EIP)),calls=calls)))
        raise
    return dict(input=c,returned=result,calls=calls,name_refcount=m.r32(nil+0xc))

def main():
    original=h.P; rows=[]
    try:
        for label,image in [('original',original),('v14',h.V)]:
            h.P=image
            for precision in h.PRECISION:
                m=h.Machine(precision)
                for c in cases():rows.append(dict(image=label,precision=precision,**run(m,c)))
    finally:h.P=original
    half=len(rows)//2
    assert [{k:v for k,v in r.items() if k!='image'} for r in rows[:half]]==[{k:v for k,v in r.items() if k!='image'} for r in rows[half:]]
    doc=dict(scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
             producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             pairs=half,executions=len(rows),rows=rows,
             executed_pins=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items())])
    path=HERE/'native-create-member.json';payload=(json.dumps(doc,indent=1)+'\n').encode()
    if '--check' in sys.argv:assert path.read_bytes()==payload,'immutable fixture differs'
    else:
        assert not path.exists(),'use --check for replay'
        path.write_bytes(payload)
    print(json.dumps(dict(pairs=half,executions=len(rows),pins=len(h.VISITED),sha256=hashlib.sha256(payload).hexdigest(),result='PASS')))
if __name__=='__main__':main()
