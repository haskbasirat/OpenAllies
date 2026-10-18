"""Write findings.json for phase8 triggers from the corpora and coverage report."""
from pathlib import Path
import hashlib, json
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def main():
    elements = json.loads((HERE / 'native-elements.json').read_text())
    fmt = json.loads((HERE / 'native-format.json').read_text())
    extended = json.loads((HERE / 'native-extended.json').read_text())
    strings = json.loads((HERE / 'native-mapstrings.json').read_text())
    loads = json.loads((HERE / 'native-triggers.json').read_text())
    coverage = json.loads((HERE / 'coverage.json').read_text())
    seam_calls = sorted({c['call'] for case in elements['cases'] for c in case['output'].get('calls', [])})
    doc = {
        'schema': 1,
        'scope': 'Original AA.exe trigger system: all 115 registered elements (546AF0 dispatch, selectors, '
                 'TriggerElement leaves, Kingdom objective/mission-result methods, CRT command formatting), the TRIGGERS '
                 'loader (54D3D0, manager v4/v5) and the MAPSTRINGS loader and localized binding (47EA40, 49AD70).',
        'original_sha256': elements['original_sha256'],
        'v14_sha256': elements['v14_sha256'],
        'fpu': 'Precision control is an explicit input (CoreProgram::precision); callers pass PC53 (world tick). '
               'Numeric opcodes are proven at PC24/PC53/PC64. Opcode 68 (FYL2X): the binary32 store is proven; FYL2X '
               'is extended precision and not correctly rounded, Unicorn computes it via binary64, so Rust refuses '
               'results within 2^-40 of a binary32 rounding boundary. Camera angles and 4454A0 rotation are seams.',
        'corpora': [
            {'file': 'native-elements.json', 'sha256': sha(HERE / 'native-elements.json'), 'cases': len(elements['cases']),
             'visited_instruction_windows': len(elements['visited_windows']),
             'rust_unsupported_cases': sum(1 for c in elements['cases'] if c['input'].get('rust_unsupported')),
             'recorded_seam_calls': seam_calls},
            {'file': 'native-format.json', 'sha256': sha(HERE / 'native-format.json'), 'cases': len(fmt['cases']),
             'scope': fmt['scope']},
            {'file': 'native-extended.json', 'sha256': sha(HERE / 'native-extended.json'), 'cases': len(extended['cases']),
             'native_faults': sum(1 for c in extended['cases'] if 'fault' in c['output']),
             'rust_unsupported_cases': sum(1 for c in extended['cases'] if c['input'].get('rust_unsupported')),
             'scope': 'the 26 element types no campaign mission uses (round 2)'},
            {'file': 'native-mapstrings.json', 'sha256': sha(HERE / 'native-mapstrings.json'), 'cases': len(strings['cases']),
             'scope': strings['scope']},
            {'file': 'native-triggers.json', 'sha256': sha(HERE / 'native-triggers.json'), 'cases': len(loads['cases']),
             'scope': loads['scope']},
            {'file': 'native-capture.json', 'sha256': sha(HERE / 'native-capture.json'),
             'cases': len(json.loads((HERE / 'native-capture.json').read_text())['cases']),
             'scope': '54BEA0 TriggerManager::QueueCaptured (combat capture binding: predicates::queue_captured)'},
            {'file': '../phase7-triggers/native-core-program.json', 'sha256': sha(ROOT / 'research/phase7-triggers/native-core-program.json'),
             'note': 'phase7 manager-local opcodes 0/1/16/80/81/91/92/93 (unchanged)'},
            {'file': '../phase7-triggers/native-world-program.json', 'sha256': sha(ROOT / 'research/phase7-triggers/native-world-program.json'),
             'note': 'phase7 world opcodes 43/44/54/61/62/63/64/114; now dispatched through the shared phase8 selectors'},
        ],
        'mission_data': {'chunks_index_sha256': sha(HERE / 'chunks/index.json'), 'census_sha256': sha(HERE / 'census.json'),
                         'coverage_sha256': sha(HERE / 'coverage.json')},
        'coverage_summary': coverage['summary'],
        'native_ported_functions': {
            '546AF0': 'dispatch order of every ported case (argument registers evaluated right-to-left as native)',
            '541900': 'validity (phase7)',
            '5422F0/54A5A0': 'register conversion incl. element kind (+1C) used for context writes',
            '54DAC0/54DC20': 'Kingdom/actor selectors with event/condition context writes (manager+24/+28/+10)',
            '54DB80/54DA80/54DCC0/54DF10/54DA20/54DA40/54DA60/54DE00/54DED0/54E8E0/54E910/54E8A0': 'other registers',
            '54C450/54C530/54C5F0/54C660/54C710/54C790/54BA90/54BB00/54BB70/54BBE0/54BC50/54BCC0/54CAC0': 'event scanners',
            '4F1AD0/4E7050/4E2120/4E2020/4E21F0/4E2250/4E22B0/542630/542690': 'objectives',
            '4E9720': 'Kingdom::SetMissionResult (victory/defeat)',
            '544540/544780': 'MoveCamera command text', '545C60': 'PlayVideo command text',
            '5452A0/543BA0': 'named and transported creation (incl. 4281E0/409230 vector math)',
            '542EB0/5435D0/543410': 'actor command structs (4D9DC0/4D9E20/4D9AB0/4D9BF0/4CA760)',
            '544A50/544BA0/544040': 'team flares and ability request',
            '96C7EC (VC7.1 _vswprintf %f/%d)': 'proven separately, domain |v| < 1e13',
            '54D3D0/541410/5468A0/54A640': 'TRIGGERS records, manager v4/v5, element v2/v3, 544190 rules explicit',
            '47EA40 path (5BD080 parser, 4BA710, 4B9D40/4B9F40) and 49AD70': 'MAPSTRINGS subset and localized binding',
            'round-2 leaves': '54C2F0 542580 541A60 542BD0 542340 543550 542360 5436D0 5430B0 541C20 543800 5431D0 '
                              '5426F0 5432A0 541D80 544CF0 544ED0 5420A0 5420D0 541EA0 541F20 4F96D0 546A30 544940 '
                              '542950 545080',
        },
        'gaps': [
            'All world services behind the seams (UI script commands, audio, camera angle tables, creation, placement, '
            'command processing, abilities/flares, Kingdom heroes/properties/resources, teleports, morale 579750 -> '
            'aa_combat::morale::modify, fog tile pass, game speed) need runtime bindings.',
            'Native null dereferences are explicit errors (missing TeamVictory var, transport+C0, flare+57C, missing '
            'difficulty variable, Kingdom+1C4, stale Area+84 GTID, null organization arrays).',
            'Registers (manager+2C) are reused natively between elements; the +4 string slot is modelled and used by the '
            'ma_hero register (random-hero item 28 of 35/48/49 ported); other non-owned storage reads are explicit errors.',
            'Opcode 68: results within 2^-40 of a binary32 rounding boundary are refused (FYL2X not ground truth).',
            'TRIGGERS manager versions 0-3, element versions 0/1 and 544190 upgrade rules are explicit errors (none in the maps).',
            'MAPSTRINGS: escapes, UTF-8 BOM with non-ASCII, >2047-byte lines, non-[Text] nodes and mixed language '
            'attributes are refused; non-ASCII text lookups and keys missing from MAPSTRINGS are explicit errors.',
            'Locale traced: the only setlocale calls (7F19D9/7F1A08/7F1AA1) keep LC_NUMERIC at "C"; LC_CTYPE never changes, '
            'so the MAPSTRINGS reader, _wcsicmp and printf run in the C locale on every code page.',
        ],
    }
    (HERE / 'findings.json').write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps(doc['coverage_summary']))


if __name__ == '__main__':
    main()
