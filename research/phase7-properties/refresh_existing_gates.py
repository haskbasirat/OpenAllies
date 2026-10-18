"""Bind unchanged constructor/promotion scopes to the final full aa-body run.

This does not rerun native fixtures, broaden their scope or claim peer review.
Their implementation and test hashes must still equal their preceding reports.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
validation=json.loads((HERE/'effects-findings.json').read_text());assert validation['passed']
for path,value in validation['source_sha256'].items():assert sha(ROOT/path)==value,path
for path,value in validation['dependency_sha256'].items():assert sha(ROOT/path)==value,path
for command in validation['commands']:assert sha(HERE/command['log'])==command['sha256']
for report_path,implementation,test in [
    (HERE/'findings.json','crates/aa-body/src/properties.rs','crates/aa-body/tests/native_properties.rs'),
    (ROOT/'research/phase7-promotion/findings.json','crates/aa-body/src/promotion.rs','crates/aa-body/tests/native_promotion.rs'),
]:
    report=json.loads(report_path.read_text())
    for path in [implementation,test]:assert sha(ROOT/path)==report['source_sha256'][path],path
    report['commands']=[{**c,'log':(HERE/c['log']).relative_to(ROOT).as_posix()} for c in validation['commands']]
    for field in ['source_sha256','dependency_sha256']:
        for path in report[field]:report[field][path]=sha(ROOT/path)
    report['checkpoint_rebind']={'producer':Path(__file__).relative_to(ROOT).as_posix(),'producer_sha256':sha(Path(__file__)),'scope':'Implementation and native comparison test unchanged; added sibling module exports and documentation are rebound after the final22-test aa-body run and strict Clippy. No new native cases or peer acceptance implied.','full_test_count':22}
    if implementation.endswith('properties.rs'):
        for name in ['inspect_domain.py','domain-evidence.json']:report['source_sha256'][(HERE/name).relative_to(ROOT).as_posix()]=sha(HERE/name)
    report_path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'path':report_path.relative_to(ROOT).as_posix(),'sha256':sha(report_path)}))
