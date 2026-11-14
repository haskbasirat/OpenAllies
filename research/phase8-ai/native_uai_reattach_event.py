"""Original KKC_UAI::EventReattach 52F160 (element Mind) under Unicorn on
the emulated element Mind (native_uai_harness.py), called directly, with
52A000 (current state), 52D6D0, 529C30 and the Mind activation 52A810
(52A420 for mode 2) all unchanged. Recorded: the state virtuals
+4/+18/+1C, Mind virtuals +1A0/+1A4 and the Updatable +8 scheduler; the
logger 404F90 is a seam (not recorded). Actor +8 flags, Mind+C, the stack
and the membership are case inputs.

Writes native-uai-reattach-event.json next to this script.
"""
from pathlib import Path
import json, random, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_uai_harness import ELEMENT_STATES, DISCARD, MindMachine, PRECISIONS, body_instructions
from native_machine import header, instructions, sha

ENTRY, END = 0x52f160, 0x52f1e3


class EventMachine(MindMachine):
    def __init__(self, c):
        super().__init__(c, ELEMENT_STATES, DISCARD)
        self.w(self.actor + 8, c['flags'])
        self.w(self.mind + 0xc, c['mind_word_c'])
        if c['membership'] is not None:
            ms = self.alloc(0x40); self.w(self.actor + 0x80, ms)
            if c['membership']:
                self.w(ms + 0x10, self.alloc(0x40))

    def run(self):
        r = self.call(ENTRY, ecx=self.mind)
        return dict(result=r & 0xff, events=self.events, **self.snap())


def cases():
    rnd = random.Random(0x52f160)
    rows = []
    pr = list(PRECISIONS)
    for i in range(300):
        st = {k: rnd.choice([0, 0, 1, 2]) for k in ELEMENT_STATES}
        rows.append({'name': 'reattach%d' % i, 'precision': pr[i % 3], 'status': st,
                     'stack': rnd.choice([[], ['reattach'], ['reattach', 'move'], ['move'], ['sleep', 'reattach'], ['deploy']]),
                     'flags': rnd.choice([0, 0, 1, 1 << 29, (1 << 29) | 1, 0x100, 1 << 28, 0xdead0000]),
                     'mind_word_c': rnd.choice([0, 0, 1]), 'membership': rnd.choice([None, False, True, True]),
                     'can_activate': rnd.random() < 0.7})
    return rows


def main():
    out, visited = [], {}
    for c in cases():
        m = EventMachine(c); result = m.run()
        m.verify_v14(); visited.update(m.visited)
        out.append({'input': c, 'output': result})
    d = header(__file__, __doc__)
    cover = {'52F160': [len([a for a in visited if ENTRY <= a < END]), body_instructions(ENTRY, END)]}
    d.update(instructions=instructions(visited), coverage=cover, cases=out)
    p = HERE / 'native-uai-reattach-event.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    print(json.dumps({'cases': len(out), 'instructions': len(visited), 'coverage': cover, 'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
