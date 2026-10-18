"""Execute original 4FDA70 traversal, checking all executed bytes against v14.

Seams: resolver allocation/construction/destruction, GTID next-object iterator,
owner vf108, Nation 473370 substitution, replacement vf38, 4B6FF0 random choice,
77DC50/77DA70 logging, 404F60 missing-substitution diagnostic, replacement request
4FD7F0/4FD8C0 and final processing 4FC4A0. No creation/publication is claimed.
Native code performs category/definition/count/kingdom/RNG reads and every branch.
The imported guarded machine traps unseamed allocators/imports and non-FS page 0.
"""
from pathlib import Path
import hashlib
import json
import random
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-metagame'))
from mg_machine import Machine, P, V, VISITED, ORIGINAL_SHA256, V14_SHA256
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_ECX


def run(spec, cw):
    m = Machine(cw)
    trace = []
    reads = {}
    def watch(address, event):
        reads[address] = event
    def memory(u, access, address, size, value, _):
        if address in reads:
            trace.append(reads[address]())
    m.u.hook_add(UC_HOOK_MEM_READ, memory)
    def seam(address, callback):
        m.seams[address] = callback
    def virtual(callback):
        p = m.alloc(16)
        m.u.mem_write(p, b'\xc3')
        seam(p, callback)
        return p

    registry = m.alloc(0x60020)
    m.w32(0xb364c0, registry)
    m.w32(registry + 0x60004, spec.get('count', len(spec['objects'])))
    watch(registry + 0x60004, lambda: ['count', m.r32(registry + 0x60004)])
    world = m.alloc(0x300)
    m.w32(world + 0xd4, spec['seed'])
    watch(world + 0xd4, lambda: ['rng', m.r32(world + 0xd4)])
    m.w32(0xb3655c, 0x111111)
    m.w32(0xb36560, 0x222222)

    object_ptrs = []
    definitions = {}
    definition_ids = {}
    kingdoms = {}
    nation_ids = {}
    pointers = {}
    settings = {}
    owner_calls = {}
    def definition(token, category=0, actor=False):
        if token is None:
            return 0
        if token not in definitions:
            p = m.alloc(0x600)
            cat = m.alloc(32)
            vt = m.alloc(0x40)
            definitions[token] = p
            definition_ids[p] = token
            m.w32(p + 0x18, cat)
            m.w32(cat + 0xc, category)
            watch(cat + 0xc, lambda p=p, cat=cat: ['category', definition_ids[p], m.r32(cat + 0xc)])
            def vf(machine, token=token, actor=actor):
                trace.append(['is_actor', token, actor])
                machine.ret(0, 0x100 if actor else 0)
            m.w32(vt + 0x38, virtual(vf))
            m.w32(p, vt)
        return definitions[token]

    def kingdom(token, blocked=False):
        if token is None:
            return 0
        if token not in kingdoms:
            p = m.alloc(0x240)
            n = m.alloc(0x30)
            kingdoms[token] = p
            nation_ids[n] = token + 500
            m.w32(p + 0x190, int(blocked))
            m.w32(p + 0x234, n)
            watch(p + 0x190, lambda p=p, token=token: ['kingdom_word', token, m.r32(p + 0x190)])
            watch(p + 0x234, lambda p=p, token=token: ['nation', token, nation_ids.get(m.r32(p + 0x234))])
        return kingdoms[token]

    for i, obj in enumerate(spec['objects']):
        token = i + 1
        d = definition(token + 1000, obj['category'])
        replacement = definition(obj.get('replacement'), actor=obj.get('actor', False))
        if obj.get('changed_definition'):
            definition(token + 10000, obj['category'])
        p = m.alloc(0x200)
        pointers[p] = token
        settings[token] = obj
        owner_calls[token] = 0
        object_ptrs.append(p)
        vt = m.alloc(0x110)
        m.w32(p, vt)
        m.w32(p + 4, d)
        m.w32(p + 0x14, token + 7000)
        watch(p + 4, lambda p=p, token=token: ['definition', token, definition_ids[m.r32(p + 4)]])
        owners = obj.get('owners', [None])
        for owner in owners:
            kingdom(owner, obj.get('blocked', False))
        def owner(machine, p=p, token=token, owners=owners, obj=obj):
            index = owner_calls[token]
            answer = owners[min(index, len(owners) - 1)]
            owner_calls[token] += 1
            trace.append(['owner', token, answer])
            if index == 1 and obj.get('changed_definition'):
                machine.w32(p + 4, definitions[token + 10000])
            machine.ret(0, kingdoms.get(answer, 0))
        m.w32(vt + 0x108, virtual(owner))

    # The two resolver vector constructors have no gameplay-side effects;
    # native allocation failure/unwinding and their member constructors are
    # not claimed. They are grouped into the explicit begin boundary.
    seam(0x6f7810, lambda machine: machine.ret(0, machine.alloc(machine.arg(0))))
    seam(0x96d18d, lambda machine: machine.ret(20))
    def begin(machine):
        assert [machine.arg(0), machine.arg(1)] == [8, 1]
        trace.append(['begin'])
        machine.ret(8)
    seam(0x4f5160, begin)
    def next_object(machine):
        cursor = machine.arg(0)
        old = machine.r32(cursor)
        p = object_ptrs[old] if old < len(object_ptrs) else 0
        new = old + 1 if p else old
        machine.w32(cursor, new)
        trace.append(['next', old, new, pointers.get(p)])
        machine.ret(4, p)
    seam(0x4efbe0, next_object)
    def substitute(machine):
        d = definition_ids[machine.arg(0)]
        # Current actor is the last owner caller; this seam's actual inputs
        # are the native Nation this-pointer and reread placeholder definition.
        token = d - (10000 if d >= 10000 else 1000)
        answer = settings[token].get('replacement')
        trace.append(['substitute', nation_ids.get(machine.u.reg_read(UC_X86_REG_ECX)), d, answer])
        machine.ret(4, definitions.get(answer, 0))
    seam(0x473370, substitute)
    def missing(machine):
        trace.append(['missing', machine.arg(5) - 7000])
        machine.ret()
    seam(0x404f60, missing)
    def random_scope(machine):
        scope = {0xa2d3c8: 'object', 0xa2d388: 'actor'}[machine.arg(0)]
        assert machine.arg(1) == 0
        trace.append(['scope', scope])
        machine.ret(8)
    seam(0x77dc50, random_scope)
    def rng_log(machine):
        channel = {0x111111: 0, 0x222222: 1}[machine.u.reg_read(UC_X86_REG_ECX)]
        assert machine.arg(0) == 0xa2acd4 and machine.arg(2) == 0
        trace.append(['log_rng', channel, machine.arg(1)])
        if spec.get('mutate_log'):
            machine.w32(world + 0xd4, machine.r32(world + 0xd4) + channel + 17)
        machine.ret(12)
    seam(0x77da70, rng_log)
    def choose(machine):
        table = machine.u.reg_read(UC_X86_REG_ECX)
        token, offset = next((token, offset) for token, p in definitions.items()
                             for offset in [0x170, 0x5c0] if p + offset == table)
        obj = settings[token - 1000]
        assert machine.arg(0) == world + 0xd4
        before = machine.r32(world + 0xd4)
        after = (before * 1664525 + 1013904223) & 0xffffffff
        # Scripted RNG mutation at this explicit seam, not a port or parity
        # claim for the weighted selector's own algorithm.
        machine.w32(world + 0xd4, after)
        trace.append(['choose', token, 'object' if offset == 0x170 else 'actor',
                      obj.get('replacement')])
        machine.ret(4, definitions.get(obj.get('replacement'), 0))
    seam(0x4b6ff0, choose)
    def queue(machine, kind):
        token = pointers[machine.arg(0)]
        trace.append(['queue', token, definition_ids.get(machine.arg(1)), kind])
        if 'count_after' in settings[token]:
            machine.w32(registry + 0x60004, settings[token]['count_after'])
        machine.ret(8)
    seam(0x4fd7f0, lambda machine: queue(machine, 'object'))
    seam(0x4fd8c0, lambda machine: queue(machine, 'actor'))
    def end(machine, name):
        trace.append([name])
        machine.ret()
    seam(0x4fc4a0, lambda machine: end(machine, 'process'))
    seam(0x4f6eb0, lambda machine: end(machine, 'finish'))
    try:
        result, _ = m.call(0x4fda70, ecx=world)
        assert result & 0xff == 1
        outcome = {'success': True}
    except Exception as error:
        pc = getattr(error, 'pc', None)
        if pc not in [0x4fdb56, 0x4fdb6f]:
            raise
        outcome = {'null_owner': 2 if pc == 0x4fdb56 else 3}
    return {'input': spec, 'cw': cw, 'trace': trace, 'outcome': outcome,
            'rng_final': m.r32(world + 0xd4)}


