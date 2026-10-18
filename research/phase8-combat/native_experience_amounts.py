"""Original experience amount routines under Unicorn.

Executes unchanged 571820 (value ratio), 571960 (kill), 5719E0 (capture),
5718E0 (rout) and 5718B0 (engagement). The only seam is 4D01E0 actor value,
whose extended return is supplied as an exact binary64 value. Victim
definition+4F4 is real fixture memory. Results are read from ST0 as the full
80-bit value (mantissa and sign/exponent), not rounded by the harness.
"""
import copy
from harness import Machine, PRECISIONS, f32, f64, install_settings, report
from unicorn.x86_const import UC_X86_REG_ECX

OFFSETS = {'minimum': 0x160, 'maximum': 0x164, 'kill': 0x168, 'capture': 0x16c, 'rout': 0x170,
           'engagement': 0x174, 'default_value': 0x178}
ENTRY = {'ratio': 0x571820, 'kill': 0x571960, 'capture': 0x5719e0, 'rout': 0x5718e0,
         'engagement': 0x5718b0}


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.c = copy.deepcopy(c)
        install_settings(self, {OFFSETS[k]: v for k, v in c['settings'].items()})
        self.actors = {}
        for tag in (1, 2):
            actor = self.alloc(0x40)
            definition = self.alloc(0x600)
            self.w(actor + 4, definition)
            self.w(definition + 0x4f4, c['definition4f4'] if tag == 2 else 0)
            self.actors[tag] = actor
        self.tags = {v: k for k, v in self.actors.items()}
        self.values = {k: list(v) for k, v in c['values'].items()}
        self.hooks[0x4d01e0] = self.value

    def value(self, m):
        assert self.arg() == 0
        tag = self.tags[self.reg(UC_X86_REG_ECX)]
        bits = self.values[str(tag)].pop(0)
        self.events.append({'ActorValue': tag})
        self.ret_ext(4, bits)

    def run(self):
        victim = self.actors[2] if self.c['victim_present'] else 0
        self.call(ENTRY[self.c['operation']], 0, (self.actors[1], victim), pop=0)
        result = self.st0_ext()
        assert all(not v for v in self.values.values()), self.values
        return {'result': result, 'events': self.events}


def cases():
    rows = []
    settings = dict(minimum=f32(.25), maximum=f32(6), kill=f32(1.5), capture=f32(.75), rout=f32(2.5),
                    engagement=f32(1.1), default_value=f32(1))
    pairs = [(4.0, 2.0), (2.0, 4.0), (3.0, 1.0), (1.0, 3.0), (7.0, 1.0 / 3.0), (1e-3, 0.1), (0.1, 1e-3),
             (5.0, 0.0), (0.0, 5.0), (-2.0, 2.0), (2.0, -6.0), (3.0, 1e20), (1.0, 2.0 ** -20),
             (16777217.0, 1.0), (1.0, 16777217.0), (0.30000000000000004, 0.1)]
    for op in ['ratio', 'kill', 'capture', 'rout', 'engagement']:
        for source, victim in pairs:
            for d4f4 in ([0, 0xbf800000] if victim == 0.0 else [0]):
                values = {'2': [f64(victim)], '1': [f64(source)]}
                rows.append(dict(name=f'{op}_{source}_{victim}_{d4f4:x}', operation=op, settings=settings,
                                 values=values, victim_present=True, definition4f4=d4f4))
        rows.append(dict(name=f'{op}_no_victim', operation=op, settings=settings, values={'1': [], '2': []},
                         victim_present=False, definition4f4=0))
        for name, s in [('narrow', dict(settings, minimum=f32(1), maximum=f32(1))),
                        ('inverted', dict(settings, minimum=f32(3), maximum=f32(2))),
                        ('negative', dict(settings, minimum=f32(-4), maximum=f32(-1), default_value=f32(-3)))]:
            for source, victim in [(4.0, 2.0), (1.0, 9.0), (2.0, 0.0)]:
                rows.append(dict(name=f'{op}_{name}_{source}_{victim}', operation=op, settings=s,
                                 values={'2': [f64(victim)], '1': [f64(source)]}, victim_present=True,
                                 definition4f4=0))
    return rows


def main():
    out = []
    visited = {}
    for precision in PRECISIONS:
        for c in cases():
            m = Run(c, precision)
            try:
                result = m.run()
            except Exception as exc:
                raise RuntimeError(f'{c["name"]} {precision}') from exc
            visited.update(m.visited)
            out.append({'input': {**c, 'precision': precision}, 'output': result})
    report('native-experience-amount-fixtures.json', __doc__, out, visited)


if __name__ == '__main__':
    main()
