"""Read-only independent replay of the frozen saved-refresh integration fixtures.

Does not regenerate any owner fixture or review the borrowed-flow primitive.
Runs composition cases in reverse order, verifies both executable identities,
and independently re-executes source getters rather than trusting Rust output.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import native_saved_refresh as native

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    initialized=json.loads((HERE/'native-initialized-refresh.json').read_text())
    natural=json.loads((HERE/'native-natural-hq-refresh.json').read_text())
    graphpath=HERE.parents[1]/'research/skirmish-20261016-saved-economy-values/actual-saved-input.json'
    graph=json.loads(graphpath.read_text())
    assert initialized['source_graph_sha256']==natural['source_graph_sha256']==sha(graphpath)
    for document in (initialized,natural):
        assert document['generator_sha256']==sha(HERE/'native_saved_refresh.py')
        assert document['original_sha256']==native.sha(native.ORIGINAL.data)
        assert document['v14_sha256']==native.sha(native.ACCEPTED.data)
    counts={'initialized':0,'natural_hq':0,'source_getters':0}
    for row in reversed(initialized['cases']):
        for image in ('v14','original'):
            case=row['input']
            for op,key in [('Production','provided'),('Upkeep','upkeep'),('Cost','cost')]:
                result=native.source_result(graph,row['actor'],op,image,case['precision'])
                assert result['values']==case[key]
                counts['source_getters']+=1
            result=native.RefreshMachine(case,image).run_actual()
            assert result==row['output']
            counts['initialized']+=1
    for row in reversed(natural['cases']):
        case=row['input'];actor=row['actor'];image=row['image'];pc=row['precision']
        assert native.source_result(graph,actor,'eligible',image,pc)['predicate'] is False
        assert native.source_result(graph,actor,'upkeep_blocked',image,pc)['predicate'] is True
        assert native.source_result(graph,actor,'Cost',image,pc)['values']==case['cost']
        counts['source_getters']+=3
        result=native.RefreshMachine(case,image,natural=True).run_actual()
        assert result==row['output']
        counts['natural_hq']+=1
    assert counts=={'initialized':6,'natural_hq':12,'source_getters':54}
    print(json.dumps({'passed':True,'counts':counts,'source_sha256':sha(HERE/'native_saved_refresh.py'),'scope':__doc__}))
if __name__=='__main__':main()
