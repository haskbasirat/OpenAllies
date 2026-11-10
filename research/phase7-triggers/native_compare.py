"""Original trigger comparison 541DE0(left, kind, right) and the activation
comparison 5425C0(kind, right) under Unicorn, executed unchanged.

541DE0 runs directly (stdcall-like ret 0xC) over a grid of binary32 operands
(zeros of both signs, subnormals, normals, the largest finite values,
infinities, quiet and signaling NaNs of both signs) and kinds 0..7. 5425C0
(thiscall element, ret 8) reads [B36534]+E0 (game time) and the element's
+C trigger record +4 (activation time); its pairs include differences that
are not binary32 values, that overflow binary32 and that are subnormal. No
seam: both functions are leaves apart from 5425C0's call of 541DE0.

Usage: native_compare.py (AA.exe is located by the shared harness relative to
this repository).
"""
import itertools
import struct
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'phase8-combat'))
from harness import Machine, PRECISIONS, f32, install_world, report  # noqa: E402
import harness  # noqa: E402

harness.HERE = Path(__file__).resolve().parent  # write the fixture here

SPECIAL = [0x00000000, 0x80000000, 0x00000001, 0x80000001, 0x007fffff, 0x00800000, f32(1.0), f32(-1.0),
           f32(0.1), f32(0.3), 0x3f800001, 0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000, 0x7fc00000,
           0xffc00000, 0x7f800001, 0xff800001, 0x7fbfffff]


def compare_cases():
    rows = []
    for left, right in itertools.product(SPECIAL, repeat=2):
        for kind in range(8):
            rows.append({'function': 'compare', 'left': left, 'kind': kind, 'right': right})
    return rows


def activation_cases():
    pairs = [(f32(10.0), f32(1.0)), (f32(16777216.0), f32(0.1)), (f32(0.1), f32(16777216.0)),
             (f32(1.0), 0x33800001), (0x7f7fffff, 0xff7fffff), (0xff7fffff, 0x7f7fffff),
             (0x00800000, 0x00800001), (f32(3.0), f32(3.0)), (0x7f7fffff, 0xf3000000),
             (f32(1e-30), f32(-1e-30)), (f32(1234.5678), f32(0.0001))]
    rights = [f32(0.0), f32(9.0), f32(16777216.0), 0x7f800000, 0x7fc00000, f32(-0.1)]
    rows = []
    for (time, activation), right in itertools.product(pairs, rights):
        for kind in range(7):
            rows.append({'function': 'activation', 'time': time, 'activation': activation, 'kind': kind,
                         'right': right})
    return rows


def run(c, precision):
    m = Machine(precision)
    m.u.mem_map(0, 0x1000)
    if c['function'] == 'compare':
        result = m.call(0x541de0, 0, (c['left'], c['kind'], c['right']))
    else:
        world = install_world(m, time=c['time'])
        record = m.alloc(0x40)
        m.w(record + 4, c['activation'])
        element = m.alloc(0x40)
        m.w(element + 0xc, record)
        result = m.call(0x5425c0, element, (c['kind'], c['right']))
        assert m.r(world + 0xe0) == c['time']
    return m, {'result': result & 0xff}


def main():
    out, visited = [], {}
    jobs = [(c, PRECISIONS[:1]) for c in compare_cases()] + [(c, PRECISIONS) for c in activation_cases()]
    for c, precisions in jobs:
        for precision in precisions:
            m, result = run(c, precision)
            visited.update(m.visited)
            out.append({'input': {**c, 'precision': precision}, 'output': result})
    report('native-compare-fixtures.json', __doc__ + '\n541DE0 has no arithmetic (one precision); 5425C0 '
           'cases run at every precision control.', out, visited)


if __name__ == '__main__':
    main()
