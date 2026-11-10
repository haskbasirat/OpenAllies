"""Native oracle for the group order handout of KKC_GGroup:

* 0x4DE7F0  give member `index` the group's command, else (when the command
            type wants a position or a target) a type-5 Move to the command's
            position (command +C bit 2) or a type-5 Follow of its target
* 0x4DEA20  one type-5 Move, re-aimed at each member's place before delivery
* 0x4DEE40  the group rests: state, position, the reform of 0x4DEA20, the
            member formation flags, the slot-7 zone and the 1e-4 reschedule

Original code runs for these three and for the command constructors 0x4D9E20,
0x4CA760, 0x4DDE60, 0x4D9BF0, the handle lookups 0x4DE1F0 / 0x530F10, the
type mask tests 0x496F10 / 0x496EF0 and the reference setter 0x4B9C20.
Recorded seams (each records its arguments and the command words it sees and
answers from the case): 0x6F7810 (allocation), member virtual +150 (accepts),
0x4DDB60 (formation flag), 0x4D9B60 (delivery), 0x4DE020 (the member's place),
0x4EEB40 (set position), 0x514CE0 (slot-7 zone removal), virtual +4 of the
group's +180 scheduler and 0x51FFC0 (command vtable A3CC38 slot 0, release).
Runs at PC53 (the simulation thread).

Usage: native_group_orders.py [--exe PATH_TO_AA_EXE]
(default: the AA.exe next to the rust-rebuild folder).
"""
from pathlib import Path
import hashlib
import json
import random
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-combat'))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ECX  # noqa: E402

OUT = 'native-group-orders.json'
RULES_ROOT = 0xB36530
HANDLES = 0xB364C0
KINGDOM = 0x6600
COMMAND_WORDS = (0x8, 0xC, 0x10, 0x14, 0x18, 0x1C, 0x20, 0x24, 0x28)


class Lab:
    def __init__(self):
        m = self.m = h.Machine('Double53')
        m.u.mem_map(0, 0x1000)
        self.rules = m.alloc(0x400)
        self.kinds = m.alloc(0x40)
        m.w(self.rules + 0x298, self.kinds)
        m.w(RULES_ROOT, self.rules)
        self.table = m.alloc(0x30000)
        m.w(HANDLES, self.table)
        self.slot = 0x10
        self.events = []
        self.answers = {}
        self.members = {}
        self.member_vtable = m.vtable({0x150: self.accepts})
        self.scheduler_vtable = m.vtable({4: self.schedule})
        m.hooks[0x6F7810] = self.new
        m.hooks[0x4DDB60] = self.flag
        m.hooks[0x4D9B60] = self.deliver
        m.hooks[0x4DE020] = self.place
        m.hooks[0x4EEB40] = self.set_position
        m.hooks[0x514CE0] = self.remove_zone
        m.hooks[0x51FFC0] = self.release

    # objects ---------------------------------------------------------------
    def handle(self, target, live=True):
        m = self.m
        slot = self.slot
        self.slot += 1
        generation = 0x21 + slot
        m.u.mem_write(self.table + 4 + slot * 2, struct.pack('<H', generation if live else generation + 1))
        m.w(self.table + 0x20004 + slot * 4, target)
        return (generation << 16) | slot

    def words(self, command):
        return [self.m.r(command + offset) for offset in COMMAND_WORDS]

    def answer(self, key):
        return self.answers[key].pop(0)

    # seams -----------------------------------------------------------------
    def new(self, m):
        size = m.arg(0)
        block = m.alloc(size)
        m.u.mem_write(block, b'\xA5' * size)
        self.events.append({'new': size})
        m.ret(0, block)

    def accepts(self, m):
        member = self.members[m.reg(UC_X86_REG_ECX)]
        kingdom, ref, third = m.arg(0), m.arg(1), m.arg(2)
        self.events.append({'accepts': member, 'kingdom': kingdom, 'third': third,
                            'command': self.words(m.r(ref))})
        m.ret(12, self.answer(('accepts', member)))

    def flag(self, m):
        member = self.members[m.arg(0)]
        self.events.append({'flag': member, 'command': self.words(m.r(m.arg(1)))})
        m.ret(8, self.answer(('flag', member)))

    def deliver(self, m):
        member = self.members[m.arg(0)]
        self.events.append({'deliver': member, 'command': self.words(m.reg(UC_X86_REG_ECX))})
        m.ret(4, self.answer(('deliver', member)))

    def place(self, m):
        out, entry, point = m.arg(0), m.arg(1), m.arg(2)
        index = (entry - self.entries) // 16
        self.events.append({'place': index, 'point': [m.r(point), m.r(point + 4)]})
        offset = self.answer(('place', index))
        m.w(out, offset[0])
        m.w(out + 4, offset[1])
        m.ret(12, out)

    def set_position(self, m):
        self.events.append({'set_position': [m.arg(0), m.arg(1)]})
        m.ret(8)

    def remove_zone(self, m):
        self.events.append({'remove_zone': m.reg(UC_X86_REG_ECX) == self.zone7})
        m.ret(0)

    def schedule(self, m):
        self.events.append({'schedule': [m.arg(0), m.arg(1)],
                            'scheduler': m.reg(UC_X86_REG_ECX) == self.group + 0x180})
        m.ret(8)

    def release(self, m):
        self.events.append({'release': self.words(m.reg(UC_X86_REG_ECX)), 'arg': m.arg(0)})
        m.ret(4)

    # a group ---------------------------------------------------------------
    def build(self, rng, count, dead):
        m = self.m
        self.group = m.alloc(0x200)
        g = self.group
        m.w(g + 0xE8, KINGDOM)
        self.entries = m.alloc(16 * max(count, 1))
        m.w(g + 0x188, self.entries)
        m.w(g + 0x18C, count)
        m.w(g + 0x180, self.scheduler_vtable)
        self.members = {}
        live = []
        for index in range(count):
            actor = m.alloc(0x40)
            m.w(actor, self.member_vtable)
            self.members[actor] = index
            kind = dead[index]
            if kind == 'stale':
                handle = self.handle(actor, live=False)
            elif kind == 'null':
                handle = self.handle(0)
            else:
                handle = self.handle(actor)
                live.append(index)
            m.w(self.entries + 16 * index, handle)
            m.wb(self.entries + 16 * index + 0xC, rng.choice((0, 1, 2)))
        self.events = []
        return live

    def entry_flags(self, count):
        return [self.m.rb(self.entries + 16 * i + 0xC) for i in range(count)]


