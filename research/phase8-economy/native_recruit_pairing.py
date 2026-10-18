"""Original recruit-single pairing 0x4E1200 in isolated x86 (executed
unchanged; no seams): item+0xC = member+0x14, then Element(member+0x80)+0x14 =
item when the member has an Element."""
from pathlib import Path
import json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import Machine, MODES, ROOT, pin  # noqa: E402

RANGES = {'pair_single_4E1200': (0x4E1200, 0x4E121A)}
ITEM, MEMBER, ELEMENT = 0x2000000, 0x2001000, 0x2002000


def run(cw, case):
    m = Machine(cw)
    m.put(ITEM + 0xC, case['item_actor']); m.put(ITEM + 0x10, case['item_organization'])
    m.put(MEMBER + 0x14, case['member_gtid'])
    m.put(MEMBER + 0x80, ELEMENT if case['element'] else 0)
    m.put(ELEMENT + 0x14, case['element_item'])
    m.call(0x4E1200, [MEMBER], ITEM)
    return {'item_actor': m.read(ITEM + 0xC), 'item_organization': m.read(ITEM + 0x10),
            'element_item_is_item': m.read(ELEMENT + 0x14) == ITEM if case['element'] else None,
            'element_item_word': m.read(ELEMENT + 0x14)}


def main():
    r = random.Random(0x4E1200)
    cases = [{'name': f'random-{i}', 'item_actor': r.getrandbits(32), 'item_organization': r.getrandbits(32),
              'member_gtid': r.choice([0, (1 << 16) | r.randint(1, 900), r.getrandbits(32)]),
              'element': r.random() < 0.6, 'element_item': r.getrandbits(32)} for i in range(40)]
    fixtures = []
    for case in cases:
        for mode, cw in MODES:
            fixtures.append({'name': case['name'], 'precision': mode, 'case': case, 'expected': run(cw, case)})
    doc = {'schema': 1, 'pins': pin(RANGES), 'executions': len(fixtures), 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-recruit-pairing-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-recruit-pairing-fixtures.json')
    print(f'{len(fixtures)} fixtures')


if __name__ == '__main__':
    main()
