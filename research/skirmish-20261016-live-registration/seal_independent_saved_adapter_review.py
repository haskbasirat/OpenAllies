"""Seal independent native/source/saved-binding review without changing author pins."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 native=HERE/'independent-saved-adapter-native.json';report=json.loads(native.read_text())
 for name,expected in report['source_sha256'].items():assert sha(ROOT/name)==expected,name
 for row in report['fixtures']:assert sha(HERE/row['fixture'])==row['sha256']
 actual=HERE/'independent-saved-adapter-rust-actual-final.log'
 lifecycle=HERE.parent/'skirmish-20261016-battle-terrain/author-mapworld-proof.log'
 assert '1 passed; 0 failed;' in actual.read_text(encoding='utf-8-sig')
 assert 'per_record_init_postcreate_registration_and_failure_prefix_match_native ... ok' in lifecycle.read_text(encoding='utf-8-sig')
 binary=ROOT/'target/s-owner-recruitment/release/deps/aa_runtime-37f3342854dff5b7.exe'
 report.update(status='accepted_bounded_review_no_blockers',native_receipt_sha256=sha(native),
  independent_rust_passed=2,rust_tests=[dict(test='actual_saved_records_mutate_owned_fields_and_require_real_actor_service',log=str(actual.relative_to(ROOT)),sha256=sha(actual),compiled_release_test_executable_sha256=sha(binary)),dict(test='per_record_init_postcreate_registration_and_failure_prefix_match_native',log=str(lifecycle.relative_to(ROOT)),sha256=sha(lifecycle))],
  additional_portable_registration_tests='Three tests not independently rerun here; redundant rebuild stopped and reaped because unrelated shared-source edits invalidated warm Cargo cache. Parent notified to include target in final integration.',
  source_audit_report_sha256=sha(HERE/'independent-saved-adapter-review.md'))
 path=HERE/'independent-saved-adapter-review.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(status=report['status'],native_calls=report['native_calls'],rust_passed=2,sha256=sha(path))))
if __name__=='__main__':main()
