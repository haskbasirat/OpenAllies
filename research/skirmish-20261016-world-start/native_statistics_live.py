"""Execute unchanged original AND v14 live statistics + complete snapshots.
Only seam is Actor virtual+100. All bookkeeping, class selection, resource
snapshot arithmetic and weighted formulas execute their original instructions.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine, f32, instructions, sha
ORIGINAL, V14 = nm.original, nm.modified
R = random.Random(0x58F010)

def setup_track(m, case):
    stats, actor, data, world = m.alloc(0x108), m.alloc(0x200), m.alloc(0x500), m.alloc(0x300)
    m.w(0xB36534, world); m.wb(world+0x274, int(case['started']))
    m.w(actor+4, data); m.w(actor+0x13C,case['field'])
    for i,x in enumerate(case['scores']): m.w(stats+0x28+4*i,x)
    f=case['facts']
    for name,off in [('component_7c',0x7c),('component_d8',0xd8),('structure_94',0x94),('construction_9c',0x9c),('component_88',0x88),('component_d4',0xd4)]:
        m.w(actor+off,m.alloc(4) if f[name] else 0)
    if f['component_98_level'] is not None:
        org=m.alloc(0x20);m.w(org+0x10,f['component_98_level']);m.w(actor+0x98,org)
    m.w(data+0x4d0,int(f['definition_4d0']));m.w(data+0x43c,m.alloc(8) if f['definition_43c_nonempty'] else 0);m.w(data+0x4e0,f['definition_4e0'])
    vt=m.alloc(0x110);m.w(actor,vt)
    calls=[]
    def virtual(mm):
        value=case['virtual_100'][min(len(calls),len(case['virtual_100'])-1)]
        calls.append(value);mm.ret(int(value))
    m.w(vt+0x100,m.stub(virtual))
    return stats,actor,calls

def run_track(m, case):
    stats,actor,calls=setup_track(m,case)
    m.call(0x58F010 if case['op']=='track' else 0x58EFE0,stats,[actor])
    return {'field':m.r(actor+0x13c),'scores':[m.r(stats+0x28+4*i) for i in range(56)],'virtual_calls':calls}

def run_snapshot(m,case):
    stats,kingdom,dest,data,registry=m.alloc(0x108),m.alloc(0x400),m.alloc(0x160),m.alloc(0x600),m.alloc(0x30)
    n=len(case['resources']);start=62+2*n
    m.w(0xB36530,data);m.w(data+0x574,registry)
    m.w(0xB3B428,62);m.w(0xB3B424,62+n);m.w(0xB3B420,start)
    for i,x in enumerate(case['production']+[0]*(10-n)+case['scores']):m.w(stats+4*i,x)
    m.w(kingdom+0x1a0,m.array(case['provided']));m.w(kingdom+0x1b8,m.array(case['consumed']));m.w(kingdom+0x218,case['count']);m.w(kingdom+0x340,case['relation_identity'] or 0)
    resources=[]
    for r in case['resources']:
        p=m.alloc(0x30);m.w(p+0x18,r['kind']);m.wb(p+0x2c,int(r['limited_provider']));resources.append(p)
    m.w(data+0x34,m.array(resources));m.w(data+0x38,n)
    defs=[0]*start
    for row in case['rows']:
        d=m.alloc(0x20);head=0
        for term in reversed(row):
            pair=m.array([term['index'],term['weight']]);node=m.array([pair,head]);head=node
        m.w(d+0x1c,head);defs.append(d)
    m.w(registry+0x24,m.array(defs));m.w(registry+0x28,len(defs))
    m.call(0x58F530,dest,[stats,kingdom])
    return {'production':[m.r(dest+4+4*i) for i in range(n)],'scores':[m.r(dest+0x2c+4*i) for i in range(56)],
      'resource_levels':[m.r(dest+0x10c+4*i) for i in range(n)],'kingdom_count':m.r(dest+0x134),
      'derived':[m.r(dest+0x138+4*i) for i in range(max(5,len(case['rows'])))], 'relation_identity':m.r(dest) or None}

def tracks():
    cases=[]
    for i in range(110):
        facts={k:bool(R.getrandbits(1)) for k in ['component_7c','component_d8','structure_94','construction_9c','component_88','component_d4','definition_4d0','definition_43c_nonempty']}
        facts['component_98_level']=R.choice([None,0,1,3,16777216,2147483647,-2147483648])
        facts['definition_4e0']=R.choice([0,0,1,5])
        field=R.choice(list(range(47,57))+[0,999])
        if field in [50,51] and facts['component_98_level'] is None: facts['component_98_level']=1
        cases.append({'name':f'track-{i}','op':'remove' if i%5==0 else 'track','started':i%7!=0,'field':field,
          'scores':[f32(R.choice([0,1,3,16777216,0.1,-1])) for _ in range(56)], 'facts':facts,
          'virtual_100':[bool(R.getrandbits(1)),bool(R.getrandbits(1))]})
    return cases

def snapshots():
    cases=[]
    for i in range(44):
        n=i%11;start=62+2*n
        resources=[{'kind':R.randrange(3),'limited_provider':bool(R.getrandbits(1))} for _ in range(n)]
        rows=[]
        for j in range(6):
            choices=list(range(62))+list(range(62,62+2*n))+list(range(start,start+6))
            rows.append([{'index':R.choice(choices),'weight':f32(R.choice([0,-1,0.1,1,1.5,3,0.0001]))} for _ in range(R.randrange(7))])
        vals=lambda count:[f32(R.choice([0,-0.0,0.1,1,3,16777216,1e-30])) for _ in range(count)]
        cases.append({'name':f'snapshot-{i}','resources':resources,'production':vals(n),'scores':vals(56),
          'provided':vals(n),'consumed':vals(n),'count':R.choice([0,1,-1,16777217,2147483647,-2147483648]),
          'relation_identity':R.choice([None,12345]),'rows':rows})
    return cases

def main():
    output={'schema':1,'original_sha256':sha(ORIGINAL.b),'v14_sha256':sha(V14.b),'producer_sha256':sha(Path(__file__).read_bytes()),'track':[],'snapshot':[]}
    visited={};executions=0
    for kind,cases,run in [('track',tracks(),run_track),('snapshot',snapshots(),run_snapshot)]:
        for case in cases:
            for pc in nm.PRECISIONS:
                expected=None
                for exe in [ORIGINAL,V14]:
                    nm.original=exe;m=Machine(pc);actual=run(m,case);executions+=1
                    for a,b in m.visited.items():assert ORIGINAL.range(a,len(b))==b==V14.range(a,len(b)),hex(a)
                    visited.update(m.visited)
                    if expected is None:expected=actual
                    else:assert expected==actual,(case['name'],pc)
                output[kind].append({'name':case['name'],'precision':pc,'case':case,'expected':expected})
    nm.original=ORIGINAL
    output['executions']=executions;output['instructions']=instructions(visited)
    (HERE/'native-statistics-live.json').write_text(json.dumps(output,separators=(',',':'))+'\n',encoding='utf-8')
    print(f"{len(output['track'])} tracking + {len(output['snapshot'])} snapshot fixtures; {executions} original/v14 executions; {len(visited)} pinned instructions")

if __name__=='__main__':main()
