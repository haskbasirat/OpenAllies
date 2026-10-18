"""Reviewer replay of stored inputs in reverse order, without rewriting fixtures."""
from pathlib import Path
import hashlib,importlib,json,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
ledger=json.loads((HERE/'findings.json').read_text())
for relative,expected in ledger['sha256'].items():
    assert sha(ROOT/relative)==expected,relative
rows=[]
for module_name,fixture_name,expected_count in [
    ('native_weapon','native-weapon-fixtures.json',486),
    ('native_periodic','native-periodic-fixtures.json',90),
    ('native_damage_types','native-damage-type-fixtures.json',8)]:
    module=importlib.import_module(module_name)
    document=json.loads((HERE/fixture_name).read_text())
    assert len(document['cases'])==expected_count
    for dependency,expected in document['oracle_dependencies'].items():
        assert sha(ROOT/dependency)==expected,dependency
    assert sha(HERE/(module_name+'.py'))==document['producer_sha256']
    for row in reversed(document['cases']):
        machine=module.Machine(row['input'])
        actual=machine.run()
        assert actual==row['output'],(module_name,row['input']['name'])
    # Check the producer's observed instruction windows independently against
    # both images, including explicitly intercepted seam entry addresses.
    for window in document['visited_windows']:
        address=int(window['address'],16);raw=bytes.fromhex(window['bytes'])
        if module_name=='native_damage_types':
            assert module.original.range(address,len(raw))==raw
            assert module.modified.range(address,len(raw))==raw
        else:
            assert module.body.IMAGE.at(address,len(raw))==raw
            assert module.body.ACCEPTED.at(address,len(raw))==raw
    rows.append({'module':module_name,'cases_reexecuted':expected_count,
                 'observed_instruction_windows_verified':len(document['visited_windows']),
                 'fixture_sha256':sha(HERE/fixture_name)})
for relative,expected in ledger['sha256'].items():
    assert sha(ROOT/relative)==expected,relative
print(json.dumps({'passed':True,'cases':sum(row['cases_reexecuted'] for row in rows),
                  'ledger_sha256':sha(HERE/'findings.json'),'rows':rows},indent=2))
