"""58F530/58F420 inputs with +218 backed by feasible owned recruit vectors.
The primitive oracle separately covers signed/raw integer boundaries. Here
all counts are actual vector lengths:0,1,2,5,17 (not shadow scalar storage).
"""
from pathlib import Path
import json
from native_statistics_live import nm, Machine, ORIGINAL, V14, run_snapshot, snapshots, sha, instructions
HERE=Path(__file__).resolve().parent
def main():
    out={'schema':1,'original_sha256':sha(ORIGINAL.b),'v14_sha256':sha(V14.b),'producer_sha256':sha(Path(__file__).read_bytes()),'fixtures':[]};visited={}
    for i,case in enumerate(snapshots()):
        case['count']=[0,1,2,5,17][i%5];case['name']='owned-'+case['name']
        for pc in nm.PRECISIONS:
            expected=None
            for image in [ORIGINAL,V14]:
                nm.original=image;m=Machine(pc);actual=run_snapshot(m,case)
                for a,b in m.visited.items():assert b==ORIGINAL.range(a,len(b))==V14.range(a,len(b))
                visited.update(m.visited)
                if expected is None:expected=actual
                else:assert expected==actual
            out['fixtures'].append({'precision':pc,'case':case,'expected':expected})
    nm.original=ORIGINAL;out['executions']=len(out['fixtures'])*2;out['instructions']=instructions(visited)
    (HERE/'native-runtime-snapshots.json').write_text(json.dumps(out,separators=(',',':'))+'\n')
    print(len(out['fixtures']),'rows;',out['executions'],'original/v14 executions')
if __name__=='__main__':main()
