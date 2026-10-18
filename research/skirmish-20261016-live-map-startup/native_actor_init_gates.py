"""Actual Actor NULL Init leaf gates, not a complete initialized Actor/world.

4D1FD0 executes its null source+4E0 return; nonnull stops BEFORE the first
required-object read+158. 4D67F0 and4D4130 execute their bit29 early returns;
otherwise stop BEFORE required4D10A0. No downstream service is substituted.
Both original/v14 images execute independently, all three x87 PCs.
"""
from pathlib import Path
import argparse, hashlib, json, sys
sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-conversion'))
import native_machine as nm
from unicorn import UC_HOOK_CODE

class Required(Exception):
    pass

def run(image, pc, entry, flags, reference):
    m = nm.Machine(pc)
    old = (nm.P.image_size + 4095) & ~4095
    new = (image.image_size + 4095) & ~4095
    if new > old:
        m.u.mem_map(image.base + old, new - old)
    for _, rva, size, offset in image.sections:
        m.u.mem_write(image.base + rva, image.b[offset:offset + size])
    actor, definition = m.alloc(0x200, 0xA5), m.alloc(0x600, 0xA5)
    m.w(actor + 4, definition)
    m.w(actor + 8, flags)
    m.w(definition + 0x4E0, reference)
    before = bytes(m.u.mem_read(actor, 0x200))
    pins, reached = {}, []
    def observe(_, address, size, __):
        if image.base <= address < image.base + image.image_size:
            raw = image.range(address, size)
            assert raw == nm.P.range(address, size) == nm.V.range(address, size)
            pins[f'{address:08x}'] = raw.hex()
        if address == 0x4D1FE6 and entry == 0x4D1FD0:
            reached.append('nonnull_required_object_4E0')
            raise Required()
        if address == 0x4D10A0:
            reached.append('required_4D10A0')
            raise Required()
    m.u.hook_add(UC_HOOK_CODE, observe)
    outcome = 'returned'
    result = None
    try:
        result = m.call(entry, actor, [], 0)
    except Required:
        outcome = 'required_service'
    assert not m.null_accesses and m.outcome is None
    assert bytes(m.u.mem_read(actor, 0x200)) == before
    expected = reference != 0 if entry == 0x4D1FD0 else flags & (1 << 29) == 0
    assert (outcome == 'required_service') == expected
    if entry == 0x4D1FD0 and not expected:
        assert result & 255 == 1
    return {'outcome': outcome, 'required': reached,
            'returns_true': entry == 0x4D1FD0 and not expected,
            'actor_bytes_unchanged': True}, pins

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--write', action='store_true')
    args = ap.parse_args()
    flags = sorted({0, 1, 1 << 10, 1 << 24, 1 << 29, (1 << 29) | 1,
                    (1 << 29) | (1 << 10), 0xA0C46900, 0xFFFFFFFF})
    rows, pins = [], {}
    for pc in ('Single24', 'Double53', 'Extended64'):
        for entry in (0x4D1FD0, 0x4D67F0, 0x4D4130):
            for flag in flags:
                for ref in (0, 0x12345678):
                    a, pa = run(nm.P, pc, entry, flag, ref)
                    b, pb = run(nm.V, pc, entry, flag, ref)
                    assert a == b and pa == pb
                    pins.update(pa)
                    rows.append({'precision': pc, 'entry': f'{entry:08x}',
                                 'flags': flag, 'source_required_reference': ref,
                                 'output': a})
    owners = []
    for native_class, vtable in ((3, 0xA273F0), (4, 0xA299D8)):
        slot = vtable + 0x108
        raw = nm.P.range(slot, 4)
        assert raw == nm.V.range(slot, 4) == (0x4D1E80).to_bytes(4, 'little')
        leaf = nm.P.range(0x4D1E80, 7)
        assert leaf == nm.V.range(0x4D1E80, 7) == bytes.fromhex('8b81e8000000c3')
        owners.append({'native_class': native_class, 'slot': f'{slot:08x}',
                       'slot_bytes': raw.hex(), 'leaf': '004d1e80', 'leaf_bytes': leaf.hex()})
    output = {'schema': 1, 'scope': __doc__, 'paired_case_count': len(rows),
              'native_execution_count': 2 * len(rows), 'cases': rows,
              'owner_virtual108': owners, 'instruction_pins': pins,
              'source_images': [{'name': label, 'sha256': hashlib.sha256(image.b).hexdigest()}
                                for label, image in (('original', nm.P), ('v14', nm.V))]}
    target = HERE / 'native-actor-init-gates.json'
    if args.write:
        assert not target.exists(), 'Frozen fixture already exists'
        target.write_text(json.dumps(output, indent=2) + '\n', encoding='utf-8')
    else:
        assert json.loads(target.read_text(encoding='utf-8')) == output
    print(json.dumps({'paired_cases': len(rows), 'native_executions': len(rows)*2,
                      'instruction_pins': len(pins), 'sha256': hashlib.sha256(target.read_bytes()).hexdigest()}))

if __name__ == '__main__':
    main()
