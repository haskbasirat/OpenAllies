"""Native tracking cases exactly representable by current runtime owners.
This complements broad primitive tests with Organization/City combinations,
started guard and all applicable old categories, without fake component payloads.
"""
from pathlib import Path
import json
from native_statistics_live import nm, Machine, ORIGINAL, V14, run_track, f32, sha, instructions
HERE=Path(__file__).resolve().parent

def main():
    out={'schema':1,'original_sha256':sha(ORIGINAL.b),'v14_sha256':sha(V14.b),'producer_sha256':sha(Path(__file__).read_bytes()),'fixtures':[]}
    visited={}
    for org in [False,True]:
        for city in [False,True]:
            for started in [False,True]:
                for old in [0,47,48,49,52,53,54,55,56,777]:
                    facts={k:False for k in ['component_7c','component_d8','structure_94','construction_9c','component_88','component_d4','definition_4d0','definition_43c_nonempty']}
                    facts.update(component_7c=org,component_d4=city,component_98_level=None,definition_4e0=0)
                    case={'name':f'org{org}-city{city}-started{started}-old{old}','op':'track','started':started,'field':old,'scores':[f32((i%5)+0.5) for i in range(56)],'facts':facts,'virtual_100':[False]}
                    for pc in nm.PRECISIONS:
                        expected=None
                        for image in [ORIGINAL,V14]:
                            nm.original=image;m=Machine(pc);actual=run_track(m,case)
                            for a,b in m.visited.items(): assert b==ORIGINAL.range(a,len(b))==V14.range(a,len(b))
                            visited.update(m.visited)
                            if expected is None:expected=actual
                            else:assert expected==actual
                        out['fixtures'].append({'precision':pc,'case':case,'expected':expected})
    nm.original=ORIGINAL
    out['executions']=2*len(out['fixtures']);out['instructions']=instructions(visited)
    (HERE/'native-runtime-tracking.json').write_text(json.dumps(out,separators=(',',':'))+'\n')
    print(len(out['fixtures']),'rows;',out['executions'],'original/v14 executions')
if __name__=='__main__':main()
