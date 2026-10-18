"""Independent native water color/topology/selection replay without source writes."""
from pathlib import Path
from unittest.mock import patch
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-water'))
import recover_water as native
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
paths=[ROOT/'research/phase6-water'/name for name in ['recover_water.py','water-metadata.json','native-water-vectors.json']]
before={str(p.relative_to(ROOT)):sha(p)for p in paths}
captured=[]
with patch.object(Path,'write_text',lambda self,text,*a,**kw:captured.append((self,text))):native.main()
assert len(captured)==2
for path,text in captured:
    actual=json.loads(text);expected=json.loads(path.read_text())
    assert actual==expected,path
    (HERE/f'independent-{path.name}').write_text(json.dumps(actual,indent=2)+'\n')
assert before=={str(p.relative_to(ROOT)):sha(p)for p in paths}
vectors=json.loads(paths[2].read_text())
report={'passed':True,'source_sha256':before,'native_curve_cases':len(vectors['curve']),
    'native_topology_cases':len(vectors['regular_quads']),'native_selection_cases':len(vectors['selection']),
    'scope':'Every original color-curve, regular-cell emission and water-cell selector case reexecuted and compared in memory; authored metadata regenerated identically, no production/source fixture overwrite.'}
(HERE/'shading-replay.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
