"""4FBF40 direct placement tail with a declared578290 seam that mutates
the caller's XY and the separate facing stack copy. Native57D950 is seamed;
the real caller must forward new XY but its captured original facing.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h

def run(m,pc,i):
    mode=3 if i%2 else 0;slot=i%3
    input=dict(precision=pc,game_mode=mode,params_28=0,actor_xy=[0,0],radius=0,
        org_xy=[h.f2b((i-9)*.125),h.f2b((12-i)*.2)],
        offsets=[[h.f2b(j*.3),h.f2b(-j*.7)]for j in range(3)],
        facing=[h.f2b(.6),h.f2b(.8)],byte_50=[0,1,255][i%3],byte_51=[128,0,1][i%3],
        slot=slot,accepted_defs=[],recruit_flag_defs=[],candidates=[],
        structure_creation=False,placement_ok=i%4!=0,member_creation=i%5!=0,
        mutated_xy=[h.f2b(50+i*.5),h.f2b(-5-i*.25)])
    session=m.alloc(0x100);m.w32(0xb36550,session);m.w32(session+0xe4,mode)
    actor=m.alloc(0x100);org=m.alloc(0x100);offsets=m.alloc(24);params=m.alloc(0x54);world=m.alloc(0x40)
    m.w32(actor+0x7c,org);m.w32(actor+0x2c,*input['facing']);m.w32(org+0xbc,*input['org_xy']);m.w32(org+0x78,offsets)
    for j,offset in enumerate(input['offsets']):m.w32(offsets+j*8,*offset)
    m.w8(params+0x50,input['byte_50']);m.w8(params+0x51,input['byte_51'])
    calls=[]
    def placement(mm):
        assert mm.arg()==org+0x10
        point=mm.arg(2)
        calls.append(['slot_placement',mm.arg(1),mm.r32(point),mm.r32(point+4),mm.arg(3),mm.arg(4)])
        mm.w32(point,*input['mutated_xy'])
        mm.w32(mm.esp()+16,0x7fc12345,0xff800000)
        mm.ret(0,int(input['placement_ok']))
    m.seams[0x578290]=placement
    def create(mm):
        assert mm.ecx()==org
        calls.append(['create_member',*[mm.arg(j)for j in range(8)]])
        mm.ret(32,0x7000 if input['member_creation'] else 0)
    m.seams[0x57d950]=create
    result=m.call(0x4fbf40,ecx=world,args=(params,actor,slot))
    return dict(input=input,native=dict(result=result,calls=calls))

def main():
    original=h.P;rows=[]
    try:
        for label,image in [('original',original),('v14',h.V)]:
            h.P=image
            for pc in h.PRECISION:
                m=h.Machine(pc)
                for i in range(18):rows.append(dict(image=label,**run(m,pc,i)))
    finally:h.P=original
    half=len(rows)//2
    assert [{k:v for k,v in r.items()if k!='image'}for r in rows[:half]]==[{k:v for k,v in r.items()if k!='image'}for r in rows[half:]]
    doc=dict(scope=__doc__,pairs=half,executions=len(rows),rows=rows,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
        producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),observed_pins=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
    path=HERE/'native-member-slot-writeback.json';payload=(json.dumps(doc,indent=1)+'\n').encode()
    if '--check'in sys.argv:assert path.read_bytes()==payload
    else:
        assert not path.exists(),'use --check'
        path.write_bytes(payload)
    print(json.dumps(dict(pairs=half,executions=len(rows),observed_pins=len(h.VISITED),sha256=hashlib.sha256(payload).hexdigest(),result='PASS')))
if __name__=='__main__':main()
