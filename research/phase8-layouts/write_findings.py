"""Writes findings.json from the two fixtures and the Rust sources (run from
research/phase8-layouts)."""
import json, hashlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def H(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


lay = json.loads((HERE / 'native-layout-data.json').read_text())
org = json.loads((HERE / 'native-organization-component.json').read_text())
findings = {
    'schema': 1, 'area': 'phase8-layouts', 'date': '2026-10-15',
    'goal': 'Native OrganizationLayoutData/FormationData producer and the ActorData OrganizationComponent '
            'layout binding (restoration request R2-a).',
    'rust': {
        'module': 'crates/aa-rules/src/native_layout_data.rs (aa_rules::native::layout_data)',
        'mod_line': 'crates/aa-rules/src/native.rs: #[path = "native_layout_data.rs"] pub mod layout_data;',
        'tests': 'crates/aa-rules/tests/native_layout_data.rs',
        'sha256': {'native_layout_data.rs': H(ROOT / 'crates/aa-rules/src/native_layout_data.rs'),
                   'tests/native_layout_data.rs': H(ROOT / 'crates/aa-rules/tests/native_layout_data.rs')},
    },
    'native': {
        'managers': {
            '0x464 Property': {'ctor': '0x47EFD0(0x40, mode 1)', 'set_at': '0x48B1B4', 'vf1C': '0x48B580',
                               'create': '0x48B640', 'object': '0x47D8D0 (0x60, vtable 0xA0C834)',
                               'type_load': '0x4C48C0 = 0x4C42D0 + is_technology (+5C)'},
            '0x470 OrganizationLayout': {'ctor': '0x481740(0x3D, mode 0)', 'global': '0xB3AB00', 'vf1C': '0x4818A0',
                                         'create': '0x481960', 'object': '0x4C2690 (0x54, vtable 0xA232D8)',
                                         'type_load': '0x4C3610'},
            '0x474 Formation': {'ctor': '0x481A50(0x11, mode 0)', 'vf1C': '0x481BB0', 'create': '0x481C70',
                                'object': '0x4C2540 (0x28, vtable 0xA232C4)', 'type_load': '0x4C34A0'},
            '0x478 GroupFormation': {'ctor': '0x4840A0(2, mode 0)', 'vf1C': '0x484200', 'create': '0x4842C0',
                                     'object': '0x49A660', 'type_load': 'not ported (registration only)'},
        },
        'find': '0x463FA0 = 0x48D500 (thunks 0x48D490/0x48D4A0): empty -> null, translation map +C first, '
                'registry +30 with the target only (case-insensitive)',
        'dispatch': '0x48D660 (catch funclets 0x48D824/0x48D886/0x48D8F0, log line 0x917); GameData+4 = 0x85A550(path)',
        'layout_data_fields': {
            '+8': 'IDS', '+C': 'registration ordinal', '+10/+14': 'name / named', '+18': 'primary slot (-1)',
            '+1C/+20/+24': '16-byte entries {+0 name, +4 Property*, +8 number, +C required, +D locked, +E morale_line}; '
                           'ctor 0x4C2250 defaults (name "", 0, 1, 1, 0, 0)',
            '+28/+2C/+30': 'entry index per slot',
            '+34/+38/+3C': 'LayoutFormation* (0x1C, vtable 0xA232A8: +8 layout, +C FormationData*, +10/+14/+18 '
                           '12-byte positions {Property*, x, y})',
            '+40': 'default formation index', '+44': 'combat formation index', '+48': 'retreat formation index',
            '+4C': 'morale-line number sum', '+50': 'BrokenProperty ModifierPropertyData* (0x461ED0, 0x5C bytes)'},
        'formation_data_fields': {
            '+8': 'IDS', '+C': 'ordinal', '+10/+14': 'name / named', '+18': 'organized (ctor 1)', '+19': 'herding',
            '+1A': 'aggressive', '+1C': 'texture (0x4763C0 path -> 0x5A77E0)', '+20': 'GroupFormation*',
            '+24': 'Property ModifierPropertyData*'},
        'functions': {
            '0x4C3610': 'KKC_OrganizationLayoutData::LoadData', '0x4C32B0': 'LayoutFormation load',
            '0x4C34A0': 'KKC_FormationData::LoadData', '0x4C1BE0': 'CheckFormationMatch (0x404ED0 lines 0xCB/0xD5)',
            '0x4C3110': 'formation index by IDS', '0x4763C0': 'icon path', '0x85A550': 'directory of a path',
            '0x4C4CF0': 'ActorData +1D4 property list',
            '0x466C4F..0x46707C': 'ActorData OrganizationComponent block (464BD0)',
            '0x4C2110': 'KKC_OrganizationLayout ctor', '0x4C2310': 'set layout parameter (0x45EEF0 gate)',
            '0x473AD0': 'deferred reference (GameData+824 list)', '0x475230': 'deferred walk (head first)',
            '0x461750/0x460000': 'element link', '0x45EEF0': 'ActorData has property'},
        'globals': {'0xB3B6B0': 'max slot number (.bss, 0)', '0xB3B6B4': 'max entry count (.bss, 0)'},
        'constants': {'0x9FC064': '0x3C8EFA35 (degrees -> radians)', '0x9F32DC': '0.0'},
    },
    'oracles': {
        'native-layout-data.json': {'producer': 'native_layout_data.py', 'producer_sha256': lay['producer_sha256'],
                                    'machine_sha256': lay['machine_sha256'], 'shipped_runs': len(lay['shipped']),
                                    'cases': len(lay['cases']), 'native_windows': len(lay['native_windows']),
                                    'file_sha256': H(HERE / 'native-layout-data.json')},
        'native-organization-component.json': {'producer': 'native_organization_component.py',
                                               'producer_sha256': org['producer_sha256'], 'cases': len(org['cases']),
                                               'native_windows': len(org['native_windows']),
                                               'file_sha256': H(HERE / 'native-organization-component.json')},
    },
    'shipped_result': {
        'properties_before_layouts': 160, 'properties_total_v14': 165,
        'group_formations': ['group_aggressive', 'group_defensive', 'group_column'],
        'formations': ['regiment_aggressive', 'regiment_defensive', 'regiment_column', 'regiment_herding', 'regiment_hq'],
        'layouts': ['herd_ambient_huge', 'herd_ambient_large', 'herd_ambient_single', 'regiment_hq',
                    'regiment_hq_mobile', 'regiment_infantry', 'regiment_mechanized', 'regiment_single_unit',
                    'regiment_vehicle_heavy', 'regiment_vehicle_light'],
        'max_slot_number_b3b6b0': 4, 'max_entry_count_b3b6b4': 6, 'native_errors': 0,
        'organization_definitions': 192,
        'organization_definitions_by_layout': {'herd_ambient_large': 4, 'herd_ambient_single': 12, 'regiment_hq': 20,
                                               'regiment_hq_mobile': 20, 'regiment_infantry': 64,
                                               'regiment_mechanized': 13, 'regiment_single_unit': 4,
                                               'regiment_vehicle_heavy': 23, 'regiment_vehicle_light': 32},
        'precision_dependence': 'none for shipped data (PC24/53/64 identical)',
    },
    'saved_payloads': {'organizations': 1082, 'layout_resolves': 1082, 'parameter_count_matches': 1082,
                       'records_with_unknown_parameter_ids': 143, 'maps_with_them': 29, 'slot_count_differs': 2},
    'proven': [
        'Formation/OrganizationLayout/Property/GroupFormation registration and Find (shipped files at PC24/53/64 + synthetic)',
        '4C3610 complete, incl. the partial state after every throw, the 4C1BE0 logs and the [B3B6B0]/[B3B6B4] globals',
        '4C32B0, 4C1BE0, 4C3110, 4C34A0 (except the recorded 4C42D0 seam)',
        '4763C0/85A550 icon path', '48D660 abandon-rest-of-file for these managers',
        '466C4F..46707C incl. the x87 +304 and rampage_direction products at PC24/53/64 (incl. PC24 double rounding)',
        '460000 incl. walk order, the 4C2310 store gate and the +17C merge'],
    'compiled_and_real_data_only': [
        'NativeLayoutData over the full v14 native-order catalog (192 organization definitions, 0 native errors)',
        'native_actor_properties (+1D4) for real actors (4C4CF0 itself not executed under Unicorn)',
        'element_facts: +174 bits 8/19/21/30 and +558 from component / ResearchItem presence'],
    'not_ported': ['0x4C42D0 KKC_ModifierPropertyData::LoadData (seam)', 'GroupFormation type LoadData',
                   '0x5A77E0 texture load (presentation)', '0x4B9F40 string table (caller supplies the lookup)',
                   '48D2D0 throw message formats for translations of these managers (Unsupported)',
                   "other managers' throws abandoning roots in the same file (Unsupported)"],
    'removed_caps': [],
    'requests': 'requests-to-lead.md L-1..L-5',
}
(HERE / 'findings.json').write_text(json.dumps(findings, indent=1))
print('findings.json written')
