"""Write research/phase8-economy/findings.json (hashes, counts, scope)."""
from pathlib import Path
import hashlib, json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
OWNED = ['crates/aa-economy/src/*.rs', 'crates/aa-economy/tests/*.rs', 'crates/aa-economy/tests/fixtures/*.json',
         'crates/aa-economy/README.md', 'crates/aa-production/src/*.rs', 'crates/aa-production/tests/*.rs',
         'crates/aa-runtime/src/kingdom_economy.rs', 'crates/aa-runtime/src/economy_refresh.rs',
         'crates/aa-runtime/src/economy_values_world.rs', 'crates/aa-runtime/src/actor_economy_focus.rs',
         'research/phase8-economy/*.py', 'research/phase8-economy/*.json', 'research/phase8-economy/*.txt',
         'research/phase8-economy/*.md']


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    inventory = json.loads((HERE / 'inventory.json').read_text(encoding='utf-8'))
    oracles = {}
    for fixture in sorted(HERE.glob('native-*-fixtures.json')):
        doc = json.loads(fixture.read_text(encoding='utf-8'))
        name = fixture.stem.removeprefix('native-').removesuffix('-fixtures')
        entry = {'pins': doc['pins']}
        if 'fixtures' in doc:
            entry['fixtures'] = len(doc['fixtures'])
            entry['native_executions'] = doc['executions']
        else:
            entry['case_runs'] = len(doc['cases'])
            entry['steps'] = sum(len(c['steps']) for c in doc['cases'])
        if 'seams' in doc:
            entry['seams'] = doc['seams']
        oracles[name] = entry
    sources = sorted({p.relative_to(ROOT).as_posix() for pattern in OWNED for p in ROOT.glob(pattern)}
                     - {'research/phase8-economy/findings.json'})
    doc = {
        'schema': 2,
        'phase': 'phase8-economy',
        'original_sha256': '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
        'v14_sha256': '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
        'fpu_policy': 'game simulation thread runs at Double53 (CW 0x027F, research/phase8-tick/fpu-precision.md); '
                      'every oracle runs Single24, Double53 and Extended64; no pinned range executes a '
                      'transcendental x87 instruction',
        'oracles': oracles,
        'precisions': ['Double53 (0x027F, game default)', 'Single24 (0x007F)', 'Extended64 (0x037F)'],
        'inventory_summary': inventory['summary'],
        'removed_caps': [
            {'address': '0x48BA89', 'original': 10, 'what': 'stat data manager resource count; statistics production totals inline at stats+0 (float[10])',
             'rust': 'aa_economy::statistics::Statistics::production is a Vec sized to the registry'},
        ],
        'proven_behaviours': [
            'Upkeep-shortage ratio 1 - provided/consumed over STOCKPILED rows plus min(provided,consumed)*cost for UPKEEP rows, x87 accumulators, event 3 percent = _ftol(PC(ratio*100)) only when |ratio-announced| > 0.05f (or previous ratio zero) and ratio > 0; event 4 when previous ratio nonzero and no new overdraw; PC24 vs PC53/64 percent differences are reproduced (e.g. 0.29f reports 28 at PC53, the game default, and 29 at PC24).',
            'Recruit-single regeneration: eligible when available or both GTIDs dead; one charging index; ActorData+2CC*seconds; index-0 completion leaves charging = 0 (native quirk).',
            'Commander (kingdom+0x230) experience income seconds*cache+0x180, clamp to [0, FactionData+0x3C] when max >= 0, score 44 only for positive amounts; 4E0FD0 multiplier path.',
            'Statistics: 60 s snapshot timer, production totals and TECH_BUILDINGS_POSSESSION_TIME += dt*TECH_BUILDINGS_OWNED; relation statistics +0x340 slots 0..4 keyed by definition (58D8C0, indices 57..61) and 5..9 keyed by kingdom (58D950, slot index-52).',
            'Research request/refund/completion, requirements 5842F0 (outcome 0/1/2 with missing-item description), kingdom AddProperty 4E3AD0 and property removal 4E23D0/5034F0.',
            'Steal and give resources, tribute (team command 4) with events 0x1F/0x20 even when defeated, faction ability affordability.',
            'Construction: construct/upgrade affordability, upgrade start/cancel, construction cancel; BuildActor 563160 quotes and debits only after creation and never stores the paid cost; settlement BuildStructure 587FD0 quotes before creation, debits the reread owner and stores the paid cost with 5655C0(cost, 1); enemy-nearby messages are sent whatever the notify flag.',
            'Construction lifecycle: salvage floor(double(PC(PC(fraction*paid)+0.5))) per row, sell = cancel upgrade then refund+destroy unless destroyed, command dispatch 0x13..0x16, Tick auto-upgrade (501570 interval timer, first 566110(def,1,0) passing) and state 0x20 = (constructing or upgrading) and PC(time - start) < window.',
            'Supply: EconomyComponent::Tick 56F2B0, supply component tick 58AE10, relay chains 58ADB0 (cycles rejected), in-supply 4D0A80, state 4D7CE0.',
            'Kingdom registry AddActor 4E6180 / RemoveActor 4E44E0 and the recruit item table 4E2A60/4E2CB0/4E4C30 used by ChangeKingdom 0x4D8450.',
            'Purchases: denizen air attack/recon, HQ attach cost/gate/payment, transport unload refund.',
            'Construction completion 566AC0/565B00/566400: settlement founding, CreateComponents failure, kingdom completion 4E4470, statistics, events, triggers, component hand-offs and the 565700 refund on a failed upgrade, in native order.',
            'Start mission 4E6DB0: stockpile reset (+0 unless > 0, clamped), Commander/profile/start technologies with owned filtering, credits, faction diagnostic, Commander starting experience clamp 4CDF10, including NaN/infinite inputs.',
            'Restrictions 4E1910/4E1810/4E1890 over native string maps: ASCII case-insensitive keys, last inserted entry wins, null values do not restrict.',
            'Settlement slots 585500/5890C0/586B40.',
            'Structure enable 565400 / disable 564DC0 / command recompute 565140 (565090, 585F00, 4D14C0, 517FA0, 584E10, 584DE0) with actor-state writes as seams.',
            'World start 4F1E10 economy parts: promotion and mission score reset, actor+0x13C = 0x38 and owner statistics tracking walk, forced initial snapshots.',
            'Harness: page-0 accesses fault unless FS-prefixed (Unicorn FS base 0); three silent null reads fixed, one fixture lost 57 out-of-domain executions.',
            'Kingdom load economy semantics: 58EE80/58DBE0 statistics loads, 4E3690 promotion + 58EA00 mission score reset, v7+ property loop (source 1..3, legacy world < 0x15 discard with one log).',
        ],
        'source_sha256': {s: sha(ROOT / s) for s in sources},
    }
    (HERE / 'findings.json').write_text(json.dumps(doc, indent=1) + '\n', encoding='utf-8')
    print('findings.json written', len(sources), 'sources', len(oracles), 'oracles')


if __name__ == '__main__':
    main()
