import sys, json
from native_actor_creation import cases, run, build
from actor_machine import FUNCS
label = sys.argv[1]
for l, spec in cases():
    if l == label:
        m = build(spec)
        m.calls, m.outcome = [], None
        m.null_accesses.clear()
        m.guard = True
        entry = {'create': 0x4f9c10, 'actor': 0x4fa030, 'organization': 0x4fa150}[spec.get('entry', 'create')]
        r = m.call(entry, m.gworld, [m.params], 4)
        print('outcome', m.outcome, 'null', m.null_accesses)
        print('trace', [hex(a) for a in m.trace[-40:]])
        for c in m.calls[-30:]:
            print(' ', json.dumps(c)[:300])
