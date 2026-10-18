"""Original construction/upgrade completion statistics in isolated x86.

Executed unchanged: 564ED0 construction completion statistics, 564E10 upgrade
completion statistics, 58EAB0 scored-structure test, 58EAE0, 58EE00 structure
class, 45ECA0 list iterator, 58E870 score add, 58D8C0 by-definition relation
add (+7BA3C0/606750), and the 58EB60/58D560 constructors. Seams (recorded):
actor virtual +0x108 owner and +0x100, allocator.
"""
from pathlib import Path
import json, random, shutil, sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from harness import MODES, ROOT, bits, pin  # noqa: E402
from native_statistics import StatisticsMachine, STATS, RELATIONS  # noqa: E402

RANGES = {'construction_completed_564ED0': (0x564ED0, 0x564F80), 'upgrade_completed_564E10': (0x564E10, 0x564EA2),
          'scored_58EAB0': (0x58EAB0, 0x58EAD3), 'settlement_58EAE0': (0x58EAE0, 0x58EAF2),
          'structure_class_58EE00': (0x58EE00, 0x58EE78), 'list_begin_45ECA0': (0x45ECA0, 0x45ECB4),
          'score_add_58E870': (0x58E870, 0x58E883), 'definition_add_58D8C0': (0x58D8C0, 0x58D940)}
K, COMP, ACTOR, AVT, ADATA = 0x2010000, 0x2011000, 0x2012000, 0x2013000, 0x2014000
STUB_OWNER, STUB_V100 = 0x3000100, 0x3000110


class CompletionMachine(StatisticsMachine):
    def __init__(self, cw):
        super().__init__(cw)
        self.put(COMP + 4, ACTOR)
        self.put(ACTOR, AVT); self.put(ACTOR + 4, ADATA)
        self.put(AVT + 0x108, STUB_OWNER); self.put(AVT + 0x100, STUB_V100)
        self.put(K + 0x32C, STATS); self.put(K + 0x340, RELATIONS)
        self.trace = []
        self.seam(STUB_OWNER, self._owner)
        self.seam(STUB_V100, self._v100)

    def _owner(self, m, esp):
        self.trace.append({'kind': 'kingdom'})
        return (K if self.facts['owner'] else 0), 0

    def _v100(self, m, esp):
        self.trace.append({'kind': 'virtual_100'})
        return int(self.facts['virtual_100']), 0

    def run(self, op):
        f = self.facts = op['facts']
        self.trace = []
        self.put(K + 0x32C, STATS if f['statistics'] else 0)
        self.put(K + 0x340, RELATIONS if f['relations'] else 0)
        for off, key in ((0x88, 'component_88'), (0x90, 'component_90'), (0x94, 'structure'), (0x98, 'settlement'),
                         (0x9C, 'construction')):
            self.put(ACTOR + off, 0x2015000 + off if f[key] else 0)
        self.put(ADATA + 0x43C, 0x2016000 if f['data_list_43c'] else 0)
        self.put(ADATA + 0x4E0, f['site_class'])
        self.call(0x564ED0 if op['kind'] == 'construction' else 0x564E10, [0] if op['kind'] == 'upgrade' else [],
                  COMP)
        snap = self.snapshot()
        return {'scores': snap['scores'], 'relations': [{k: r[k] for k in ('values', 'maximum', 'leader')}
                                                        for r in snap['relations']], 'events': self.trace}


R = random.Random(0x564ED0)


def facts():
    return {'owner': R.random() < 0.9, 'statistics': R.random() < 0.9, 'relations': R.random() < 0.85,
            'structure': R.random() < 0.6, 'virtual_100': R.random() < 0.5, 'settlement': R.random() < 0.3,
            'construction': R.random() < 0.85, 'data_list_43c': R.random() < 0.3,
            'site_class': R.choice([0, 0, 7]), 'component_88': R.random() < 0.4, 'component_90': R.random() < 0.5}


def main():
    cases = [{'name': f'random-{i}', 'ops': [{'kind': R.choice(['construction', 'upgrade']), 'facts': facts()}
                                             for _ in range(R.randint(4, 12))]} for i in range(60)]
    fixtures, executions = [], 0
    for case in cases:
        for mode, cw in MODES:
            m = CompletionMachine(cw)
            steps = []
            for op in case['ops']:
                steps.append({'op': op, 'expected': m.run(op)})
                executions += 1
            fixtures.append({'name': case['name'], 'precision': mode, 'steps': steps})
    doc = {'schema': 1, 'definition': ADATA, 'pins': pin(RANGES), 'executions': executions, 'fixtures': fixtures}
    out = Path(__file__).resolve().parent / 'native-completion-statistics-fixtures.json'
    out.write_text(json.dumps(doc, separators=(',', ':')) + '\n', encoding='utf-8')
    shutil.copyfile(out, ROOT / 'crates/aa-economy/tests/fixtures/native-completion-statistics-fixtures.json')
    print(f'{len(fixtures)} fixtures, {executions} native executions')


if __name__ == '__main__':
    main()
