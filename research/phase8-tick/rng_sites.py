"""Static census of native RNG draw sites.

Primitives (instructions checked by hand, see README.txt):
  0x85D840  cdecl raw draw, state pointer is the single STACK argument.
  0x40C8A0  thiscall bounded draw: push ecx; call 85D840; div [esp+8].
  0x4F02B0  GWorld::GetRNG (fastcall ecx=world): returns world+0xD4 (m_rng).

A linear forward pass over each caller tracks symbolic register values
(this=ecx at entry, [0xB36534]=g_gworld, call results) and the pushed
arguments since the previous call. Each draw site's state pointer is classified:
  world_rng          ret of 0x4F02B0, or g_gworld+0xD4
  this_plus_0xd4     this+0xD4 (GWorld method candidates)
  object_rng         other this/field relative state
  global_rng         fixed .data address
  unresolved         anything the linear pass cannot name
This is a HEURISTIC static census (linear sweep, branches merged), not an
oracle. Reachability uses direct calls in calls.tsv only (virtual calls are
not followed), so "reachable" is a LOWER bound. Output rng-sites.json.
"""
from pathlib import Path
import hashlib, json, sys, collections, re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
NATIVE_AA = GAME / 'reconstruction/research/native-aa'
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
from audit_baseline import PE
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

SOURCE = GAME / 'AA.exe'
NATIVE = PE(SOURCE)
assert hashlib.sha256(NATIVE.b).hexdigest() == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
cs = Cs(CS_ARCH_X86, CS_MODE_32)

RAW, BOUNDED, GETRNG, WORLD = 0x85D840, 0x40C8A0, 0x4F02B0, 0xB36534


def functions():
    out = {}
    for line in (NATIVE_AA / 'functions.tsv').read_text(encoding='utf-8').splitlines()[1:]:
        f = line.split('\t')
        out[int(f[0], 16)] = int(f[3])
    return out


FUNCS = functions()


def load_calls():
    graph = collections.defaultdict(set)
    for line in (NATIVE_AA / 'calls.tsv').read_text(encoding='utf-8').splitlines()[1:]:
        a, b, *_ = line.split('\t')
        if b.startswith('EXTERNAL'):
            continue
        graph[int(a, 16)].add(int(b, 16))
    return graph


GRAPH = load_calls()


def disasm(start):
    size = FUNCS.get(start)
    if not size:
        return []
    return list(cs.disasm(NATIVE.range(start, size), start))


MEM = re.compile(r'^(?:dword ptr )?\[(.+)\]$')
REGS = ('eax', 'ebx', 'ecx', 'edx', 'esi', 'edi', 'ebp')


def value(state, operand):
    operand = operand.strip()
    if operand in state:
        return state[operand]
    if operand in REGS:
        return None
    if re.fullmatch(r'0x[0-9a-f]+|\d+', operand):
        return ('imm', int(operand, 0))
    m = MEM.match(operand)
    if not m:
        return None
    inner = m.group(1).replace(' ', '')
    if re.fullmatch(r'0x[0-9a-f]+', inner):
        return ('load_global', int(inner, 16))
    m2 = re.fullmatch(r'(e[a-z]{2})([+-]0x[0-9a-f]+|[+-]\d+)?', inner)
    if m2 and m2.group(1) in REGS:
        base = state.get(m2.group(1))
        off = int(m2.group(2) or '0', 0)
        if base is not None:
            return ('load', base, off)
    return None


def address(state, operand):
    m = MEM.match(operand.strip())
    if not m:
        return None
    inner = m.group(1).replace(' ', '')
    if re.fullmatch(r'0x[0-9a-f]+', inner):
        return ('imm', int(inner, 16))
    m2 = re.fullmatch(r'(e[a-z]{2})([+-]0x[0-9a-f]+|[+-]\d+)?', inner)
    if m2 and m2.group(1) in REGS and state.get(m2.group(1)) is not None:
        return ('add', state[m2.group(1)], int(m2.group(2) or '0', 0))
    return None


def classify(expr):
    if expr is None:
        return 'unresolved', None
    if expr == ('ret', GETRNG):
        return 'world_rng', 'GetRNG 0x4F02B0'
    if expr == ('add', ('load_global', WORLD), 0xD4):
        return 'world_rng', 'g_gworld+0xD4'
    if expr == ('add', ('this',), 0xD4):
        return 'this_plus_0xd4', 'this+0xD4'
    if expr[0] == 'imm':
        return 'global_rng', f'{expr[1]:#x}'
    if expr[0] == 'load_global':
        return 'global_pointer_rng', f'[{expr[1]:#x}]'
    return 'object_rng', render(expr)


def render(expr):
    if expr is None:
        return '?'
    tag = expr[0]
    if tag == 'this':
        return 'this'
    if tag == 'imm':
        return f'{expr[1]:#x}'
    if tag == 'load_global':
        return f'[{expr[1]:#x}]'
    if tag == 'ret':
        return f'ret({expr[1]:#x})'
    if tag == 'load':
        return f'[{render(expr[1])}+{expr[2]:#x}]'
    if tag == 'add':
        return f'{render(expr[1])}+{expr[2]:#x}'
    return str(expr)


