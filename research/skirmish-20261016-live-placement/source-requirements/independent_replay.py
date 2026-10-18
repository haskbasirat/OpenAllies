"""Read-only fresh replay of the frozen source-requirements native producer.
Intercept its one fixture write and demand byte equality with the frozen file.
Only this reviewer's summary file is written; original fixtures are immutable.
"""
from pathlib import Path
import hashlib, importlib.util, json, os
HERE=Path(__file__).resolve().parent
fixture=HERE/'native-source-requirements.json'
expected=fixture.read_bytes();digest=hashlib.sha256(expected).hexdigest()
producer=HERE/'native_source_requirements.py'
spec=importlib.util.spec_from_file_location('independent_native_source_requirements',producer)
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
original=Path.write_text
observed=[]
def protected_write(path,text,*args,**kwargs):
    assert path.resolve()==fixture.resolve(),f'unexpected producer write: {path}'
    payload=text.replace('\n',os.linesep).encode(kwargs.get('encoding')or'utf-8')
    assert payload==expected,'fresh native replay differs from frozen fixture'
    observed.append(len(payload))
    return len(text)
Path.write_text=protected_write
try:module.main()
finally:Path.write_text=original
assert observed==[len(expected)]
assert fixture.read_bytes()==expected
print(json.dumps({'independent_read_only_replay':True,'fixture_sha256':digest,'bytes_compared':len(expected)}))