def main():
    cases = [
        {'name': 'empty', 'objects': [], 'seed': 1},
        {'name': 'negative_count', 'objects': [{'category': 0x22}], 'count': -1, 'seed': 2},
        {'name': 'owner_changes_and_definition_reread', 'seed': 3, 'objects': [
            {'category': 0x1f, 'owners': [1, 2, 3], 'changed_definition': True,
             'replacement': 20001, 'actor': True}]},
        {'name': 'owner_second_null', 'seed': 4, 'objects': [
            {'category': 0x1f, 'owners': [1, None]}]},
        {'name': 'owner_third_null', 'seed': 5, 'objects': [
            {'category': 0x1f, 'owners': [1, 2, None]}]},
        {'name': 'count_reread', 'seed': 6, 'objects': [
            {'category': 0x22, 'replacement': 20002, 'count_after': 0},
            {'category': 0x23, 'replacement': 20003}]},
        {'name': 'logs_reread_rng', 'seed': 0xffffffff, 'mutate_log': True, 'objects': [
            {'category': 0x22, 'replacement': None},
            {'category': 0x23, 'replacement': 20004}]},
        {'name': 'all_branches', 'seed': 42, 'objects': [
            {'category': 0}, {'category': 0x1f},
            {'category': 0x1f, 'owners': [10], 'blocked': True},
            {'category': 0x1f, 'owners': [11]},
            {'category': 0x1f, 'owners': [12], 'replacement': 20005, 'actor': False},
            {'category': 0x1f, 'owners': [13], 'replacement': 20006, 'actor': True},
            {'category': 0x22, 'replacement': 20007},
            {'category': 0x23, 'replacement': 20008}]},
    ]
    randomizer = random.Random(0x4fda70)
    for case in range(24):
        objects = []
        for index in range(randomizer.randrange(1, 16)):
            objects.append({'category': randomizer.choice([0, 3, 0x1f, 0x22, 0x23]),
                            'owners': [None if randomizer.randrange(3) == 0 else index + 100],
                            'blocked': bool(randomizer.randrange(3) == 0),
                            'replacement': None if randomizer.randrange(4) == 0 else 21000 + index,
                            'actor': bool(randomizer.randrange(2))})
        cases.append({'name': f'generated_{case}', 'seed': randomizer.getrandbits(32), 'objects': objects})
    rows = [run(case, cw) for case in cases for cw in [0x007f, 0x027f, 0x037f]]
    out = HERE / 'native-map-prepare.json'
    if out.exists():
        raise RuntimeError('Refusing to overwrite existing oracle fixture')
    report = {'scope': __doc__, 'original_sha256': ORIGINAL_SHA256, 'v14_sha256': V14_SHA256,
              'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'machine_sha256': hashlib.sha256((ROOT / 'research/phase8-metagame/mg_machine.py').read_bytes()).hexdigest(),
              'function_bytes_sha256': hashlib.sha256(P.range(0x4fda70, 0x26d)).hexdigest(),
              'function_bytes_identical_to_v14': P.range(0x4fda70, 0x26d) == V.range(0x4fda70, 0x26d),
              'rows': rows, 'executed': [{'address': hex(k), 'bytes': v} for k, v in sorted(VISITED.items())]}
    out.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'rows': len(rows), 'instructions': len(VISITED),
                      'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
