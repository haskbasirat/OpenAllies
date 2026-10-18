"""Native oracle for the actor property removal wrapper 0x4CEF70.

0x4CEF70(actor; property, source): actor+0x78 Properties component; null ->
`ret 8` with no call; otherwise tail jump to 0x581810(property, source) with
ecx = component. 0x581810 is a seam here (its body is aa-body
property_effects::remove, proven by the phase7 property-effects fixtures): the
hook records (component, property, source) and returns a chosen al.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import harness as h  # noqa: E402

REMOVE = 0x4CEF70
SELECT_REMOVE = 0x581810


def run(component, property_, source, result):
    m = h.Machine('Double53')
    calls = []

    def removal(mm):
        calls.append({'component': mm.reg(h.UC_X86_REG_ECX), 'property': mm.arg(0), 'source': mm.arg(1)})
        mm.ret(8, result)
    m.hooks[SELECT_REMOVE] = removal
    actor = m.alloc(0x100)
    m.w(actor + 0x78, component)
    eax = m.call(REMOVE, ecx=actor, args=(property_, source), pop=8)
    return {'input': {'component': component, 'property': property_, 'source': source,
                      'seam_result': result},
            'calls': calls,
            'returned_al': (eax & 0xFF) if calls else None,
            'visited': m.visited}


def main():
    cases, visited = [], {}
    for component in (0, 0x2000400):
        for property_, source in ((0x11, 0), (0x22, 0x33), (0, 0x44)):
            for result in (0, 1):
                case = run(component, property_, source, result)
                visited.update(case.pop('visited'))
                if component == 0:
                    assert not case['calls']
                else:
                    assert case['calls'] == [{'component': component, 'property': property_,
                                              'source': source}]
                    assert case['returned_al'] == result
                cases.append(case)
    h.report('native-property-wrappers-fixtures.json',
             '0x4CEF70 actor property removal wrapper (0x581810 seam recorded).',
             cases, visited)


if __name__ == '__main__':
    main()
