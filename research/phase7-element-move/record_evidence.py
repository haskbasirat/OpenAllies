"""Pin ElementMove implementation/proof scope; no self-approval."""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).parent
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 fixture=json.loads((HERE/'native-element-move.json').read_text());validation=json.loads((HERE/'validation-record.json').read_text())
 assert fixture['producer_sha256']==H(HERE/'native_element_move.py') and validation['passed']
 assert len(fixture['cases'])==384 and len(fixture['leaf_cases'])==288
 n=len(fixture['cases'])//3;witnesses=sum(fixture['cases'][i]['output']!=fixture['cases'][i+n]['output']or fixture['cases'][i]['output']!=fixture['cases'][i+2*n]['output']for i in range(n));assert witnesses==6
 source=['crates/aa-mind/src/element_move.rs','crates/aa-mind/tests/element_move_native.rs']
 evidence=['README.txt','native_element_move.py','native-element-move.json','rust-tests.log','clippy.log','validation-record.json','record_evidence.py']
 report={'schema':1,'passed':True,'scope':'Concrete ElementMove Think, entry, exit and small gates; required live services remain explicit',
 'native_state_cases':384,'native_numeric_cases':288,'cross_precision_branch_witnesses':6,'rust_tests_passed':3,'validation':validation['runs'],
 'source_sha256':{p:H(ROOT/p)for p in source}|{str((HERE/p).relative_to(ROOT)).replace('\\','/'):H(HERE/p)for p in evidence},
 'dependency_sha256':{p:H(ROOT/p)for p in ['crates/aa-mind/src/lib.rs','crates/aa-navigation/src/lib.rs','crates/aa-navigation/src/result_plan.rs','crates/aa-numeric/src/lib.rs','research/phase4-rng/original_rng.py']},
 'limits':['Original outer operation executes, but fixture services are explicit boundaries for world/component/path virtual behavior; first-time SVar registration is not replayed.',
 'Active state and retained object identity lifetime must survive callbacks; completed effects are not rolled back on failure.',
 'Runtime must compose actual ResultPlan/continuation/geometry/components; no playable full-world or multiplayer claim.',
 'Finite arithmetic, explicit PC24/53/64 and ordinary source component representation; no guessed constructor defaults.'],
 'independent_review':'Pending separate peer; producer does not self-approve.'}
 p=HERE/'findings.json';p.write_text(json.dumps(report,indent=2)+'\n');print(H(p))
if __name__=='__main__':main()
