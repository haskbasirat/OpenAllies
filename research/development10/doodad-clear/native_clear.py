"""4F5560 clear coordinator and REAL54C200/540C40/543A90 trigger membership.
Overlap descriptor/query, diagnostic transport, Thing-name ownership, and
destruction/allocation are declared seams; wcscmp and all membership execute.
"""
from pathlib import Path
import hashlib,json,random,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h

def cases():
    rng=random.Random(0x4f5560)
    out=[]
    for i in range(48):
        actors=[dict(gtid=(1<<16)|(j+1),category=[2,9,3,8][(i+j)%4],
                     flags=(1<<10 if (i+j)%5==0 else 0),
                     name=([65,66,0,90] if (i+j)%3==0 else [65,98]),ids=j+100)
                for j in range(4)]
        triggers=[]
        for t in range(i%4):
            lists=[]
            for l in range(3):
                elements=[]
                for e in range((i+t+l)%3):
                    parameters=[]
                    for p in range((i+t+l+e)%4):
                        a=rng.randrange(4)
                        parameters.append(dict(argument=[5,11,10,6,4][rng.randrange(5)],
                            item=[12,31,30,15,11,45][rng.randrange(6)],
                            word=actors[a]['gtid'] if i%3 else 0xf00d,
                            text=actors[a]['name'] if i%2 else [97,66]))
                    elements.append(parameters)
                lists.append(elements)
            triggers.append(lists)
        out.append(dict(index=i,actors=actors,triggers=triggers,
                        order=([3,1,0,2] if i%2 else [0,2,1,3]),
                        mutate_next=i%3==0))
    # Positive coverage at each short circuit and named case-sensitive/NUL path.
    for list_index in range(3):
        c=json.loads(json.dumps(out[list_index+1]));c['index']=48+list_index
        c['actors'][0]['category']=2;c['actors'][0]['flags']=0
        c['triggers']=[[[ ],[ ],[ ]]]
        c['triggers'][0][list_index]=[[dict(argument=5,item=15,word=0,text=[65,66,0,88])]]
        c['actors'][0]['name']=[65,66,0,90]
        c['mutate_next']=False
        out.append(c)
    return out

def run(m,c):
    m.next=h.HEAP+0x1000
    calls=[];membership=[]
    world=m.alloc(0x100); manager=m.alloc(0x80);m.w32(0xb36520,manager)
    m.w32(0xb36534,world);m.w32(world+0x30,m.alloc(0x10))
    registry=m.alloc(0x60020);m.w32(0xb364c0,registry)
    actors=[]; ptr_index={};names=[]
    def text(units):
        p=m.alloc(2*(len(units)+1));m.u.mem_write(p,b''.join(int(x).to_bytes(2,'little') for x in units)+b'\0\0');return p
    for j,a in enumerate(c['actors']):
        thing=m.alloc(0x100);data=m.alloc(0x40);cls=m.alloc(0x30)
        m.w32(thing+4,data,a['flags']);m.w32(thing+0x14,a['gtid'])
        m.w32(data+8,a['ids']);m.w32(data+0x18,cls);m.w32(cls+0x1c,a['category'])
        m.u.mem_write(registry+4+(j+1)*2,(1).to_bytes(2,'little'))
        m.w32(registry+0x20004+(j+1)*4,thing)
        actors.append(thing);ptr_index[thing]=j
        # CString+0C=2 is decremented back to1 by each actual543A90 tail.
        s=m.alloc(0x10+2*(len(a['name'])+1));m.w32(s+0xc,1)
        m.u.mem_write(s+0x10,b''.join(x.to_bytes(2,'little') for x in a['name'])+b'\0\0')
        names.append(s)
    trigger_ptrs=[]
    def linked(values):
        head=0
        for value in reversed(values):
            n=m.alloc(8);m.w32(n,value,head);head=n
        return head
    for trigger in c['triggers']:
        tr=m.alloc(0x38);trigger_ptrs.append(tr)
        for l,elements in enumerate(trigger):
            ep=[]
            for parameters in elements:
                el=m.alloc(0x14);pa=m.alloc(0x34*len(parameters));m.w32(el+4,len(parameters),pa)
                for p,v in enumerate(parameters):
                    entry=pa+p*0x34;m.w32(entry+4,v['word']);m.w32(entry+0x1c,text(v['text']))
                    m.w32(entry+0x28,v['argument'],v['item'])
                ep.append(el)
            m.w32(tr+0x14+l*8,linked(ep))
    m.w32(manager+4,linked(trigger_ptrs))
    def name(mm):
        j=ptr_index[mm.ecx()];calls.append(['name',j]);membership.append(['name',j])
        mm.w32(names[j]+0xc,mm.r32(names[j]+0xc)+1)
        mm.w32(mm.arg(),names[j]+0x10);mm.ret(4,mm.arg())
    m.seams[0x4bedf0]=name
    # Execute the actual membership chain independently for EVERY Thing first.
    refs=[]
    for j,p in enumerate(actors):
        calls.clear();membership.clear()
        r=m.call(0x54c200,ecx=manager,args=(p,))
        refs.append(dict(ordinal=None if not r else trigger_ptrs.index(r),calls=list(membership)))
    calls.clear();membership.clear()
    # Trigger name getters can only read/refcount; all loaded arrays stay put.
    nodes=[]
    for j in c['order']:
        n=m.alloc(8);m.w32(n,c['actors'][j]['gtid'],0);nodes.append(n)
    for k,n in enumerate(nodes[:-1]):m.w32(n+4,nodes[k+1])
    def descriptor(mm):mm.ret(4,mm.ecx())
    m.seams[0x5530f0]=descriptor
    def collect(mm):
        calls.append(['collect']);out=mm.arg(1);mm.w32(out,nodes[0],nodes[-1]);mm.ret(12)
    m.seams[0x555620]=collect
    def log(mm):
        calls.append(['purge',mm.arg(5),mm.arg(6)]);mm.ret()
    m.seams[0x404f90]=log
    def destroy(mm):
        j=ptr_index[mm.arg()];assert mm.ecx()==world;calls.append(['destroy',j])
        if c['mutate_next']:
            # Current fields of later Things are reread, not preclassified.
            k=(j+1)%4;m.w32(actors[k]+8,1<<10)
        mm.ret(4)
    m.seams[0x4f5490]=destroy
    freed=[]
    def free(mm):freed.append(nodes.index(mm.arg()));mm.ret()
    m.seams[0x6f7430]=free
    placement=m.alloc(24)
    m.call(0x4f5560,ecx=world,args=(placement,))
    assert freed==list(range(len(nodes)))
    assert all(m.r32(s+0xc)==1 for s in names)
    return dict(input=c,references=refs,calls=calls,
                final_flags=[m.r32(p+8) for p in actors],freed=freed)

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
        producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
        declared_seams=[hex(a) for a in [0x5530f0,0x555620,0x4bedf0,0x404f90,0x4f5490,0x6f7430]],
        observed_pins=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items())])
    path=HERE/'native-clear.json';payload=(json.dumps(doc,indent=1)+'\n').encode()
    if '--check' in sys.argv:assert path.read_bytes()==payload
    else:
        assert not path.exists();path.write_bytes(payload)
    print(json.dumps(dict(result='PASS',pairs=half,executions=len(rows),observed_pins=len(h.VISITED),sha256=hashlib.sha256(payload).hexdigest())))
if __name__=='__main__':main()