def scan(start):
    insns = disasm(start)
    state = {'ecx': ('this',)}
    pushes = []
    out = []
    for i in insns:
        mn, ops = i.mnemonic, [o.strip() for o in i.op_str.split(', ')] if i.op_str else []
        if mn == 'call':
            target = int(ops[0], 16) if re.fullmatch(r'0x[0-9a-f]+', ops[0]) else None
            if target == RAW:
                out.append((i.address, 'raw_85D840', pushes[-1] if pushes else None))
            elif target == BOUNDED:
                out.append((i.address, 'bounded_40C8A0', state.get('ecx')))
            for r in ('eax', 'ecx', 'edx'):
                state.pop(r, None)
            if target is not None:
                state['eax'] = ('ret', target)
            pushes = []
            continue
        if mn == 'push' and ops:
            pushes.append(value(state, ops[0]))
            continue
        if mn in ('mov', 'lea') and len(ops) == 2 and ops[0] in REGS:
            state[ops[0]] = value(state, ops[1]) if mn == 'mov' else address(state, ops[1])
            if state[ops[0]] is None:
                state.pop(ops[0])
            continue
        if mn in ('add',) and len(ops) == 2 and ops[0] in REGS and re.fullmatch(r'0x[0-9a-f]+|\d+', ops[1]):
            if ops[0] in state and ops[0] != 'esp':
                state[ops[0]] = ('add', state[ops[0]], int(ops[1], 0))
            continue
        if ops and ops[0] in REGS and mn not in ('cmp', 'test', 'push'):
            state.pop(ops[0], None)
    return out


def reach(root):
    seen = {root}
    stack = [root]
    while stack:
        f = stack.pop()
        for g in GRAPH.get(f, ()):
            if g not in seen:
                seen.add(g)
                stack.append(g)
    return seen


def main():
    roots = {'world_tick_4F9970': 0x4F9970, 'admin_tick_755290': 0x755290,
             'frame_75CA70': 0x75CA70, 'world_init_4F27E0': 0x4F27E0}
    reached = {k: reach(v) for k, v in roots.items()}
    sites = []
    callers = sorted({c for c, cs_ in GRAPH.items() if cs_ & {RAW, BOUNDED}})
    for caller in callers:
        for site, prim, expr in scan(caller):
            kind, name = classify(expr)
            sites.append({'function': f'{caller:08x}', 'site': f'{site:08x}', 'primitive': prim,
                          'state_kind': kind, 'state': name,
                          **{f'reachable_{k}': caller in v for k, v in reached.items()}})
    getter = sorted({c for c, cs_ in GRAPH.items() if GETRNG in cs_})
    # Raw E8 rel32 scan of executable sections: call sites outside any
    # Ghidra function body (e.g. the periodic queue add 0x50B2C0).
    import bisect
    starts = sorted(FUNCS)
    def inside(va):
        i = bisect.bisect_right(starts, va) - 1
        return i >= 0 and va < starts[i] + FUNCS[starts[i]]
    orphan = []
    for vs, rva, size, off in NATIVE.sections:
        if rva + NATIVE.base > 0x9A0000 or rva + NATIVE.base < 0x401000:
            continue
        blob = NATIVE.b[off:off + size]
        base = NATIVE.base + rva
        pos = blob.find(bytes([0xE8]))
        while pos != -1 and pos + 5 <= len(blob):
            target = (base + pos + 5 + int.from_bytes(blob[pos + 1:pos + 5], 'little', signed=True)) & 0xFFFFFFFF
            if target in (RAW, BOUNDED, GETRNG) and not inside(base + pos):
                # confirm it decodes as a call at this address
                ins = next(cs.disasm(blob[pos:pos + 5], base + pos), None)
                if ins is not None and ins.mnemonic == 'call':
                    orphan.append({'site': f'{base + pos:08x}', 'target': f'{target:08x}'})
            pos = blob.find(bytes([0xE8]), pos + 1)
    summary = collections.Counter((s['state_kind'], s['state']) for s in sites)
    report = {
        'schema': 1,
        'scope': __doc__,
        'original_sha256': hashlib.sha256(NATIVE.b).hexdigest(),
        'heuristic_static_census': True,
        'reachability_lower_bound_direct_calls_only': True,
        'reachable_function_counts': {k: len(v) for k, v in reached.items()},
        'state_summary': [{'kind': k, 'state': v, 'sites': n} for (k, v), n in summary.most_common()],
        'sites': sites,
        'call_sites_outside_ghidra_functions': orphan,
        'world_rng_getter_4F02B0_callers': [
            {'function': f'{c:08x}', **{f'reachable_{k}': c in v for k, v in reached.items()}} for c in getter],
    }
    out = HERE / 'rng-sites.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    world_sites = [s for s in sites if s['state_kind'] == 'world_rng']
    print(json.dumps({
        'sites': len(sites),
        'summary': report['state_summary'][:15],
        'world_rng_sites': len(world_sites),
        'world_rng_sites_reachable_from_world_tick': sum(s['reachable_world_tick_4F9970'] for s in world_sites),
        'world_rng_sites_reachable_from_admin_tick': sum(s['reachable_admin_tick_755290'] for s in world_sites),
        'getter_callers': len(getter),
        'orphan_sites': report['call_sites_outside_ghidra_functions'],
        'sha256': hashlib.sha256(out.read_bytes()).hexdigest()}, indent=1))


if __name__ == '__main__':
    main()
