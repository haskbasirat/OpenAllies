"""Oracle: ORIGINAL TriggerManager::QueueCaptured 54BEA0 (54B820 Captured payload).

The combat capture path (4D5230) notifies the trigger manager through
54BEA0(captured, source): a 0x10-byte payload (vtable A39DA4, kind virtual
4F0500 returns 1) with +4 captured GTID, +8 source GTID and +C the GTID of the
source if its virtual +A8 is true, else of its virtual +108 owner, prepended
to manager+50. The original code runs under Unicorn (harness.Machine); the
virtuals are recorded seams. Writes native-capture.json.
"""
from pathlib import Path
import copy, hashlib, json, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402

PAYLOAD_VT = 0xa39da4


class Machine(h.Machine):
    def run_pushes(self):
        out = {}
        try:
            for captured, source in self.c['pushes']:
                cap = self.objects[captured] if captured else 0
                src = self.objects[source] if source else 0
                self.call(0x54bea0, h.MANAGER, [cap, src])
        except Exception as e:
            cause = e
            while cause is not None and 'native null-page access' not in str(cause):
                cause = cause.__cause__ or cause.__context__
            if cause is None:
                raise
            out['fault'] = str(cause)
        out['calls'] = self.calls
        events = []
        node = self.r(h.MANAGER + 0x50)
        while node:
            p = self.r(node)
            if p == 0:
                events.append(None)
            elif self.r(p) == PAYLOAD_VT:
                events.append({'kind': 1, 'words': [self.r(p + 4 * i) for i in (1, 2, 3)]})
            else:
                events.append({'fixture': True})
            node = self.r(node + 4)
        out['events'] = events
        return out


def base():
    return {
        'time_bits': h.core.bits(10), 'precision': 'Double53', 'variables': {},
        'triggers': [{'id': 10, 'enabled': 1, 'activation_bits': 0}],
        'objects': {
            'actor_a': {'kind': 'actor', 'slot': 3, 'gen': 1},
            'actor_b': {'kind': 'actor', 'slot': 4, 'gen': 2},
            'kingdom_a': {'kind': 'kingdom', 'slot': 2, 'gen': 1},
            'kingdom_b': {'kind': 'kingdom', 'slot': 5, 'gen': 3},
            'loose': {'kind': 'actor'},
        },
        'manager': {'context_word': 0, 'subjects': [], 'condition_kingdom': 0, 'event_kingdom': 0, 'events': []},
        'returns': {'IsKingdom': {'kingdom_a': 1, 'kingdom_b': 1}, 'Owner': {'actor_a': '@kingdom_a', 'actor_b': '@kingdom_b'}},
    }


CASES = []


def add(name, pushes, **changes):
    c = base()
    for k, v in changes.items():
        if isinstance(v, dict) and isinstance(c.get(k), dict):
            c[k] = {**c[k], **v}
        else:
            c[k] = v
    c['name'] = name
    c['pushes'] = pushes
    CASES.append(c)


for captured in ['actor_a', 'actor_b', 'loose', None]:
    for source in ['actor_a', 'actor_b', 'kingdom_a', 'kingdom_b', 'loose']:
        add(f'capture_{captured}_{source}', [[captured, source]])
add('capture_owner_null', [['actor_a', 'actor_b']], returns={'IsKingdom': {}, 'Owner': {}})
add('capture_owner_unregistered', [['actor_a', 'actor_b']], returns={'IsKingdom': {}, 'Owner': {'actor_b': '@loose'}})
add('capture_actor_is_kingdom', [['actor_b', 'actor_a']], returns={'IsKingdom': {'actor_a': 1}, 'Owner': {}})
add('capture_two', [['actor_a', 'actor_b'], ['actor_b', 'kingdom_a']])
add('capture_after_events', [['actor_b', 'actor_a']],
    manager={'context_word': 0, 'subjects': [], 'condition_kingdom': 0, 'event_kingdom': 0,
             'events': [{'kind': 0, 'words': ['gtid:actor_a', 'gtid:actor_b', 0]},
                        {'kind': 2, 'words': ['gtid:actor_a', '@kingdom_a']}]})
add('capture_null_source', [['actor_a', None]])
add('capture_null_source_after_push', [['actor_a', 'actor_b'], ['actor_b', None]])


def main():
    rows, visited = [], {}
    for c in CASES:
        m = Machine(c)
        rows.append({'input': c, 'output': m.run_pushes()})
        visited.update(m.visited)
    windows = []
    for a, n in sorted(visited.items()):
        raw = h.core.IMAGE.at(a, n)
        assert raw == h.core.ACCEPTED.at(a, n)
        windows.append({'address': hex(a), 'bytes': raw.hex()})
    doc = {'schema': 1, 'original_sha256': h.core.pe.ORIGINAL_SHA, 'v14_sha256': h.core.pe.ACCEPTED_SHA,
           'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
           'harness_sha256': hashlib.sha256((HERE / 'harness.py').read_bytes()).hexdigest(),
           'scope': __doc__.strip().splitlines()[0], 'cases': rows, 'visited_windows': windows}
    path = HERE / 'native-capture.json'
    path.write_text(json.dumps(doc, separators=(',', ':')) + '\n')
    print(json.dumps({'cases': len(rows), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
