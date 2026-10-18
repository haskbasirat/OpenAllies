"""Execute original/v14 Kingdom4E1430/4E1E40, with explicit HQ leaf seams.

HQ573E50 and574BF0 are observed callbacks, NOT claimed as implementations.
Includes duplicate HQs, ID-generation failure, changing vector pointers/counts,
changing actor definition pointers and low-byte callback return semantics.
"""
from pathlib import Path
import sys,json,random,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'phase8-ai'))
from native_machine import Machine,original,modified,header,instructions

VISITED={}
def run(case,image,precision):
    m=Machine(precision)
    if image=='v14':
        for _,rva,raw,off in modified.sections:
            mapped=max(0,min(raw,((original.image_size+0xfff)&~0xfff)-rva))
            if mapped:m.u.mem_write(modified.base+rva,modified.b[off:off+mapped])
    owner=m.alloc(0x400);actor=m.alloc(0x200);org=m.alloc(0x100)
    m.w(actor+0x7c,org)
    defs=[m.alloc(0x600) for _ in range(3)]
    defid={p:i for i,p in enumerate(defs)}
    m.w(actor+4,defs[case['definition']])
    hqs={i:m.alloc(0x200) for i in range(1,5)}
    hqid={p:i for i,p in hqs.items()}
    comps={i:m.alloc(0x100) for i in hqs}
    compid={p:i for i,p in comps.items()}
    for i in hqs:m.w(hqs[i]+0xd8,comps[i])
    m.w(owner+0x2dc,m.array([hqs[i] for i in case['hqs']]))
    m.w(owner+0x2e0,len(case['hqs']))
    # Generation checked401060 is inlined in4E1E40.
    table=m.alloc(0x60010);m.w(0xb364c0,table)
    token=0x90002;m.w(org+0xd0,token)
    m.u.mem_write(table+4+2*2,(9 if case['valid_reference'] else 8).to_bytes(2,'little'))
    m.w(table+0x20004+2*4,hqs[case['reference']])
    events=[];calls=[0]
    def after_callback():
        calls[0]+=1
        if calls[0]==1:
            if case['replace'] is not None:
                m.w(owner+0x2dc,m.array([hqs[i] for i in case['replace']]))
                # Captured count must remain unchanged after this store.
                m.w(owner+0x2e0,0)
            m.w(actor+4,defs[case['next_definition']])
    def can(machine):
        hq=compid[m.ecx()];d=defid[m.arg(1)]
        events.append(['can',hq,d]);v=case['returns'][hq-1]
        after_callback();m.ret(v,pop=4)
    def accepts(machine):
        hq=compid[m.ecx()];d=defid[m.arg(1)]
        assert m.arg(2)==0
        events.append(['accept',hq,d]);v=case['returns'][hq-1]
        after_callback();m.ret(v,pop=8)
    m.seam(0x573e50,can);m.seam(0x574bf0,accepts)
    if case['kind']=='can':
        m.observe(0x4e1450,lambda _:events.append(['hq',hqid[m.r(m.r(owner+0x2dc)+4*calls[0])]]))
        value=bool(m.call(0x4e1430,ecx=owner,args=[defs[case['definition']]])&255)
    else:
        m.observe(0x4e1e63,lambda _:events.append(['hq',hqid[m.r(m.r(owner+0x2dc)+4*calls[0])]]))
        m.observe(0x4e1e6c,lambda _:events.append(['definition',defid[m.r(actor+4)]]))
        m.observe(0x4e1e81,lambda _:events.append(['reference',case['reference'] if case['valid_reference'] else None]))
        value=m.call(0x4e1e40,ecx=owner,args=[actor])
        if value>=0x80000000:value-=0x100000000
    m.verify_v14();VISITED.update(m.visited)
    return {'value':value,'events':events}

def main():
    rng=random.Random(0x4e1430)
    cases=[]
    for kind in ['can','count']:
        for i in range(100):
            n=i%7
            hqs=[rng.randint(1,4) for _ in range(n)]
            cases.append({'kind':kind,'hqs':hqs,'returns':[rng.choice([0,1,2,255,256]) for _ in range(4)],
                'reference':rng.randint(1,4),'valid_reference':bool(i%3),
                'definition':rng.randrange(3),'next_definition':rng.randrange(3),
                'replace':([rng.randint(1,4) for _ in range(n)] if i%2 else None)})
    rows=[]
    for pc in ['Single24','Double53','Extended64']:
        for case in cases:
            result=run(case,'original',pc)
            assert result==run(case,'v14',pc)
            rows.append({'precision':pc,'input':case,'expected':result})
    report=header(__file__,__doc__)
    report.update({'rows':rows,'executions':len(rows)*2,'instructions':instructions(VISITED),
        'seams':{'573E50':'HQ can recruit(definition)','574BF0':'HQ accepts(definition,false)'},
        'limits':['HQ leaf implementations are not executed or bound by this proof.',
            'Valid owned HQ vectors only; malformed/null HQ faults not claimed.']})
    out=HERE/'native-owner.json';out.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'sha256':hashlib.sha256(out.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
