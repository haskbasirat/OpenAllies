"""Original ProcessRequest / decree-handler control flow for every descriptor.

Runs each ORIGINAL handler to completion under Unicorn with the payload
decoded from generated bits (same placement as native_order_traces.py), and
records, in order: sync-log entries (0x77DA70 int / 0x40F8D0+0x40F950 string,
synchronizer A=[0xB3655C] or S=[0xB36560]), rejection log lines (0x404F30),
the call of the order's process function (stubbed: it is the gameplay/lobby
effect, a declared seam), writes to the decree stream admin+0x1F8 and to the
mirror stream admin+0x270, and the 0x766C50 mirror query.

Configurations per request: accepted, no ordering client (admin+0x284=0),
version not acknowledged (client+0x6C=0), film session (session+0xE4=4),
accepted with mirror. Per decree: mirror off/on. Per-order validity
functions found statically (0x5092F0) are forced true as in the trace oracle;
the inline cheat permission reads [0xB39AB0]+0x10C/+0x124 (enabled).
"""
from pathlib import Path
import json, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_order_traces import (TraceMachine, DESCRIPTORS, containing, ret_pop, STATIC, sha, PE0, PE1,
                                 OUT_BASE, WRITER_VTABLE)
from unicorn.x86_const import *

SYNC_A = 0x7600000; SYNC_S = 0x7600100
# Decree handlers whose process is inlined: (first instruction, resume address).
# 87 KKC_TellActorCommandOrderDecree: 0x61FA53..0x61FA9C is the phase5
# process_actor block (permission +0x150, link 0x44DEA0, delivery 0x4D9B60).
INLINE_PROCESS = {87: (0x61fa53, 0x61fa9c)}
MIRROR_BASE = 0x6180000

def virtual_process_site(handler):
    """Order-object virtual Execute call (`call [reg+4]`) that follows the
    first decree-type log 0x40F8D0; returns (call address, resume address,
    stack bytes the callee pops = pushes since the previous call)."""
    from natdis import insns
    from native_order_traces import extent
    lo, hi = extent(handler); seen = False; pushes = 0
    for i in insns(lo, size=hi - lo):
        if i.mnemonic == 'call' and i.op_str == '0x40f8d0': seen = True; pushes = 0; continue
        if i.mnemonic == 'call':
            if seen and i.op_str.startswith('dword ptr ['):
                return (i.address, i.address + i.size, 4 * pushes)
            pushes = 0; continue
        if i.mnemonic == 'push': pushes += 1
        elif i.mnemonic == 'add' and i.op_str.startswith('esp,'): pushes = 0
    raise ValueError(hex(handler))

