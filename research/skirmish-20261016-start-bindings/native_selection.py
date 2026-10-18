"""Original/v14 4B6BA0 +4B6FF0 with real85D840, and473370.
Seams: allocation; missing-substitution diagnostic only. No RNG/arithmetic seam.
"""
from pathlib import Path
import json,random,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine,f32,sha,instructions
ORIGINAL,V14=nm.original,nm.modified
R=random.Random(0x4B6FF0)

def weighted(m,c):
    table=m.alloc(8);rng=m.alloc(4);m.w(rng,c['seed'])
    m.seam(0x6f7810,lambda x:x.ret(x.alloc(x.arg(1))))
    for e in c['input']:m.call(0x4b6ba0,table,[e['definition']or 0,e['weight_bits']])
    entries=[];node=m.r(table)
    while node:entries.append({'definition':m.r(node+4)or None,'weight_bits':m.r(node)});node=m.r(node+8)
    draws=[]
    for _ in range(6):draws.append({'selected':m.call(0x4b6ff0,table,[rng])or None,'state':m.r(rng)})
    return {'entries':entries,'total_bits':m.r(table+4),'draws':draws}

def substitution(m,c):
    nation=m.alloc(0x30);m.w(nation+8,m.alloc(4));ids={}
    for key in set([c['key']]+[r['placeholder'] for r in c['entries']]):ids[key]=m.alloc(0x10)
    head=0
    for row in reversed(c['entries']):head=m.array([ids[row['placeholder']],row['replacement']or 0,head])
    m.w(nation+0x1c,head);logs=[]
    def log(x):logs.append(True);x.ret()
    m.seam(0x404ed0,log)
    result=m.call(0x473370,nation,[ids[c['key']]])
    return {'replacement':result or None,'missing_logged':bool(logs)}

def main():
    out={'schema':1,'original_sha256':sha(ORIGINAL.b),'v14_sha256':sha(V14.b),'weighted':[],'substitution':[]};visited={};execs=0
    cases=[{'seed':s,'input':[{'definition':i+1 if i%4 else None,'weight_bits':f32(w)} for i,w in enumerate(weights)]} for s in [0,1,0xffffffff,0x8e4b12a5,3627657861] for weights in [[],[0,-1],[1],[1,2,3],[0,1,-0.,2], [0.01,0.02,0.03],[1e-20,1e-10,1],[16777216,1,1,8]]]
    for i in range(25):cases.append({'seed':R.getrandbits(32),'input':[{'definition':j+1,'weight_bits':f32(R.uniform(.01,5))}for j in range(R.randrange(1,14))]})
    for c in cases:
        for pc in nm.PRECISIONS:
            expected=None
            for image in [ORIGINAL,V14]:
                nm.original=image;m=Machine(pc);actual=weighted(m,c);execs+=len(c['input'])+6
                for a,b in m.visited.items():assert b==ORIGINAL.range(a,len(b))==V14.range(a,len(b))
                visited.update(m.visited)
                if expected is None:expected=actual
                else:assert expected==actual
            out['weighted'].append({'precision':pc,'case':c,'expected':expected})
    for rows,key in [([],1),([{'placeholder':1,'replacement':None}],1),([{'placeholder':1,'replacement':7}],2),([{'placeholder':1,'replacement':7},{'placeholder':1,'replacement':8}],1),([{'placeholder':1,'replacement':None},{'placeholder':2,'replacement':5}],2)]:
        c={'entries':rows,'key':key};expected=None
        for image in[ORIGINAL,V14]:
            nm.original=image;m=Machine('Double53');actual=substitution(m,c);execs+=1
            for a,b in m.visited.items():assert b==ORIGINAL.range(a,len(b))==V14.range(a,len(b))
            visited.update(m.visited)
            if expected is None:expected=actual
            else:assert expected==actual
        out['substitution'].append({'case':c,'expected':expected})
    nm.original=ORIGINAL;out['function_executions']=execs;out['instructions']=instructions(visited);out['producer_sha256']=sha(Path(__file__).read_bytes())
    (HERE/'native-selection.json').write_text(json.dumps(out,separators=(',',':'))+'\n');print(len(out['weighted']),'weighted rows;',execs,'native calls')
if __name__=='__main__':main()
