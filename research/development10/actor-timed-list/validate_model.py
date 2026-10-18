"""Independent integer/rational model check of the frozen native fixtures.

This is research validation, not production Rust acceptance. Finite addition
rounds at PC24/53/64 without a binary32 intermediate store. Nonfinite cases
implement the observed FCOMP/test-AH41 parity branch explicitly.
"""
from pathlib import Path
from fractions import Fraction
import json

HERE = Path(__file__).resolve().parent
SELF = 0x10001
PRECISION = {'Single24': 24, 'Double53': 53, 'Extended64': 64}


def finite(bits):
    exponent = (bits >> 23) & 255
    assert exponent != 255
    mantissa = bits & 0x7fffff
    if exponent:
        mantissa |= 1 << 23
    return (-mantissa if bits >> 31 else mantissa), (-149 if not exponent else exponent - 150)


def exact(pair):
    value, exponent = pair
    return Fraction(value << exponent) if exponent >= 0 else Fraction(value, 1 << -exponent)


def round_sum(a, b, precision):
    a, ae = finite(a)
    b, be = finite(b)
    exponent = min(ae, be)
    value = (a << (ae - exponent)) + (b << (be - exponent))
    sign = -1 if value < 0 else 1
    value = abs(value)
    shift = max(0, value.bit_length() - precision)
    if shift:
        q, r = divmod(value, 1 << shift)
        half = 1 << (shift - 1)
        value = q + (r > half or (r == half and q & 1))
        exponent += shift
    return exact((value * sign, exponent))


def keep(start, duration, now, precision):
    def category(value):
        magnitude = value & 0x7fffffff
        return 'nan' if magnitude > 0x7f800000 else ('inf' if magnitude == 0x7f800000 else 'finite')
    sk, dk, nk = map(category, [start, duration, now])
    if 'nan' in [sk, dk, nk]:
        return True
    if sk == dk == 'inf' and (start ^ duration) >> 31:
        return True
    if 'inf' in [sk, dk]:
        positive = (start if sk == 'inf' else duration) >> 31 == 0
        if nk == 'inf':
            return positive and now >> 31 != 0
        return positive
    if nk == 'inf':
        return now >> 31 != 0
    return round_sum(start, duration, precision) > exact(finite(now))


def model(data, precision):
    reciprocal = {k: [{'index': i, 'value': [v]} for i, v in enumerate(values)] for k, values in data['reciprocal'].items()}
    timed = [{'index': i, 'value': values.copy()} for i, values in enumerate(data['timed'])]
    trace, freed = [], []

    def resolve(reference):
        trace.append(['resolve', reference])
        generation, present = data['registry'].get(str(reference & 0xffff), [0, False])
        return generation == reference >> 16 and present

    for entry in list(timed):
        reference, start, duration = entry['value']
        trace.append(['flags', data['flags']])
        if not data['flags'] & 1 and resolve(reference):
            trace.append(['time', data['time']])
            if keep(start, duration, data['time'], precision):
                continue
        if resolve(reference):
            assert resolve(reference)
            if data.get('cutoff') == '0x4f1430':
                trace.append(['required', '0x4f1430'])
                break
            index = reference & 0xffff
            trace.append(['reciprocal', index, SELF])
            rows = reciprocal[str(index)]
            for i, row in enumerate(rows):
                if row['value'][0] == SELF:
                    del rows[i]
                    free = ['reciprocal', index, row['index']]
                    freed.append(free)
                    trace.append(['free', free])
                    break
        if data.get('cutoff') == '0x5ac280':
            trace.append(['required', '0x5ac280'])
            break
        trace.append(['remove', entry['index']])
        timed.remove(entry)
        free = ['timed', 1, entry['index']]
        freed.append(free)
        trace.append(['free', free])
    return {'timed': timed, 'reciprocal': reciprocal, 'freed': freed, 'trace': trace}


def main():
    doc = json.loads((HERE / 'native-timed-list.json').read_text())
    for i, case in enumerate(doc['cases']):
        actual = model(case['input'], PRECISION[case['precision']])
        assert actual == case['output'], (i, case, actual)
    load = json.loads((HERE / 'native-load-timed-lists.json').read_text())
    for case in load['cases']:
        for name in ['reciprocal', 'timed']:
            assert case['output'][name] == case['input']['existing_' + name] + case['input'][name]
    print(json.dumps({'update_model_cases': len(doc['cases']), 'load_model_cases': len(load['cases']),
                      'production_rust_execution': False}))


if __name__ == '__main__':
    main()
