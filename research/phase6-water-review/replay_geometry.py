"""Independent read-only replay of native region-prefix and saved-water arrays."""
from pathlib import Path
from unittest.mock import patch
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-water-geometry'))
import native_water_geometry as native
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
producer=ROOT/'research/phase6-water-geometry/native_water_geometry.py'
fixture=ROOT/'research/phase6-water-geometry/native-water-geometry.json'
source=ROOT/'crates/aa-client/src/water_geometry.rs'
before={str(p.relative_to(ROOT)):sha(p)for p in [producer,fixture,source]}
captured=[]
with patch.object(Path,'write_text',lambda self,text,*a,**kw:captured.append((self,text))):
    native.main()
assert len(captured)==1
expected=json.loads(fixture.read_text());actual=json.loads(captured[0][1])
actual.pop('seconds');expected.pop('seconds')
assert actual==expected
assert before=={str(p.relative_to(ROOT)):sha(p)for p in [producer,fixture,source]},'source changed during replay'
(HERE/'independent-native-water-geometry.json').write_text(json.dumps(actual,indent=2)+'\n')
(HERE/'geometry-replay.json').write_text(json.dumps({'passed':True,'maps':len(actual['maps']),
    'source_sha256':before,'output_sha256':sha(HERE/'independent-native-water-geometry.json'),
    'scope':'Reexecuted original region stream/read/cache/lookup paths for all80 source suffixes and compared every retained array/entry hash and byte offset; omitted only wall-clock duration from fixture equality.'},indent=2)+'\n')
print(json.dumps({'passed':True,'maps':len(actual['maps']),'report_sha256':sha(HERE/'geometry-replay.json')}))