def rnd_word(rng):
    return h.f32(rng.uniform(-900.0, 900.0))


def member_kinds(rng, count):
    return [rng.choice(('live', 'live', 'live', 'stale', 'null')) for _ in range(count)]


def answers_for(rng, count, places=False):
    out = {}
    for i in range(count):
        out[('accepts', i)] = [rng.choice((0, 1)) for _ in range(2)]
        out[('flag', i)] = [rng.choice((0, 1, 2)) for _ in range(2)]
        out[('deliver', i)] = [rng.choice((0, 1)) for _ in range(2)]
        if places:
            out[('place', i)] = [[rnd_word(rng), rnd_word(rng)]]
    return out


def give_cases(lab, rng):
    m = lab.m
    cases = []
    for number in range(400):
        count = rng.randint(1, 4)
        kinds = member_kinds(rng, count)
        lab.build(rng, count, kinds)
        flags_before = lab.entry_flags(count)
        index = rng.randrange(count)
        descriptor5 = rng.choice((0, m.alloc(0x40), m.alloc(0x40)))
        m.w(lab.kinds + 5 * 4, descriptor5)
        descriptor = m.alloc(0x40)
        mask = rng.choice((0, 0x100, 0x10, 0x110, 0xF, 0x300, 0x20, 0x80000000, 0x10000))
        m.w(descriptor + 0x2C, mask)
        m.w(descriptor + 0xC, rng.choice((5, 3)))
        command = m.alloc(0x40)
        m.u.mem_write(command, bytes(0x40))
        m.w(command, 0xA3CC38)
        m.w(command + 4, 1)
        m.w(command + 8, descriptor)
        flags = rng.getrandbits(8)
        m.w(command + 0xC, flags)
        target_kind = rng.choice(('live', 'stale', 'zero'))
        target = m.alloc(0x40)
        target_self = rng.getrandbits(32)
        m.w(target + 0x14, target_self)
        target_handle = {'live': lambda: lab.handle(target), 'stale': lambda: lab.handle(target, False),
                         'zero': lambda: 0}[target_kind]()
        m.w(command + 0x10, target_handle)
        position = [rnd_word(rng), rnd_word(rng)]
        m.w(command + 0x18, position[0])
        m.w(command + 0x1C, position[1])
        ref = m.alloc(4)
        m.w(ref, command)
        lab.answers = answers_for(rng, count)
        answers = {f'{k[0]}:{k[1]}': list(v) for k, v in lab.answers.items()}
        result = m.call(0x4DE7F0, ecx=lab.group, args=(index, ref), pop=8) & 0xFF
        cases.append({'members': kinds, 'index': index, 'descriptor5': descriptor5,
                      'descriptor': descriptor, 'mask': mask, 'command': lab.words(command),
                      'target_kind': target_kind, 'target_object_handle': target_self,
                      'flags_before': flags_before, 'answers': answers, 'events': lab.events,
                      'entry_flags': lab.entry_flags(count), 'result': result})
    return cases


