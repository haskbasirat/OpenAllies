"""Original ROE primary damage history under Unicorn.

Executes unchanged 570F70 (record damage on the organization's primary
element), 570A70 (prune records older than ROEPrimaryDamageHistoryLength),
570E30 (damage rate: pruned in-order sum / history length) and the vector
push 570EA0 with its reallocation 5707E0. Only the allocator (6F7810) and
free (6F7430) are seams. Vector capacity/addresses are allocation details and
are not compared; size and every live 7-word record are.
"""
import copy
from harness import Machine, PRECISIONS, f32, install_settings, install_world, report


class Run(Machine):
    def __init__(self, c, precision):
        super().__init__(precision)
        self.c = copy.deepcopy(c)
        install_settings(self, {0x194: c['history_length']})
        install_world(self, 0, c['now'])
        self.component = self.alloc(0x40)
        self.owner = self.alloc(0x40)
        self.other = self.alloc(0x40)
        self.org_actor = self.alloc(0x100)
        self.org = self.alloc(0x100)
        self.w(self.component + 4, self.owner)
        self.w(self.component + 0x10, self.org_actor if c['organization_actor'] else 0)
        self.w(self.org_actor + 0x7c, self.org)
        primary = {'owner': self.owner, 'other': self.other, 'none': 0}[c['primary']]
        self.w(self.org + 0x44, primary)
        capacity = c['capacity']
        records = c['records']
        assert capacity >= len(records)
        data = self.alloc(max(capacity, 1) * 0x1c) if capacity else 0
        self.w(self.component + 0x28, data)
        self.w(self.component + 0x2c, len(records))
        self.w(self.component + 0x30, capacity)
        for i, record in enumerate(records):
            for j, word in enumerate(record):
                self.w(data + i * 0x1c + j * 4, word)
        self.hooks[0x6f7810] = self.allocate
        self.hooks[0x6f7430] = self.free

    def allocate(self, m):
        size = self.arg()
        self.events.append({'Allocate': size // 0x1c})
        self.ret(0, self.alloc(max(size, 16)))

    def free(self, m):
        self.events.append('Free')
        self.ret(0)

    def records(self):
        data = self.r(self.component + 0x28)
        size = self.r(self.component + 0x2c)
        return [[self.r(data + i * 0x1c + j * 4) for j in range(7)] for i in range(size)]

    def run(self):
        op = self.c['operation']
        result = None
        if op == 'record':
            record = self.alloc(0x1c)
            for j, word in enumerate(self.c['record']):
                self.w(record + j * 4, word)
            self.call(0x570f70, self.component, (record,))
        elif op == 'prune':
            self.call(0x570a70, self.component, ())
        elif op == 'rate':
            self.call(0x570e30, self.component, ())
            result = self.st0_ext()
        else:
            raise ValueError(op)
        return {'result': result, 'records': self.records(), 'events': self.events}


def rec(source, amount, time, kind=1, reason=4):
    return [source, 0x20007, 9, f32(amount), f32(time), kind, reason]


def cases():
    rows = []
    histories = {
        'empty': [],
        'one_old': [rec(1, 5, 1)],
        'one_new': [rec(1, 5, 9)],
        'edge': [rec(1, 5, 7), rec(2, 6, 6.9999995), rec(3, 7, 7.0000005)],
        'mixed': [rec(1, 5, 1), rec(2, 6, 8), rec(3, 7, 2), rec(4, 8, 3), rec(5, 9, 9.5), rec(6, 1, 0)],
        'all_old': [rec(i, i * 1.5, i * 0.1) for i in range(1, 8)],
        'many': [rec(i, 0.1 * i, 5 + i * 0.37) for i in range(1, 12)],
        'fractions': [rec(i, 1.0 / 3.0 * i, 9) for i in range(1, 10)],
        'large': [rec(1, 16777216, 9), rec(2, 1, 9), rec(3, 1, 9), rec(4, -2, 9), rec(5, 0.5, 9)],
        'negative_time': [rec(1, 5, -3), rec(2, 6, -2)],
    }
    for name, records in histories.items():
        for now, length in [(10, 3), (10, 0.5), (7, 10), (10.000001, 3.0000002), (-1, 1)]:
            for op in ['prune', 'rate']:
                rows.append(dict(name=f'{op}_{name}_{now}_{length}', operation=op, records=records,
                                 capacity=len(records) + 1, now=f32(now), history_length=f32(length),
                                 organization_actor=True, primary='owner'))
            for organization_actor, primary, capacity in [(True, 'owner', len(records)),
                                                          (True, 'owner', len(records) + 2),
                                                          (True, 'other', len(records) + 1),
                                                          (True, 'none', len(records) + 1),
                                                          (False, 'owner', len(records) + 1)]:
                rows.append(dict(name=f'record_{name}_{now}_{length}_{organization_actor}_{primary}_{capacity}',
                                 operation='record', records=records, capacity=capacity, now=f32(now),
                                 history_length=f32(length), organization_actor=organization_actor,
                                 primary=primary, record=rec(0x30009, 12.5, now, 3, 5)))
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
    report('native-roe-fixtures.json', __doc__, out, visited)


if __name__ == '__main__':
    main()
