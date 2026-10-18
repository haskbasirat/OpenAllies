"""Phase8 trigger coverage report -> coverage.json.

For every registered trigger element (115): kind, usage in the 26 campaign
missions and all decodable maps, the native 546AF0 case address and the leaf
calls it makes, the Rust module that dispatches it, and how many ORIGINAL-code
oracle cases prove it (phase7 core/world corpora + phase8 native-elements).
Status: proven (ported + oracle cases), ported (no oracle), missing.
Round 2 adds the extended module (native-extended.json) and decodes the 32
manager-version-4 developer maps (native-triggers.json proves the loader).
"""
from pathlib import Path
import collections, hashlib, json, re, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-body'))
from original_body import IMAGE  # noqa: E402
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402

SRC = ROOT / 'crates/aa-triggers/src'
SELECTORS = {0x54d9e0: 'value', 0x54da20: 'boolean', 0x54da40: 'compare', 0x54da50: 'trigger', 0x54da60: 'location',
             0x54da70: 'objective', 0x54da80: 'area', 0x54dac0: 'kingdom', 0x54dc20: 'actor', 0x54dcc0: 'camera',
             0x54de00: 'string', 0x54ded0: 'localized', 0x54df10: 'difficulty', 0x54e910: 'actor_data',
             0x54e880: 'technology', 0x54e8a0: 'vfx', 0x54e8e0: 'command', 0x54e950: 'audio', 0x54e970: 'weather',
             0x54e990: 'sun', 0x54e9b0: 'formation', 0x54e9d0: 'sai', 0x54e9f0: 'ability', 0x54db80: 'actor_id',
             0x408980: 'raw_word', 0x5422f0: 'registers', 0x541900: 'validity'}
MISSING_NOTES = {
    30: 'EffectActorCreateTransported 543BA0: transport path math (4281E0/409230/4BE560/58B5E0/58B760), '
        '4F9C10 create, 58BA90 load, 4CA760 move; not ported this round',
    77: 'Team flare on position 544A50: 496EF0 availability, 4C8C70 spawn request, cached FlareRadius var, 572E50 create',
    78: 'Team flare on actor 544BA0: same flare path with actor position (+20)',
    96: 'Ability use 544040: 4C8300(5) ability request, 4C86D0, 4C9F10, 4CE080',
}
# Leaves whose deeper engine work is one Effects seam (documented, not ported here).
SEAM_NOTES = {
    37: 'organization_modify_morale (579750) is bound by the runtime to aa_combat::morale::modify',
    111: 'fog_restore_tiles: the 554DD0/5541F0/554330 tile iterator (aa-spatial) and 54F930 per tile',
    68: 'FYL2X: binary32 store decided from binary64 log2 with a 2^-40 guard band (Unicorn FYL2X is binary64)',
    48: 'item 28 (random hero) reads the manager+2C register file; world RNG 4F02B0/85D840 is a seam',
    49: 'item 28 (random hero) reads the manager+2C register file; world RNG 4F02B0/85D840 is a seam',
}


def rust_list(path, name):
    text = (SRC / path).read_text()
    m = re.search(name + r'\s*:\s*&\[u32\]\s*=\s*&\[(.*?)\];', text, re.S)
    return {int(x) for x in re.findall(r'\d+', m.group(1))}


def case_calls(op):
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    target = struct.unpack('<I', IMAGE.at(0x548730 + op * 4, 4))[0]
    calls = []
    for ins in md.disasm(IMAGE.at(target, 600), target):
        if ins.mnemonic == 'call' and ins.op_str.startswith('0x'):
            calls.append(int(ins.op_str, 16))
        if ins.mnemonic == 'ret' or (ins.mnemonic == 'jmp' and not ins.op_str.startswith('dword')):
            break
    return target, calls


def oracle_counts():
    counts = collections.Counter()
    for path in [ROOT / 'research/phase7-triggers/native-core-program.json',
                 ROOT / 'research/phase7-triggers/native-world-program.json',
                 HERE / 'native-elements.json', HERE / 'native-extended.json']:
        doc = json.loads(path.read_text())
        for c in doc['cases']:
            counts[c['input']['element']['opcode']] += 1
    return counts


