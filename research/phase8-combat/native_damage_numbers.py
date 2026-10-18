"""Original damage-number presentation queue 50D0B0 (with 50D070 append).

aa-body names this Body-damage response Action::PresentationDamaged. Executes
unchanged 50D0B0, 50D070 and 4CF1B0: gates (session mode 6, the
ViewShowDamage byte at [B39940]+88, actor+0C definition +54 bit1), then a
0x4C-byte entry {0, actor GTID, seven-word damage record} appended to the
vector at this+10. Seam: 50CF30 vector reallocation (allocation detail; it
is emulated by copying the live entries into a larger buffer).
"""
import copy
from harness import Machine, PRECISIONS, f32, install_settings, install_world, report


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.c = copy.deepcopy(c)
        c = self.c
        install_settings(self, {})
        install_world(self, c['mode'])
        view = self.alloc(0x100)
        flag = self.alloc(0x10)
        self.wb(flag, int(c['show']))
        self.w(view + 0x88, flag)
        self.w(0xb39940, view)
        self.presenter = self.alloc(0x40)
        self.vector = self.presenter + 0x10
        existing = c['existing']
        cap = c['capacity']
        buf = self.alloc(0x4c * max(cap, 1)) if cap else 0
        self.w(self.vector, buf)
        self.w(self.vector + 4, existing)
        self.w(self.vector + 8, cap)
        for i in range(existing):
            for j in range(9):
                self.w(buf + 0x4c * i + 4 * j, 0x1000 * (i + 1) + j)
        self.actor = self.alloc(0x40)
        self.w(self.actor + 0x14, 0x30003)
        if c['definition'] is not None:
            d = self.alloc(0x80)
            self.w(self.actor + 0xc, d)
            self.w(d + 0x54, c['definition'])
        self.hooks[0x50cf30] = self.grow

    def grow(self, m):
        cap = self.arg()
        old = self.r(self.vector)
        count = self.r(self.vector + 4)
        new = self.alloc(0x4c * cap)
        if old:
            self.u.mem_write(new, bytes(self.u.mem_read(old, 0x4c * count)))
        self.w(self.vector, new)
        self.w(self.vector + 8, cap)
        self.events.append({'Grow': cap})
        self.ret(4)

    def run(self):
        record = self.alloc(0x20)
        for i, w in enumerate(self.c['record']):
            self.w(record + 4 * i, w)
        self.call(0x50d0b0, self.presenter, (self.actor, record))
        buf = self.r(self.vector)
        count = self.r(self.vector + 4)
        entries = [[self.r(buf + 0x4c * i + 4 * j) for j in range(9)] for i in range(count)]
        return {'entries': entries, 'events': self.events}


def cases():
    rows = []
    rec = [0x20002, 0x70007, 5, f32(12.5), f32(3), 1, 4]
    for mode, show, d in [(0, True, 2), (6, True, 2), (0, False, 2), (0, True, 0), (0, True, None), (0, True, 3)]:
        for existing, cap in [(0, 0), (1, 1), (2, 4)]:
            rows.append(dict(name=f'numbers_{mode}_{show}_{d}_{existing}_{cap}', mode=mode, show=show, definition=d,
                             existing=existing, capacity=cap, record=rec))
    return rows


def main():
    out = []
    visited = {}
    for c in cases():
        m = Run(c, PRECISIONS[1])
        try:
            result = m.run()
        except Exception as exc:
            raise RuntimeError(c['name']) from exc
        visited.update(m.visited)
        out.append({'input': {**c, 'precision': PRECISIONS[1]}, 'output': result})
    report('native-damage-number-fixtures.json', __doc__ + '\nInteger-only: one precision.', out, visited)


if __name__ == '__main__':
    main()