class FlowMachine(TraceMachine):
    def __init__(self):
        self.flow_mode = False; self.flow = []; self.process = None; self.mirror = False
        super().__init__()
        self.u.mem_map(SYNC_A, 0x1000)
        self.w(0xb3655c, SYNC_A); self.w(0xb36560, SYNC_S)
        self.mark = self.heap
    def hook(self, u, pc, size, user):
        if not (self.mode == 'trace' and self.flow_mode):
            return super().hook(u, pc, size, user)
        p = self.pending
        if p is not None and pc == p['ret'] and u.reg_read(UC_X86_REG_ESP) == p['esp']:
            return super().hook(u, pc, size, user)
        if pc == 0x4012c0:
            return  # no early stop: run the handler to completion
        if pc == 0x40d420:
            stream = self.arg(0)
            if stream in (self.admin + 0x1f8, self.admin + 0x270):
                ret = self.r(u.reg_read(UC_X86_REG_ESP)); width = self.arg(2)
                data = bytes(self.u.mem_read(self.arg(1), (width + 7) // 8))
                value = struct.unpack('<I', (data + b'\0' * 4)[:4])[0] & ((1 << width) - 1) if width <= 32 else data.hex()
                self.flow.append({'kind': 'write', 'stream': 'decree' if stream == self.admin + 0x1f8 else 'mirror',
                                  'width': width, 'value': value, 'fn': f'{containing(ret):08x}'})
                self.visited[pc] = size
                return
        if pc == 0x77da70:
            ecx = u.reg_read(UC_X86_REG_ECX)
            self.flow.append({'kind': 'sync_int', 'log': 'A' if ecx == SYNC_A else 'S' if ecx == SYNC_S else hex(ecx),
                              'label': self.text(self.arg(0)), 'value': self.arg(1)})
            self.ret(12); return
        if pc in (0x40f8d0, 0x40f950):
            self.flow.append({'kind': 'sync_string', 'log': 'A' if pc == 0x40f8d0 else 'S',
                              'label': self.text(self.arg(0)), 'value': self.text(self.arg(1))})
            self.ret(); return
        if pc == 0x404f30:
            self.flow.append({'kind': 'rejected', 'line': self.arg(2), 'format': self.text(self.arg(4))})
            self.ret(); return
        if pc == 0x766c50:
            self.flow.append({'kind': 'mirror_query', 'value': self.mirror}); self.ret(0, int(self.mirror)); return
        if self.process is not None and pc == self.process[0]:
            self.flow.append({'kind': 'process', 'function': f'{pc:#x}'})
            if self.process[1] == 'inline':
                u.reg_write(UC_X86_REG_EIP, self.process[2])
                if len(self.process) > 3:
                    u.reg_write(UC_X86_REG_ESP, u.reg_read(UC_X86_REG_ESP) + self.process[3])
            else:
                self.ret(self.process[1], 0)
            return
        return super().hook(u, pc, size, user)
    def run_flow(self, d, seed, config):
        ident, name, handler = d
        info = STATIC[ident]
        proc = info.get('process')
        self.process = (int(proc, 16), ret_pop(int(proc, 16))) if isinstance(proc, str) and proc.startswith('0x') else None
        if ident in INLINE_PROCESS:
            self.process = (INLINE_PROCESS[ident][0], 'inline', INLINE_PROCESS[ident][1])
        elif isinstance(proc, str) and proc.startswith('dword ptr'):
            site = virtual_process_site(handler)
            self.process = (site[0], 'inline', site[1], site[2])
        self.mirror = config.get('mirror', False)
        mirror = self.admin + 0x270
        self.w(MIRROR_BASE, 1); self.w(MIRROR_BASE + 4, 0x10000 - 16)
        self.u.mem_write(MIRROR_BASE + 8, b'\0' * (0x10000 - 16))
        self.w(mirror, WRITER_VTABLE); self.w(mirror + 4, MIRROR_BASE + 8); self.w(mirror + 8, 0); self.w(mirror + 12, 3)
        self.flow = []; self.flow_mode = True
        validity = [int(v, 16) for v in info.get('validity') or []]
        # Configuration is applied after run() resets streams: patch via a pre-run hook.
        self.config = config
        r = self.run(d, seed, 'small', force=validity)
        self.flow_mode = False
        return {'config': config['name'], 'status': r['status'], 'error': r['error'], 'input': r['input'], 'end': r['end'],
                'reads': r['reads'], 'events': self.flow, 'forced_validity': [f'{v:#x}' for v in validity]}
    def run(self, d, seed, policy, force=()):
        c = getattr(self, 'config', {}) if self.flow_mode else {}
        self.w(self.admin + 0x284, 0 if c.get('no_client') else self.player)
        self.u.mem_write(self.client + 0x6c, b'\x00' if c.get('no_version') else b'\x01')
        try:
            return super().run(d, seed, policy, force)
        finally:
            self.w(self.admin + 0x284, self.player); self.u.mem_write(self.client + 0x6c, b'\x01')
            self.w(self.session + 0xe4, 0)

# film flag must be set after TraceMachine.run's own setup; it does not touch +0xE4,
# so set it here around the base run.
_base_run = TraceMachine.run
def _run_with_session(self, d, seed, policy, force=()):
    c = getattr(self, 'config', {}) if getattr(self, 'flow_mode', False) else {}
    self.w(self.session + 0xe4, 4 if c.get('film') else 0)
    return _base_run(self, d, seed, policy, force)
TraceMachine.run = _run_with_session

CONFIGS_REQUEST = [{'name': 'accepted'}, {'name': 'no_ordering_client', 'no_client': True},
                   {'name': 'version_not_acked', 'no_version': True}, {'name': 'film_session', 'film': True},
                   {'name': 'accepted_mirror', 'mirror': True}]
CONFIGS_DECREE = [{'name': 'executed'}, {'name': 'executed_mirror', 'mirror': True}]

def main(only=None):
    m = FlowMachine(); out = []
    for d in DESCRIPTORS:
        if only and d[0] not in only: continue
        configs = CONFIGS_REQUEST if d[1].endswith('Request') else CONFIGS_DECREE
        rows = [m.run_flow(d, 0, c) for c in configs]
        out.append({'id': d[0], 'name': d[1], 'handler': f'{d[2]:#x}', 'process': STATIC[d[0]].get('process'), 'cases': rows})
        print(d[0], d[1], [(r['config'], r['status'], len(r['events'])) for r in rows], flush=True)
    return m, out

if __name__ == '__main__':
    only = set(int(x) for x in sys.argv[1:]) or None
    m, out = main(only)
    report = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(PE0.b), 'v14_sha256': sha(PE1.b),
              'producer_sha256': sha(Path(__file__).read_bytes()), 'managers': {f'{g:#x}': c for g, c in m.managers.items()},
              'descriptors': out}
    (HERE / ('order-flow.json' if only is None else 'order-flow-partial.json')).write_text(json.dumps(report) + '\n')
