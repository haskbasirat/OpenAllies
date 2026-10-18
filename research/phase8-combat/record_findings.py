"""Record findings.json: fixture, producer, harness and Rust source pins."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

FIXTURES = {
    'native-morale-fixtures.json': 'native_morale.py',
    'native-experience-amount-fixtures.json': 'native_experience_amounts.py',
    'native-roe-fixtures.json': 'native_roe.py',
    'native-damage-response-fixtures.json': 'native_damage_responses.py',
    'native-combat-reaction-fixtures.json': 'native_combat_reactions.py',
    'native-capture-attempt-fixtures.json': 'native_capture_attempt.py',
    'native-org-tick-fixtures.json': 'native_org_tick.py',
    'native-member-removal-fixtures.json': 'native_member_removal.py',
    'native-siege-fixtures.json': 'native_siege.py',
    'native-denizen-fixtures.json': 'native_denizen.py',
    'native-change-kingdom-fixtures.json': 'native_change_kingdom.py',
    'native-damage-number-fixtures.json': 'native_damage_numbers.py',
    'native-crt-pow-fixtures.json': 'native_crt_pow.py',
    'native-combat-value-fixtures.json': 'native_combat_value.py',
    'native-statistics-tracking-fixtures.json': 'native_statistics_tracking.py',
    'native-regeneration-fixtures.json': 'native_regeneration.py',
    'native-entrench-delay-fixtures.json': 'native_entrench_delay.py',
    'native-org-speed-fixtures.json': 'native_org_speed.py',
    'native-property-wrappers-fixtures.json': 'native_property_wrappers.py',
    'native-ability-effect-fixtures.json': 'native_ability_effects.py',
    'native-begin-death-fixtures.json': 'native_begin_death.py',
    'native-wall-fixtures.json': 'native_walls.py',
    'native-member-spawn-fixtures.json': 'native_member_spawn.py',
    'native-turret-intention-fixtures.json': 'native_turret_intention.py',
    'native-special-ops-loading-fixtures.json': 'native_special_ops_loading.py',
    'native-special-ops-effects-fixtures.json': 'native_special_ops_effects.py',
    'native-summon-fixtures.json': 'native_summon.py',
    'native-wall-geometry-fixtures.json': 'native_wall_geometry.py',
    'native-member-extras-fixtures.json': 'native_member_extras.py',
    'native-modifier-tables-fixtures.json': 'native_modifier_tables.py',
}
RESULT = '119 passed, 0 failed (aa-combat 69, aa-body 37, aa-numeric 13)'
RULES_RESULT = ('aa-rules lib abilities: 10 passed (native_fixtures 4, special_ops_fixtures 4, original_data 2: '
                '363 actors; 24 Factions / 88 special-operation abilities)')
EXTRA_SECTIONS = ('libm_cases', 'dispatch_cases', 'power_cases', 'mean_cases', 'performer_cases',
                  'definition_cases', 'enums', 'modifiers', 'vfx', 'permanent', 'create_vfx',
                  'enchantment_property', 'sieges', 'transfers', 'intruders', 'detaches', 'summons',
                  'required', 'ability_required', 'heal_modifier', 'heal', 'enchant', 'capture', 'describe',
                  'find', 'find_wrapper', 'expiry', 'propagation', 'rank')
RUST = ['crates/aa-combat/src/' + n for n in
        ['lib.rs', 'morale.rs', 'roe.rs', 'capture.rs', 'death.rs', 'engagement.rs', 'shortfall.rs',
         'reactions.rs', 'commander.rs', 'payment.rs', 'admission.rs', 'organization.rs', 'ownership.rs',
         'siege.rs', 'denizen.rs', 'presentation.rs', 'combat_value.rs', 'statistics_tracking.rs',
         'regeneration.rs', 'begin_death.rs', 'walls.rs', 'performer.rs', 'projectile.rs',
         'special_ops.rs', 'summon.rs', 'execution.rs', 'periodic.rs', 'admission.rs']] + \
       ['crates/aa-body/src/' + n for n in ['lib.rs', 'experience_amounts.rs', 'credit.rs', 'damage.rs',
                                            'property_api.rs', 'properties.rs', 'modifier_tables.rs',
                                            'property_flags.rs', 'property_effects.rs']] + \
       ['crates/aa-runtime/src/economy_values_world.rs'] + \
       ['crates/aa-numeric/src/crt_pow.rs', 'crates/aa-rules/src/abilities.rs',
        'crates/aa-rules/src/ability_load.rs']
TESTS = ['crates/aa-combat/tests/' + n for n in
         ['native_morale.rs', 'native_roe.rs', 'native_damage_responses.rs', 'native_reactions.rs',
          'native_capture.rs', 'native_commander.rs', 'native_payment.rs', 'native_org_tick.rs',
          'native_member_removal.rs', 'native_siege.rs', 'native_denizen.rs', 'native_change_kingdom.rs',
          'native_damage_numbers.rs', 'native_combat_value.rs', 'native_statistics_tracking.rs',
          'native_regeneration.rs', 'native_entrench_delay.rs', 'native_org_speed.rs',
          'native_begin_death.rs', 'native_walls.rs', 'native_member_spawn.rs',
          'native_turret_intention.rs', 'native_special_ops.rs', 'native_summon.rs',
          'native_member_extras.rs']] + \
        ['crates/aa-body/tests/' + n for n in ['native_experience_amounts.rs', 'native_credit.rs',
                                              'native_property_wrappers.rs', 'native_modifier_power.rs',
                                              'native_modifier_tables.rs', 'property_tables_integration.rs',
                                              'native_properties.rs']]


def main():
    fixtures = []
    total = 0
    for name, producer in FIXTURES.items():
        doc = json.loads((HERE / name).read_text())
        assert doc['passed'] and doc['producer_sha256'] == sha(HERE / producer), name
        assert doc['harness_sha256'] == sha(HERE / 'harness.py'), name
        rows = list(doc['cases'])
        extra = {k: len(doc[k]) for k in EXTRA_SECTIONS if k in doc}
        for k in extra:
            if isinstance(doc[k], list):
                rows += doc[k]

        def precision(c):
            if isinstance(c, dict):
                return (c.get('input') or {}).get('precision') or c.get('precision')
            return None
        precisions = sorted({x for x in map(precision, rows) if x}) or ['n/a (SSE2 binary64)']
        total += len(rows)
        fixtures.append({'file': name, 'sha256': sha(HERE / name), 'producer': producer,
                         'producer_sha256': doc['producer_sha256'], 'cases': len(doc['cases']),
                         'extra_sections': extra, 'precisions': precisions,
                         'native_windows': len(doc['native_windows'])})
    inventory = json.loads((HERE / 'inventory.json').read_text())
    findings = {
        'schema': 1,
        'original_sha256': '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
        'accepted_sha256': '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
        'harness_sha256': sha(HERE / 'harness.py'),
        'fixtures': fixtures,
        'fixture_cases_total': total,
        'inventory_sha256': sha(HERE / 'inventory.json'),
        'inventory_counts': inventory['counts'],
        'settings_index_sha256': sha(HERE / 'settings-index.json'),
        'special_ops_census': {'file': 'special-ops-census.json', 'producer': 'special_ops_census.py',
                               'sha256': sha(HERE / 'special-ops-census.json'),
                               'producer_sha256': sha(HERE / 'special_ops_census.py')},
        'rust_sources': {p: sha(ROOT / p) for p in RUST},
        'rust_tests': {p: sha(ROOT / p) for p in TESTS},
        'validation': {
            'command': 'CARGO_TARGET_DIR=target/s-combat cargo test -p aa-combat -p aa-body -p aa-numeric '
                       '--release --locked --offline -j 4',
            'result': RESULT,
            'rules_command': 'CARGO_TARGET_DIR=target/s-combat cargo test -p aa-rules --release --locked '
                             '--offline -j 4 --lib abilities',
            'rules_result': RULES_RESULT,
            'rules_clippy': 'cargo clippy -p aa-rules --lib --tests --no-deps -- -D warnings: no lint in '
                            'abilities.rs/ability_load.rs; the only remaining lint is native_load.rs:878 '
                            '(manual Option::map, not owned here)',
            'clippy': 'CARGO_TARGET_DIR=target/s-combat cargo clippy -p aa-combat -p aa-body -p aa-numeric '
                      '--release --locked --offline -j 4 --all-targets --no-deps -- -D warnings: exit 0',
        },
    }
    (HERE / 'findings.json').write_text(json.dumps(findings, indent=1) + '\n')
    print(json.dumps({'fixtures': len(fixtures), 'cases': total}))


if __name__ == '__main__':
    main()