def place_cases(lab, rng):
    m = lab.m
    cases = []
    for number in range(200):
        count = rng.randint(0, 5)
        kinds = member_kinds(rng, count)
        lab.build(rng, count, kinds)
        descriptor5 = rng.choice((0, m.alloc(0x40)))
        m.w(lab.kinds + 5 * 4, descriptor5)
        lab.answers = answers_for(rng, count, places=True)
        answers = {f'{k[0]}:{k[1]}': list(v) for k, v in lab.answers.items()}
        at = m.alloc(8)
        point = [rnd_word(rng), rnd_word(rng)]
        m.w(at, point[0])
        m.w(at + 4, point[1])
        m.call(0x4DEA20, ecx=lab.group, args=(at,), pop=4)
        cases.append({'members': kinds, 'descriptor5': descriptor5, 'at': point,
                      'answers': answers, 'events': lab.events})
    return cases


def rest_cases(lab, rng):
    m = lab.m
    cases = []
    for number in range(200):
        count = rng.randint(0, 4)
        kinds = member_kinds(rng, count)
        lab.build(rng, count, kinds)
        flags_before = lab.entry_flags(count)
        descriptor5 = rng.choice((0, m.alloc(0x40)))
        m.w(lab.kinds + 5 * 4, descriptor5)
        g = lab.group
        m.w(g + 0x1B8, rng.choice((1, 2, 3)))
        queued = rng.choice((0, 0, 1, 3))
        m.w(g + 0x19C, queued)
        slots = rng.choice((5, 7, 8, 9))
        zones = m.alloc(4 * slots)
        lab.zone7 = m.alloc(0x40) if rng.random() < 0.6 else 0
        for s in range(slots):
            m.w(zones + 4 * s, lab.zone7 if s == 7 else m.alloc(8))
        m.w(g + 0x148, zones)
        m.w(g + 0x14C, slots)
        reform = rng.choice((0, 1, 0x40))
        point = [rnd_word(rng), rnd_word(rng)]
        lab.answers = answers_for(rng, count, places=True)
        answers = {f'{k[0]}:{k[1]}': list(v) for k, v in lab.answers.items()}
        result = m.call(0x4DEE40, ecx=g, args=(point[0], point[1], reform), pop=12) & 0xFF
        cases.append({'members': kinds, 'descriptor5': descriptor5, 'at': point,
                      'queued': queued, 'zone_slots': slots, 'zone7': lab.zone7 != 0,
                      'reform': reform, 'answers': answers, 'flags_before': flags_before,
                      'events': lab.events, 'entry_flags': lab.entry_flags(count),
                      'state': m.r(g + 0x1B8), 'result': result})
    return cases


def main():
    args = sys.argv[1:]
    if args[:1] == ['--exe']:
        h.ORIGINAL = h.Image(Path(args[1]), h.ORIGINAL_SHA)
    rng = random.Random(0x4DE7F0)
    lab = Lab()
    give = give_cases(lab, rng)
    place = place_cases(lab, rng)
    rest = rest_cases(lab, rng)
    body = {
        'schema': 1,
        'passed': True,
        'scope': '0x4DE7F0 / 0x4DEA20 / 0x4DEE40 group order handout (seams listed in the producer).',
        'original_sha256': h.ORIGINAL_SHA,
        'accepted_sha256': h.ACCEPTED_SHA,
        'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'harness_sha256': hashlib.sha256(Path(h.__file__).read_bytes()).hexdigest(),
        'move_vtable': 0xA3CC38,
        'give': give,
        'take_places': place,
        'rest': rest,
        'native_windows': h.visited_windows(lab.m.visited),
    }
    path = HERE / OUT
    path.write_text(json.dumps(body, indent=1) + '\n')
    print(json.dumps({'file': OUT, 'cases': len(give) + len(place) + len(rest),
                      'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
