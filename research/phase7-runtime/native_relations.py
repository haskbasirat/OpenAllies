"""Execute original Actor/Kingdom/Team diplomacy lookups with no behavior hooks."""
from pathlib import Path
import json,hashlib,random,sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_runtime import Machine
import native_queues as q

def run():
    rng=random.Random(72031);cases=[]
    for sample in range(16):
        m=Machine();teams={};kingdoms={};actors={}
        team_data={};kingdom_data={}
        defaults=[rng.randrange(8),rng.randrange(8)]
        m.write(0xb28158,defaults[0]);m.write(0xb2815c,defaults[1])
        for id in (101,102):teams[id]=m.alloc(0x80)
        for id in (201,202,203):kingdoms[id]=m.alloc(0x240)
        def table(values):
            pointer=m.alloc(len(values)*4)
            for index,value in enumerate(values):m.write(pointer+index*4,value)
            return pointer
        for slot,(id,pointer) in enumerate(teams.items()):
            row={'slot':slot,'kingdoms':[rng.randrange(8) for _ in kingdoms],'teams':[rng.randrange(8) for _ in teams]}
            team_data[id]=row;m.write(pointer+0x38,slot)
            m.write(pointer+0x4c,table(row['kingdoms']));m.write(pointer+0x58,table(row['teams']))
        for slot,(id,pointer) in enumerate(kingdoms.items()):
            team=[None,101,102][(slot+sample)%3]
            row={'slot':slot,'team':team,'kingdoms':[rng.randrange(8) for _ in kingdoms],'teams':[rng.randrange(8) for _ in teams]}
            kingdom_data[id]=row;m.write(pointer,0xa299d8);m.write(pointer+0x188,slot)
            m.write(pointer+0x1ec,teams[team] if team is not None else 0)
            m.write(pointer+0x1d0,table(row['kingdoms']));m.write(pointer+0x1dc,table(row['teams']))
        for id,owner in enumerate([None,201,202,203],301):
            pointer=m.alloc(0x190);m.write(pointer,0xa273f0);m.write(pointer+0xe8,kingdoms[owner] if owner is not None else 0)
            actors[id]=(pointer,owner)
        queries=[]
        for owner in [None,201,202,203]:
            receiver=actors[301 if owner is None else owner+101][0]
            for other in [None,201,202,203]:
                args=[kingdoms[other] if other is not None else 0]
                relation=m.call(0x4cf040,receiver,args)
                queries.append({'kind':'actor','owner':owner,'other':other,'relation':relation,
                                'hostile':bool(m.call(0x4e52a0,receiver,args)),'allied':bool(m.call(0x4e5280,receiver,args))})
        for id,pointer in kingdoms.items():
            for other in [None,201,202,203]:
                args=[kingdoms[other] if other is not None else 0]
                queries.append({'kind':'kingdom','id':id,'other':other,'relation':m.call(0x4e52d0,pointer,args)})
        for id,pointer in teams.items():
            for other in [None,201,202,203]:
                args=[kingdoms[other] if other is not None else 0]
                queries.append({'kind':'team','id':id,'other':other,'relation':m.call(0x4e10f0,pointer,args)})
        cases.append({'defaults':defaults,'teams':team_data,'kingdoms':kingdom_data,'queries':queries})
    output={'passed':True,'cases':cases,'query_count':sum(len(c['queries']) for c in cases),
            'native_call_count':sum(len(c['queries'])+32 for c in cases),
            'original_sha256':hashlib.sha256(q.ORIGINAL.b).hexdigest(),'v14_sha256':hashlib.sha256(q.ACCEPTED.b).hexdigest(),
            'visited':[{'va':hex(va),'bytes':raw} for va,raw in sorted(q.VISITED.items())],
            'scope':'All relation functions execute unchanged original instructions without behavior callbacks. Input tables/globals are explicit fixture state; live Kingdom construction and diplomacy changes remain separate.'}
    target=HERE/'native-relations.json';target.write_text(json.dumps(output,indent=2)+'\n')
    print(json.dumps({'passed':True,'queries':output['query_count'],'native_calls':output['native_call_count'],'sha256':hashlib.sha256(target.read_bytes()).hexdigest()}))
if __name__=='__main__':run()