def main():
    reg = json.loads((ROOT / 'research/phase7-triggers/native-registrations.json').read_text())
    schemas = {r['opcode']: r for r in reg['element_schemas']}
    census = json.loads((HERE / 'census.json').read_text())
    modules = {}
    for path, name, label in [('program.rs', 'CORE', 'core'), ('elements.rs', 'SUPPORTED', 'elements'),
                              ('actions.rs', 'SUPPORTED', 'actions'), ('world_program.rs', 'SUPPORTED', 'world_program'),
                              ('extended.rs', 'SUPPORTED', 'extended')]:
        for op in rust_list(path, name):
            assert op not in modules, op
            modules[op] = label
    proven = oracle_counts()
    rows = []
    for el in sorted(reg['elements'], key=lambda r: r['index']):
        op, name = el['index'], el['name']
        missions, campaign_uses, all_uses, maps = set(), 0, 0, 0
        for rel, rec in census.items():
            if rec['status'] != 'decoded':
                continue
            n = sum(rec['elements'][label].get(name, 0) for label in rec['elements'])
            all_uses += n
            maps += n > 0
            if rel.startswith('Campaign/') and n:
                missions.add(rel.split('/')[-1].split('.')[0])
                campaign_uses += n
        target, calls = case_calls(op)
        leaves = [hex(c) for c in calls if c not in SELECTORS]
        module = modules.get(op)
        status = 'missing' if module is None else ('proven' if proven[op] else 'ported')
        row = {'opcode': op, 'name': name, 'kind': ['event', 'condition', 'action'][schemas[op]['kind']],
               'arguments': schemas[op]['arguments'], 'campaign_missions': len(missions),
               'campaign_instances': campaign_uses, 'all_map_instances': all_uses, 'maps_using': maps,
               'native_case': hex(target), 'native_leaf_calls': leaves,
               'native_selectors': sorted({SELECTORS[c] for c in calls if c in SELECTORS}),
               'rust_module': module, 'oracle_cases': proven[op], 'status': status,
               'missions': sorted(missions)}
        if op in MISSING_NOTES and status == 'missing':
            row['gap'] = MISSING_NOTES[op]
        if op in SEAM_NOTES and module is not None:
            row['note'] = SEAM_NOTES[op]
        rows.append(row)
    rows.sort(key=lambda r: (-r['campaign_missions'], -r['campaign_instances'], r['opcode']))
    used = [r for r in rows if r['campaign_instances']]
    summary = {
        'campaign_missions': 26,
        'element_types_registered': len(rows),
        'element_types_used_in_campaign': len(used),
        'used_types_proven': sum(r['status'] == 'proven' for r in used),
        'used_types_missing': [r['opcode'] for r in used if r['status'] == 'missing'],
        'campaign_instances': sum(r['campaign_instances'] for r in used),
        'campaign_instances_proven': sum(r['campaign_instances'] for r in used if r['status'] == 'proven'),
        'unused_types_proven': sum(r['status'] == 'proven' for r in rows if not r['campaign_instances']),
        'unused_types_missing': sum(r['status'] == 'missing' for r in rows if not r['campaign_instances']),
        'maps_decoded': sum(v['status'] == 'decoded' for v in census.values()),
        'maps_not_decoded': sorted(k for k, v in census.items() if v['status'] != 'decoded'),
        'all_map_instances': sum(r['all_map_instances'] for r in rows),
        'all_map_instances_dispatched': sum(r['all_map_instances'] for r in rows if r['rust_module']),
        'types_missing': [r['opcode'] for r in rows if r['status'] == 'missing'],
    }
    doc = {'schema': 1, 'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
           'census_sha256': hashlib.sha256((HERE / 'census.json').read_bytes()).hexdigest(),
           'summary': summary, 'elements': rows}
    (HERE / 'coverage.json').write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps(summary, indent=1))


if __name__ == '__main__':
    main()
