"""Read-only preservation/payload audit after the explicit diagnostic-mode change."""
import json
from pathlib import Path
import package

plan=package.read(package.HERE/'payload-plan-draft.json')
package.verify_rows(package.ROOT,plan['previous_release_files'])
package.verify_rows(package.ROOT,plan['cache_files'])
log=(package.HERE/'self-tests-diagnostic.log').read_text(encoding='utf-8-sig')
assert 'Ran 11 tests' in log and '\nOK\n' in log
files=['package.py','test_package.py','README.md','self-tests-diagnostic.log','audit_diagnostic.py']
report=dict(schema=1,scope='Diagnostic packaging helper only; no real build/stage/launch/seal executed by this audit.',
    synthetic_tests_passed=11,previous_release_files_verified=len(plan['previous_release_files']),
    previous_releases_unchanged=True,initial_cache_plan_files_still_hash_identical=len(plan['cache_files']),
    real_dev10_exists_at_audit=package.RELEASE.exists(),
    strict_playable_seal_preserved=True,diagnostic_battle_status='unavailable',
    diagnostic_archive=package.DIAGNOSTIC_ARCHIVE,
    required_validation_flags=dict(release_mode='diagnostic',battle_status='unavailable',playable_battle=False,
        full_game_parity=False,unimplemented_battle_ui_disabled=True,claims_native_battle=False),
    required_checks=['architecture-dependencies','asset-verification','package-cli','render-review','disabled-battle-ui','independent-review'],
    files=[package.record(package.HERE/name) for name in files])
package.write_new(package.HERE/'diagnostic-mode-audit.json',report)
print(json.dumps(dict(tests=11,old_release_files=len(plan['previous_release_files']),
    cache_files=len(plan['cache_files']),real_release_created_by_audit=False,
    report_sha256=package.sha(package.HERE/'diagnostic-mode-audit.json'))))
