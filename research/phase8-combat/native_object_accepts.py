"""Native oracle for 0x4D1EE0 (object virtual +174: does this object accept
the given actor, e.g. an aircraft landing or rearming).

Nothing is hooked. The object and the actor carry real AA.exe vtables, so
the virtuals run as original code: the actor's +108 (0x4D1E80: word +E8, or
0x4C9FA0: zero) and the object's +134 (0x4D1E90 for vtables 0xA273F0 and
0xA29490: own word +E8 nonzero and equal; 0x4E5240 for 0xA299D8: the object
itself equals the argument). The three object vtables are every vtable in
the image whose slot +174 is 0x4D1EE0. Words +E8 range over zero, two other
handles and both objects' own addresses.

Usage: native_object_accepts.py [--exe PATH_TO_AA_EXE]
(default: the AA.exe next to the rust-rebuild folder).
"""
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

ENTRY = 0x4D1EE0
OBJECT_VTABLES = (0xA273F0, 0xA29490, 0xA299D8)
# 0xA28A00 resolves +108 to 0x4C9FA0 (returns zero).
ACTOR_VTABLES = (0xA273F0, 0xA29490, 0xA299D8, 0xA28A00)
OTHER = (0x7000100, 0x7000200)


def main():
    args = sys.argv[1:]
    if args[:1] == ['--exe']:
        h.ORIGINAL = h.Image(Path(args[1]), h.ORIGINAL_SHA)
    m = h.Machine('Double53')
    obj = m.alloc(0x100)
    actor = m.alloc(0x100)
    words = lambda: (0, OTHER[0], OTHER[1], obj, actor)  # noqa: E731
    cases = []
    for ovt in OBJECT_VTABLES:
        for avt in ACTOR_VTABLES:
            for oe8 in words():
                for ae8 in words():
                    m.w(obj, ovt)
                    m.w(actor, avt)
                    m.w(obj + 0xE8, oe8)
                    m.w(actor + 0xE8, ae8)
                    result = m.call(ENTRY, ecx=obj, args=(actor,), pop=4)
                    cases.append({'object_vtable': ovt, 'actor_vtable': avt,
                                  'object': obj, 'actor': actor,
                                  'object_e8': oe8, 'actor_e8': ae8, 'result': result})
    slots = {hex(vt): [hex(struct.unpack('<I', h.ORIGINAL.at(vt + slot, 4))[0])
                       for slot in (0x108, 0x134, 0x174)]
             for vt in sorted(set(OBJECT_VTABLES + ACTOR_VTABLES))}
    h.report('native-object-accepts-fixtures.json',
             '0x4D1EE0 with real vtables (+108 0x4D1E80/0x4C9FA0, +134 0x4D1E90/0x4E5240); no seams.',
             cases, m.visited, {'vtable_slots_108_134_174': slots})


if __name__ == '__main__':
    main()
