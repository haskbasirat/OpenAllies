"""Static census of x87/SSE control-state changes in AA.exe (read-only).

Finds every FLDCW / FNINIT / FINIT / FLDENV / FRSTOR / FXRSTOR / LDMXCSR
instruction (linear sweep of executable sections; each hit is re-decoded at its
containing Ghidra function to reject mid-instruction false positives), every
direct call to _controlfp 0x96F7AA / _control87 0x96F778, and every
IDirect3D9::CreateDevice call (vtable +0x40 on the IDirect3D9 pointer
0xB443F4). Each site gets its containing function and whether that function
is reachable (direct calls, calls.tsv) from the frame loop 0x765410, the
world tick 0x4F9970 and the admin tick 0x755290. Output: fpu-sites.json.
Heuristic/static: virtual calls are not followed, so reachability is a lower bound.
"""
from pathlib import Path
import bisect, collections, hashlib, json, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
NATIVE_AA = GAME / 'reconstruction/research/native-aa'
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
from audit_baseline import PE
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

NATIVE = PE(GAME / 'AA.exe')
assert hashlib.sha256(NATIVE.b).hexdigest() == '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
cs = Cs(CS_ARCH_X86, CS_MODE_32)

FUNCS = {}
NAMES = {}
for line in (NATIVE_AA / 'functions.tsv').read_text(encoding='utf-8').splitlines()[1:]:
    f = line.split('\t')
    FUNCS[int(f[0], 16)] = int(f[3])
    NAMES[int(f[0], 16)] = f[1]
STARTS = sorted(FUNCS)
GRAPH = collections.defaultdict(set)
for line in (NATIVE_AA / 'calls.tsv').read_text(encoding='utf-8').splitlines()[1:]:
    a, b, *_ = line.split('\t')
    if not b.startswith('EXTERNAL'):
        GRAPH[int(a, 16)].add(int(b, 16))


def containing(va):
    i = bisect.bisect_right(STARTS, va) - 1
    if i >= 0 and va < STARTS[i] + FUNCS[STARTS[i]]:
        return STARTS[i]
    return None


def reach(root):
    seen, stack = {root}, [root]
    while stack:
        for g in GRAPH.get(stack.pop(), ()):
            if g not in seen:
                seen.add(g)
                stack.append(g)
    return seen


ROOTS = {'frame_765410': 0x765410, 'world_tick_4F9970': 0x4F9970, 'admin_tick_755290': 0x755290,
         'admin_frame_75CA70': 0x75CA70}
REACH = {k: reach(v) for k, v in ROOTS.items()}
WANTED = ('fldcw', 'fninit', 'finit', 'fldenv', 'frstor', 'fxrstor', 'ldmxcsr', 'fnclex')


def decode_function(start):
    return list(cs.disasm(NATIVE.range(start, FUNCS[start]), start))


def main():
    sites = []
    seen_funcs = {}
    for start in STARTS:
        if FUNCS[start] > 200000:
            continue
        try:
            insns = decode_function(start)
        except ValueError:
            continue
        for i in insns:
            m = i.mnemonic
            hit = m in WANTED
            target = None
            if m == 'call' and i.op_str.startswith('0x'):
                target = int(i.op_str, 16)
                hit = hit or target in (0x96F7AA, 0x96F778)
            if hit:
                sites.append({'site': f'{i.address:08x}', 'function': f'{start:08x}', 'name': NAMES[start],
                              'instruction': f'{m} {i.op_str}',
                              **{f'reachable_{k}': start in v for k, v in REACH.items()}})
    # CreateDevice: [IDirect3D9 0xB443F4] vtable +0x40
    create = []
    for start in STARTS:
        if FUNCS[start] > 200000:
            continue
        try:
            insns = decode_function(start)
        except ValueError:
            continue
        loaded = set()
        for i in insns:
            ops = i.op_str.split(', ')
            if i.mnemonic == 'mov' and len(ops) == 2 and ops[1] == 'dword ptr [0xb443f4]':
                loaded.add(ops[0])
            if i.mnemonic == 'call' and '+ 0x40]' in i.op_str:
                create.append({'site': f'{i.address:08x}', 'function': f'{start:08x}',
                               'instruction': f'{i.mnemonic} {i.op_str}', 'idirect3d9_loaded_in_function': bool(loaded)})
    create = [c for c in create if c['idirect3d9_loaded_in_function']]
    by_kind = collections.Counter(s['instruction'].split()[0] for s in sites)
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': hashlib.sha256(NATIVE.b).hexdigest(),
              'reachable_function_counts': {k: len(v) for k, v in REACH.items()},
              'counts': dict(by_kind), 'sites': sites, 'create_device_candidates': create}
    out = HERE / 'fpu-sites.json'
    out.write_text(json.dumps(report, indent=1) + '\n')
    tick = [s for s in sites if any(s[f'reachable_{k}'] for k in REACH)]
    print(json.dumps({'sites': len(sites), 'counts': dict(by_kind), 'reachable_from_loop_or_tick': len(tick),
                      'create_device_candidates': create, 'sha256': hashlib.sha256(out.read_bytes()).hexdigest()},
                     indent=1))
    for s in tick:
        print(s['site'], s['function'], s['name'], s['instruction'],
              [k for k in REACH if s[f'reachable_{k}']])


if __name__ == '__main__':
    main()
