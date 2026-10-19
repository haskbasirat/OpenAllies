"""The original [B36548] TGC_MilesAudioManager executing unchanged under
Unicorn: constructor 749130, Init 74AD70, PlaySound 7474F0, 747630, 7476E0,
Update 747A80, the Miles EOS callbacks 749510/749550/749590, volume writes
744F90/7450E0, command queue 747150/747180/7471B0, hold 745580/7455B0 and
disable/enable 745530/745510, with every function they reach (TGC_MilesAudio
virtuals, channel pools, 746130/746A80/746C00/746F40, 7455E0, 745FA0,
746830, 46B0D0/46B190/46B110/46B2E0/46B340/46B3A0, 4453A0 ...).

Seams (explicit, recorded): allocator 6F7810/6F7430, critical sections
857D60/857EE0 (stub vtable), QPC timers 59AA70/59AB60/59ABA0 on a fake
millisecond clock (frequency 1000), every mss32.dll import (IAT stubs that
record arguments and return scripted handles), Interlocked*/GetCurrentThreadId,
UVar/AVar lookups 4B2E60/405370/4B2C30/453ED0/410CB0/454810, the UI random
draw 85D840 (scripted values), 43AAF0, 76AEE0, 5F41F0, 40EAA0, 5C0E50,
5543F0/40CFA0/54F3E0 (fog), 428790/45C7A0/428AF0 (FX context), 74A130 and
5AA340 (stream names), the diagnostic sinks 404ED0/404F30/404EA0/405CB0 and
the CString allocator 856370. Game objects, the B364C0 handle table, the
camera, AudioData and AudioResource objects are plain memory built per case.
Nothing here writes game files.
"""
from pathlib import Path
import json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / 'phase8-ai'))
from native_machine import Machine, header, instructions, sha, f32, from_f32
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EAX

IAT = {
    0x9f046c: ('close_digital_driver', 1), 0x9f0470: ('shutdown', 0), 0x9f0474: ('close_3d_listener', 1),
    0x9f0478: ('close_3d_provider', 1), 0x9f047c: ('set_3d_sample_volume', 2), 0x9f0480: ('set_sample_volume_pan', 3),
    0x9f0484: ('set_3d_speaker_type', 2), 0x9f0488: ('set_3d_orientation', 7), 0x9f048c: ('start_sample', 1),
    0x9f0490: ('register_stream_callback', 2), 0x9f0494: ('register_3d_eos_callback', 2),
    0x9f0498: ('register_eos_callback', 2), 0x9f049c: ('resume_3d_sample', 1), 0x9f04a0: ('resume_sample', 1),
    0x9f04a4: ('pause_stream', 2), 0x9f04a8: ('stop_3d_sample', 1), 0x9f04ac: ('stop_sample', 1),
    0x9f04b0: ('stream_status', 1), 0x9f04b4: ('sample_3d_status', 1), 0x9f04b8: ('sample_status', 1),
    0x9f04bc: ('enumerate_3d_providers', 3), 0x9f04c0: ('mem_free_lock', 1), 0x9f04c4: ('set_preference', 2),
    0x9f04c8: ('open_stream', 3), 0x9f04cc: ('close_stream', 1), 0x9f04d0: ('release_3d_sample_handle', 1),
    0x9f04d4: ('release_sample_handle', 1), 0x9f04d8: ('allocate_sample_handle', 1), 0x9f04dc: ('init_sample', 1),
    0x9f04e0: ('allocate_3d_sample_handle', 1), 0x9f04e4: ('set_sample_playback_rate', 2),
    0x9f04e8: ('sample_playback_rate', 1), 0x9f04ec: ('set_sample_file', 3), 0x9f04f0: ('start_3d_sample', 1),
    0x9f04f4: ('set_3d_sample_playback_rate', 2), 0x9f04f8: ('sample_3d_playback_rate', 1),
    0x9f04fc: ('set_3d_sample_distances', 3), 0x9f0500: ('set_3d_sample_file', 2), 0x9f0504: ('open_3d_listener', 1),
    0x9f0508: ('wav_info', 2), 0x9f050c: ('open_3d_provider', 1), 0x9f0510: ('file_error', 0),
    0x9f0514: ('last_error', 0), 0x9f0518: ('file_read', 2), 0x9f051c: ('start_stream', 1),
    0x9f0520: ('stream_ms_position', 3), 0x9f0524: ('set_file_callbacks', 4), 0x9f0528: ('open_digital_driver', 4),
    0x9f052c: ('startup', 0), 0x9f0530: ('set_redist_directory', 1), 0x9f0534: ('set_3d_position', 4),
    0x9f0538: ('set_stream_volume_pan', 3),
}
CALLBACKS = {0x749510: 'eos_2d', 0x749550: 'eos_3d', 0x749590: 'eos_stream'}
MGR_SIZE = 0x418


class AudioMachine(Machine):
    def __init__(self, precision, case):
        super().__init__(precision)
        self.case = case
        self.now = case.get('start_ms', 1000)
        self.trace = []
        self.handles = {}
        self.next_handle = {'sample': 0x2000, 'sample3d': 0x3000, 'stream': 0x5000}
        self.status = {}
        self.rates = {}
        self.draws = list(case.get('draws', []))
        self.draw_count = 0
        self.thread = 1
        self.env = dict(case['env'])
        self.names = {}
        # Globals.
        self.ui = self.alloc(0x200)
        self.w(0xb3653c, self.ui)
        self.world = self.alloc(0x100)
        self.w(0xb36534, self.world)
        self.vworld = self.alloc(0x200)
        self.w(0xb36538, self.vworld)
        self.w(self.world + 0x30, self.alloc(0x40))
        self.game = self.alloc(0x40)
        self.w(0xb36554, self.game)
        self.svars = self.alloc(0x200)
        self.svar_cell = self.alloc(4)
        self.w(self.svars + 0x114, self.svar_cell)
        self.w(0xb39ab0, self.svars)
        self.w(0xb3e0a8, self.alloc(0x20))
        self.w(0xb3974c, 0)
        self.w(0xb3656c, 0)
        self.app_thread = self.alloc(0x60)
        self.w(self.app_thread + 0x48, 1)
        self.w(0xb364d4, self.app_thread)
        self.camera_slot = self.alloc(4)
        self.camera = self.alloc(0x300)
        self.w(0xb40048, self.camera_slot)
        self.w(0xb4004c, self.alloc(0x10))
        self.table = self.alloc(0x20004 + 4 * 0x10000)
        self.w(0xb364c0, self.table)
        self.w(0xb396e0, self.env.get('timer_anchor', 0))
        self.w(0xb396e4, 0)
        # UVar table [B39940]: user volumes by registration index.
        uv = self.alloc(4 * 0x200)
        self.w(0xb39940, uv)
        for index, value in case['user_volumes'].items():
            cell = self.alloc(4)
            self.w(cell, f32(value))
            self.w(uv + 4 * int(index, 16), cell)
        self.vars = {}
        for name, (kind, value) in case['vars'].items():
            record = self.alloc(0x40)
            self.w(record, f32(value) if kind == 'f' else value & 0xffffffff)
            self.w(record + 0x24, 2 if kind == 'f' else 3)
            slot = self.alloc(4)
            self.w(slot, record)
            self.vars[name] = (record, slot)
        self.lock_vt = self.alloc(0x20)
        self.w(self.lock_vt + 4, self.stub(lambda m: m.ret(0)))
        self.w(self.lock_vt + 8, self.stub(lambda m: m.ret(0)))
        self.cstring_header = self.alloc(0x40)
        self.w(self.cstring_header + 0xc, 0x10000)
        cs_vt = self.alloc(0x20)
        self.w(cs_vt + 0xc, self.stub(lambda m: m.ret(self.cstring_header)))
        self.cs_allocator = self.alloc(0x10)
        self.w(self.cs_allocator, cs_vt)
        for va, (name, n) in IAT.items():
            self.w(va, self.stub(self.make_ail(name, n)))
        self.w(0x9f01ec, self.stub(self.s_decrement))
        self.w(0x9f01e8, self.stub(self.s_increment))
        self.w(0x9f01e4, self.stub(lambda m: m.ret(m.thread)))
        seams = {
            0x6f7810: self.s_alloc, 0x6f7430: lambda m: m.ret(0), 0x857d60: self.s_lock_ctor,
            0x857ee0: lambda m: m.ret(0), 0x59aa70: lambda m: m.ret(0), 0x59ab60: self.s_timer_start,
            0x59aba0: self.s_timer_elapsed, 0x4b2e60: self.s_var_record, 0x405370: self.s_var_slot,
            0x4b2c30: self.s_debug_ids, 0x453ed0: self.s_avar_float, 0x410cb0: self.s_redist,
            0x454810: self.s_narrow, 0x855c50: lambda m: m.ret(0), 0x85d840: self.s_draw,
            0x43aaf0: lambda m: m.ret(int(m.env['vworld_ready'])), 0x76aee0: lambda m: m.ret(int(m.env['running'])),
            0x5f41f0: lambda m: m.ret(int(m.env['cinematic']), 4), 0x40eaa0: lambda m: m.ret(int(m.env['dev_mode'])),
            0x5c0e50: self.s_local, 0x5543f0: self.s_tile, 0x40cfa0: lambda m: m.ret(int(m.env['fog_off'])),
            0x54f3e0: self.s_visible, 0x428790: self.s_ctx_copy, 0x45c7a0: lambda m: m.ret(0),
            0x428af0: self.s_fx_complete, 0x74a130: self.s_stream_name, 0x5aa340: self.s_open_file,
            0x404ed0: self.s_log('404ed0'), 0x404f30: self.s_log('404f30'), 0x404ea0: self.s_log('404ea0'),
            0x404fc0: self.s_log('404fc0'), 0x405cb0: self.s_log('405cb0'), 0x856370: lambda m: m.ret(m.cs_allocator),
            0x4097f0: self.s_wcsicmp,
        }
        for address, fn in seams.items():
            self.seam(address, fn)
        self.build_world()
        self.build_data()

    # memory helpers -------------------------------------------------------
    def wstr(self, text):
        p = self.alloc(2 * len(text) + 2)
        self.u.mem_write(p, text.encode('utf-16le') + b'\0\0')
        return p

    def fl(self, a):
        return self.r(a)

    def ev(self, op, **kw):
        self.trace.append({'op': op, **kw})

    # seams ---------------------------------------------------------------
    def s_alloc(self, m):
        m.ret(m.alloc(max(m.arg(1), 4)))

    def s_lock_ctor(self, m):
        m.w(m.ecx(), m.lock_vt)
        m.ret(m.ecx())

    def s_timer_start(self, m):
        m.w(m.ecx(), m.now & 0xffffffff)
        m.w(m.ecx() + 4, 0)
        m.ret(m.now)

    def s_timer_elapsed(self, m):
        m.ret((m.now - m.r(m.ecx())) & 0xffffffff)

    def s_decrement(self, m):
        p = m.arg(1)
        v = (m.r(p) - 1) & 0xffffffff
        m.w(p, v)
        m.ret(v, 4)

    def s_increment(self, m):
        p = m.arg(1)
        v = (m.r(p) + 1) & 0xffffffff
        m.w(p, v)
        m.ret(v, 4)

    def var_name(self, p):
        b = bytes(self.u.mem_read(p, 128))
        return b.decode('utf-16le').split('\0')[0]

    def s_var_record(self, m):
        name = m.var_name(m.arg(1))
        assert name in m.vars, name
        m.ret(m.vars[name][0], 8)

    def s_var_slot(self, m):
        name = m.var_name(m.r(m.arg(1)))
        assert name in m.vars, name
        m.ret(m.vars[name][1], 4)

    def s_debug_ids(self, m):
        assert m.var_name(m.arg(1)) == 'AudioDebugIDS'
        debug = m.env.get('debug_ids')
        if not debug:
            m.ret(0, 8)
            return
        record = m.alloc(0x40)
        s = m.alloc(0x100)
        m.w(s + 4, len(debug))
        m.u.mem_write(s + 0x10, debug.encode('utf-16le') + b'\0\0')
        m.w(record + 0x10, s + 0x10)
        m.ret(record, 8)

    def s_wcsicmp(self, m):
        a = m.var_name(m.r(m.ecx()))
        b = m.var_name(m.arg(1))
        m.ret(0 if a.lower() == b.lower() else 1, 4)

    def s_avar_float(self, m):
        name = m.var_name(m.arg(1))
        record = m.vars[name][0]
        m.ret_st0_f32(m.r(record), 4)

    def s_redist(self, m):
        out = m.arg(1)
        s = m.alloc(0x40)
        m.w(s + 0xc, 0x10000)
        m.u.mem_write(s + 0x10, 'mss'.encode('utf-16le') + b'\0\0')
        m.w(out, s + 0x10)
        m.ret(out, 8)

    def s_narrow(self, m):
        s = m.alloc(0x40)
        m.w(s + 0xc, 0x10000)
        m.u.mem_write(s + 0x10, b'mss\0')
        m.w(m.ecx(), s + 0x10)
        m.ret(m.ecx(), 4)

    def s_draw(self, m):
        assert m.arg(1) == m.ui + 0xd4, hex(m.arg(1))
        assert m.draws, 'UI random draw requested beyond the scripted values'
        v = m.draws.pop(0)
        m.draw_count += 1
        m.ev('draw', value=v)
        m.ret(v)

    def s_local(self, m):
        m.ret(m.objects.get(m.env['local'], 0) if m.env.get('local') else 0)

    def s_tile(self, m):
        x, y = from_f32(m.arg(1)), from_f32(m.arg(2))
        m.ev('fog_tile', x=m.arg(1), y=m.arg(2))
        m.ret(0 if m.env['fog_tile'] is None else 0x7777, 8)

    def s_visible(self, m):
        m.ret(int(m.env['fog_visible']), 4)

    def s_ctx_copy(self, m):
        m.ret(m.ecx(), 4)

    def s_fx_complete(self, m):
        m.ev('fx_complete', node=m.fx_names.get(m.ecx(), hex(m.ecx())))
        m.ret(0, 4)

    def s_stream_name(self, m):
        out, file = m.arg(1), m.arg(2)
        s = m.alloc(0x40)
        m.w(s + 0xc, 0x10000)
        m.w(out, s + 0x10)
        m.names[s + 0x10] = m.files[file]
        m.ret(out)

    def s_open_file(self, m):
        file = m.alloc(4)
        m.files[file] = m.resource_names[m.ecx()]
        m.ret(file)

    def s_log(self, which):
        def f(m):
            m.ev('log', sink=which)
            m.ret(0)
        return f

    def make_ail(self, name, n):
        def f(m):
            args = [m.arg(i + 1) for i in range(n)]
            r = m.ail(name, args)
            m.ret(r, 4 * n)
        return f

    def ail(self, name, args):
        if name in ('register_eos_callback', 'register_3d_eos_callback', 'register_stream_callback'):
            cb = args[1]
            rec = [args[0], CALLBACKS.get(cb, 'none' if cb == 0 else hex(cb))]
        elif name == 'open_stream':
            rec = [args[0], self.names[args[1]], args[2]]
        elif name == 'set_redist_directory':
            rec = [bytes(self.u.mem_read(args[0], 8)).split(b'\0')[0].decode()]
        elif name in ('enumerate_3d_providers', 'set_file_callbacks', 'wav_info', 'stream_ms_position'):
            rec = []
        else:
            rec = args
        self.ev('ail', name=name, args=rec)
        if name == 'open_digital_driver':
            return 0x1000
        if name == 'enumerate_3d_providers':
            cursor = self.r(args[0])
            providers = self.case['providers']
            if cursor >= len(providers):
                return 0
            handle, pname = providers[cursor]
            self.w(args[0], cursor + 1)
            self.w(args[1], handle)
            p = self.alloc(0x40)
            self.u.mem_write(p, pname.encode() + b'\0')
            self.w(args[2], p)
            return 1
        if name == 'open_3d_provider':
            return self.case.get('open_provider_result', 0)
        if name == 'open_3d_listener':
            return 0x4000
        if name == 'allocate_sample_handle':
            fails = self.case.get('sample_failures', [])
            h = self.next_handle['sample']
            self.next_handle['sample'] += 1
            return 0 if (h - 0x2000) in fails else h
        if name == 'allocate_3d_sample_handle':
            h = self.next_handle['sample3d']
            self.next_handle['sample3d'] += 1
            return h
        if name == 'open_stream':
            h = self.next_handle['stream']
            self.next_handle['stream'] += 1
            return h
        if name in ('sample_playback_rate', 'sample_3d_playback_rate'):
            return self.case.get('playback_rate', 22050)
        if name in ('sample_status', 'sample_3d_status', 'stream_status'):
            return 4
        return 0

    # world ----------------------------------------------------------------
    def build_world(self):
        c = self.case
        e = self.env
        self.w(self.camera_slot, self.camera if e['camera'] else 0)
        cam = e['camera'] or {'zoom': 1.0, 'eye': [0, 0, 0], 'center': [0, 0, 0]}
        self.w(self.camera + 0x208, f32(cam['zoom']))
        for i in range(3):
            self.w(self.camera + 0x240 + 4 * i, f32(cam['eye'][i]))
            self.w(self.camera + 0x24c + 4 * i, f32(cam['center'][i]))
        self.w(self.game + 0x20, f32(e['speech_gate'][0]))
        self.w(self.svar_cell, f32(e['speech_gate'][1]))
        self.w(0xb396c0, e['application_kind'])
        self.w(0xb396c4, 2 if e['active'] else 0)
        self.thread = 1 if e['view_thread'] else 2
        # Objects: voice owners first, then game objects.
        self.objects, self.handles_of = {}, {}
        self.voice_owner_objects = {}
        for name in c.get('voice_owners', []):
            o = self.alloc(0x600)
            self.voice_owner_objects[name] = o
            self.objects[name] = o
        for name, spec in c.get('objects', {}).items():
            o = self.alloc(0x200)
            vt = self.alloc(0x200)
            self.w(o, vt)
            self.objects[name] = o
            self.w(vt + 0x108, self.stub(self.make_owner(name)))
            self.w(vt + 0x144, self.stub(self.make_relation(name)))
            index, serial = spec['handle'] & 0xffff, spec['handle'] >> 16
            self.w(o + 0x14, spec['handle'])
            self.u.mem_write(self.table + 4 + index * 2, struct.pack('<H', serial))
            self.w(self.table + 0x20004 + index * 4, o)
            self.handles_of[name] = spec['handle']
        self.refresh_objects()

    def make_owner(self, name):
        def f(m):
            owner = m.case['objects'][name].get('owner')
            m.ret(m.objects[owner] if owner else 0)
        return f

    def make_relation(self, name):
        def f(m):
            m.ret(m.case['objects'][name]['relation'], 4)
        return f

    def refresh_objects(self):
        for name, spec in self.case.get('objects', {}).items():
            o = self.objects[name]
            live = self.env.get('alive', {}).get(name, True)
            index = spec['handle'] & 0xffff
            self.w(self.table + 0x20004 + index * 4, o if live else 0)
            pos = self.env.get('positions', {}).get(name, spec.get('position', [0, 0, 0]))
            for i in range(3):
                self.w(o + 0x20 + 4 * i, f32(pos[i]))
            voice = spec.get('voice')
            self.w(o + 4, self.voice_owner_objects[voice] if voice else 0)

    def build_data(self):
        self.resources, self.resource_names, self.files = {}, {}, {}
        self.fx_names = {}
        rvt = self.alloc(0x100)
        self.w(rvt + 0xc, self.stub(self.s_request))
        self.w(rvt + 0x14, self.stub(self.s_release))
        self.w(rvt + 0x18, self.stub(self.s_sync))
        for name, spec in self.case['resources'].items():
            r = self.alloc(0x60)
            self.w(r, rvt)
            self.w(r + 0x3c, spec.get('memory', 0))
            self.wb(r + 0x41, int(spec.get('streaming', False)))
            self.resources[name] = r
            self.resource_names[r] = name
        self.data = {}
        for name, d in self.case['data'].items():
            p = self.alloc(0x100)
            self.w(p + 8, self.wstr(d['ids']))
            self.w(p + 0x18, d['category'] & 0xffffffff)
            self.w(p + 0x1c, 2)
            self.wb(p + 0x20, d['control'])
            self.wb(p + 0x21, d['world'])
            self.wb(p + 0x22, d['who'])
            self.w(p + 0x24, f32(d['volume']))
            self.w(p + 0x28, f32(d['range'][0]))
            self.w(p + 0x2c, f32(d['range'][1]))
            self.w(p + 0x30, d['limit'])
            for off, key in [(0x34, 'leadin'), (0x40, 'files'), (0x4c, 'fallout')]:
                lst = d.get(key, [])
                arr = self.alloc(4 * max(len(lst), 1))
                for i, rname in enumerate(lst):
                    self.w(arr + 4 * i, self.resources[rname])
                self.w(p + off, arr)
                self.w(p + off + 4, len(lst))
                self.w(p + off + 8, len(lst))
            for off, key in [(0x58, 'fade'), (0x60, 'delay'), (0x68, 'pitch')]:
                a, b = d.get(key, [0, 0])
                self.w(p + off, f32(a))
                self.w(p + off + 4, f32(b))
            self.w(p + 0x70, f32(d.get('volume_adjust', 0)))
            self.w(p + 0x74, 0)
            self.w(p + 0x78, f32(d.get('ldf', 0)))
            self.w(p + 0x7c, f32(d.get('subtitle', 0)))
            self.data[name] = p

    def s_request(self, m):
        m.ev('request', resource=m.resource_names[m.ecx()], delta=m.arg(1), mode=m.arg(2))
        m.ret(0, 8)

    def s_release(self, m):
        m.ev('release', resource=m.resource_names[m.ecx()], delta=m.arg(1))
        m.ret(0, 4)

    def s_sync(self, m):
        m.ev('sync_load', resource=m.resource_names[m.ecx()])
        m.ret(0)

    # operations -----------------------------------------------------------
    def construct(self):
        self.mgr = self.alloc(MGR_SIZE)
        self.call(0x749130, ecx=self.mgr)
        assert self.r(0xb36548) == self.mgr

    def vcall(self, slot, args=(), pop=None):
        return self.call(self.r(self.r(self.mgr) + slot), ecx=self.mgr, args=args, pop=pop)

    def obj(self, name):
        return self.objects[name] if name else 0

    def op(self, step):
        kind = step['op']
        if kind == 'enable':
            self.vcall(4)
        elif kind == 'disable':
            self.vcall(0xc)
        elif kind == 'update':
            self.vcall(8)
        elif kind == 'play':
            pos = 0
            if step.get('position') is not None:
                pos = self.alloc(12)
                for i in range(3):
                    self.w(pos + 4 * i, f32(step['position'][i]))
            node = ctx = 0
            if step.get('fx'):
                node, ctx = self.alloc(0x80), self.alloc(0x80)
                self.fx_names[node] = step['fx']
            r = self.call(0x7474f0, ecx=self.mgr, args=[self.data[step['data']], int(step.get('loop', False)),
                          self.obj(step.get('a')), self.obj(step.get('b')), self.obj(step.get('owner')), pos, node, ctx])
            return r
        elif kind == 'music':
            return self.call(0x747630, ecx=self.mgr, args=[self.data[step['data']]])
        elif kind == 'speech':
            pos = 0
            if step.get('position') is not None:
                pos = self.alloc(12)
                for i in range(3):
                    self.w(pos + 4 * i, f32(step['position'][i]))
            return self.call(0x7476e0, ecx=self.mgr, args=[self.data[step['data']], self.obj(step.get('a')),
                             self.obj(step.get('b')), self.obj(step.get('owner')), pos])
        elif kind in ('eos_2d', 'eos_3d', 'eos_stream'):
            self.call({'eos_2d': 0x749510, 'eos_3d': 0x749550, 'eos_stream': 0x749590}[kind], args=[step['handle']])
        elif kind == 'set_volume':
            self.call(0x744f90, ecx=self.mgr, args=[step['index'], f32(step['value']), f32(step['duration'])])
        elif kind == 'set_volumes':
            self.call(0x7450e0, ecx=self.mgr, args=[step['mask'], f32(step['value']), f32(step['duration'])])
        elif kind in ('pause', 'resume', 'stop'):
            self.call({'pause': 0x747150, 'resume': 0x747180, 'stop': 0x7471b0}[kind], ecx=self.mgr, args=[step['mask']])
        elif kind == 'hold':
            self.call(0x745580, ecx=self.mgr)
        elif kind == 'unhold':
            self.call(0x7455b0, ecx=self.mgr)
        elif kind == 'advance':
            self.now += step['ms']
        elif kind == 'env':
            self.env.update(step['set'])
            self.build_world_live()
        else:
            raise ValueError(kind)
        return None

    def build_world_live(self):
        e = self.env
        self.w(self.camera_slot, self.camera if e['camera'] else 0)
        if e['camera']:
            cam = e['camera']
            self.w(self.camera + 0x208, f32(cam['zoom']))
            for i in range(3):
                self.w(self.camera + 0x240 + 4 * i, f32(cam['eye'][i]))
                self.w(self.camera + 0x24c + 4 * i, f32(cam['center'][i]))
        self.w(self.game + 0x20, f32(e['speech_gate'][0]))
        self.w(self.svar_cell, f32(e['speech_gate'][1]))
        self.w(0xb396c4, 2 if e['active'] else 0)
        self.w(0xb396c0, e['application_kind'])
        self.thread = 1 if e['view_thread'] else 2
        self.refresh_objects()

    # state ----------------------------------------------------------------
    def walk(self, head_off):
        out, n = [], self.r(self.mgr + head_off)
        while n:
            out.append(self.r(n))
            n = self.r(n + 4)
        return out

    def channel_id(self, ch):
        if not ch:
            return None
        return [self.r(ch + 4), self.r(ch + 8), self.r(ch + 0xc), self.r(ch + 0x10), self.r(ch + 0x14), self.r(ch + 0x18) if self.r(ch + 4) == 2 else 0]

    def res(self, p):
        return self.resource_names.get(p) if p else None

    def instance(self, p):
        r = self.r
        return {
            'id': r(p + 0xc), 'data': [k for k, v in self.data.items() if v == r(p + 4)][0],
            'interrupts_10': r(p + 0x10), 'claimed_by_14': r(p + 0x14), 'channel': self.channel_id(r(p + 0x18)),
            'mode_1c': r(p + 0x1c), 'object_20': r(p + 0x20), 'owner_24': r(p + 0x24),
            'position': [r(p + 0x28), r(p + 0x2c), r(p + 0x30)], 'state_34': r(p + 0x34),
            'loop_38': self.rb(p + 0x38), 'released_39': self.rb(p + 0x39),
            'current': self.res(r(p + 0x3c)), 'leadin': self.res(r(p + 0x40)), 'fallout': self.res(r(p + 0x44)),
            'main': self.res(r(p + 0x48)), 'start_4c': r(p + 0x4c), 'fx': self.fx_names.get(r(p + 0x54)),
            'voice_5c': [k for k, v in self.voice_owner_objects.items() if v == r(p + 0x5c)][0] if r(p + 0x5c) else None,
            'fade_timer': r(r(p + 0x60)) if r(p + 0x60) else None, 'fade_64': r(p + 0x64),
            'delay_68': r(p + 0x68), 'pitch_6c': r(p + 0x6c), 'volume_70': r(p + 0x70),
        }

    def state(self):
        r, m = self.r, self.mgr
        lists = [self.walk(o) for o in (0x20, 0x28, 0x30)]
        queue, n = [], r(m + 0x40)
        while n:
            queue.append([r(n), r(r(n + 4) + 0xc) if r(n + 4) else 0, r(n + 8), r(n + 0xc), r(n + 0x10)])
            n = r(n + 0x14)
        pending = []
        n = r(m + 0x40)
        while n:
            if r(n + 4):
                pending.append(r(n + 4))
            n = r(n + 0x14)
        groups = []
        for i in range(5):
            g = m + 0xa0 + 0x28 * i
            groups.append([r(g), r(g + 4), r(g + 8), r(g + 0xc), r(g + 0x10), r(g + 0x18), self.rb(g + 0x20)])
        free = [[self.channel_id(c) for c in self.walk(o)] for o in (0x3ac, 0x3b4, 0x3bc)]
        deferred = []
        n = r(m + 0x40c)
        while n:
            deferred.append(r(n))
            n = r(n + 4)
        insts = {}
        for p in [p for l in lists for p in l] + pending:
            insts[str(r(p + 0xc))] = self.instance(p)
        return {
            'lists': [[r(p + 0xc) for p in l] for l in lists], 'music_38': r(m + 0x38), 'speech_3c': r(m + 0x3c),
            'queue': queue, 'next_id_9c': r(m + 0x9c), 'groups': groups, 'recover_168': r(m + 0x168),
            'mic': [r(m + 0x170 + 4 * i) for i in range(3)], 'orient': [r(m + 0x17c + 4 * i) for i in range(3)],
            'flags': [self.rb(m + 0x188 + i) for i in range(5)], 'hold_190': r(m + 0x190),
            'initialized': self.rb(m + 0x198), 'fast_2d': r(m + 0x19c), 'open': r(m + 0x1a0),
            'driver': r(m + 0x3a8), 'listener': r(m + 0x3d0), 'free': free,
            'available': [r(m + 0x3c4), r(m + 0x3c8), r(m + 0x3cc)], 'deferred': deferred,
            'voices': {k: self.rb(v + 0x5bc) for k, v in self.voice_owner_objects.items()},
            'instances': insts, 'guard': r(0xb28a40),
        }


def env_default():
    return {'vworld_ready': True, 'running': True, 'cinematic': False, 'dev_mode': False, 'local': None,
            'fog_tile': 1, 'fog_off': False, 'fog_visible': True, 'camera': {'zoom': 1.0, 'eye': [0, -10, 20], 'center': [0, 0, 0]},
            'speech_gate': [0.0, 0.0], 'application_kind': 1, 'active': True, 'view_thread': True, 'positions': {},
            'alive': {}}


def base_case(**kw):
    c = {
        'providers': [[0x601, 'Miles Fast 2D Positional Audio']],
        'user_volumes': {'8f': 0.8, '90': 0.8, '91': 0.8, '92': 0.95, '93': 0.7},
        'vars': {
            'Miles2DChannels': ('i', 2), 'Miles3DChannels': ('i', 2), 'MilesStreamingChannels': ('i', 2),
            'MilesFrequency': ('i', 44100), 'MilesBits': ('i', 16), 'MilesHWChannels': ('i', 2),
            'AudioLatencyNormalMS': ('i', 64), 'AudioLatencyHighMS': ('i', 200),
            'AudioMicrophoneHeight': ('f', 5.0), 'AudioConsiderCenteredRadius': ('f', 12.0),
            'AudioFadeTime': ('f', 1.0), 'AudioFadeVolume': ('f', 0.5), 'AudioDropTimeout': ('f', 5.0),
            'AudioRecoverFromFadeTime': ('f', 2.0),
        },
        'resources': {f'r{i}': {'memory': 0x7000 + i} for i in range(8)},
        'data': {},
        'objects': {},
        'voice_owners': [],
        'env': env_default(),
        'draws': [],
        'steps': [],
    }
    c['resources'].update({f's{i}': {'streaming': True} for i in range(4)})
    c.update(kw)
    return c


def snd(ids, **kw):
    d = {'ids': ids, 'category': 0, 'control': 0, 'world': 0, 'who': 0x21, 'volume': 0.8, 'range': [4.0, 64.0],
         'limit': 2, 'files': ['r0']}
    d.update(kw)
    return d


def cases():
    out = []
    rnd = random.Random(0xb36548)
    def draws(n=64):
        return [rnd.getrandbits(32) for _ in range(n)]
    # 1. 2-D sound life cycle with leadin/main/fallout and randomization.
    c = base_case(data={'ui': snd('ui_click', leadin=['r1', 'r2', 'r3'], files=['r0', 'r4'], fallout=['r5', 'r6'],
                                  delay=[0, 30], pitch=[-0.05, 0.05], volume_adjust=-0.2)}, draws=draws())
    c['steps'] = [{'op': 'enable'}, {'op': 'update'}, {'op': 'play', 'data': 'ui'}, {'op': 'advance', 'ms': 16},
                  {'op': 'update'}, {'op': 'eos_2d', 'handle': 0x2000}, {'op': 'eos_2d', 'handle': 0x2000},
                  {'op': 'eos_2d', 'handle': 0x2000}, {'op': 'advance', 'ms': 16}, {'op': 'update'}]
    out.append(('2d_life_cycle', c))
    # 2. Looping music, fade in, fade out by id (FX StopSFX path uses ids).
    c = base_case(data={'music': snd('usa_ambient', category=1, files=['s0', 's1', 's2'], fade=[3.0, 1.0],
                                     volume=1.33, limit=2)}, draws=draws())
    steps = [{'op': 'enable'}, {'op': 'music', 'data': 'music'}]
    for i in range(6):
        steps += [{'op': 'advance', 'ms': 700}, {'op': 'update'}]
    steps += [{'op': 'eos_stream', 'handle': 0x5000}, {'op': 'advance', 'ms': 50}, {'op': 'update'},
              {'op': 'stop', 'mask': 2}, {'op': 'update'}, {'op': 'advance', 'ms': 50}, {'op': 'update'}]
    c['steps'] = steps
    out.append(('music_loop', c))
    # 3. 3-D world sound on an object with the centered radius and movement.
    objs = {'tank': {'handle': 0x10005, 'position': [30, 40, 0], 'relation': 0, 'voice': 'company', 'owner': 'king'},
            'king': {'handle': 0x20006, 'relation': 0}}
    c = base_case(data={'gun': snd('tank_fire', control=0x10, who=0x1e, range=[4.0, 64.0], files=['r0', 'r1'])},
                  objects=objs, voice_owners=['company'], draws=draws())
    c['env']['local'] = 'king'
    c['steps'] = [{'op': 'enable'}, {'op': 'update'}, {'op': 'play', 'data': 'gun', 'a': 'tank'}, {'op': 'update'},
                  {'op': 'env', 'set': {'positions': {'tank': [3, 4, 0]}}}, {'op': 'advance', 'ms': 20}, {'op': 'update'},
                  {'op': 'env', 'set': {'camera': {'zoom': 0.5, 'eye': [5, -20, 30], 'center': [10, 10, 0]}}},
                  {'op': 'update'}, {'op': 'env', 'set': {'alive': {'tank': False}}}, {'op': 'update'},
                  {'op': 'update'}]
    out.append(('3d_object', c))
    # 4. Out of range, location sounds and relations.
    for rel in range(5):
        objs = {'unit': {'handle': 0x10008, 'position': [10, 0, 0], 'relation': rel, 'owner': 'king'},
                'king': {'handle': 0x20009, 'relation': rel}}
        c = base_case(data={'resp': snd('unit_resp', who=0x02, files=['r0']),
                            'far': snd('far_boom', control=0x10, who=0x20, range=[4.0, 64.0]),
                            'glob': snd('global_boom', control=0x10, world=1, who=0x20)},
                      objects=objs, draws=draws())
        c['env']['local'] = 'king' if rel != 4 else None
        c['steps'] = [{'op': 'enable'}, {'op': 'update'}, {'op': 'play', 'data': 'resp', 'a': 'unit'},
                      {'op': 'play', 'data': 'far', 'position': [500, 500, 0]},
                      {'op': 'play', 'data': 'far', 'position': [20, 20, 0]},
                      {'op': 'play', 'data': 'glob', 'position': [900, 0, 0]}, {'op': 'update'}]
        out.append((f'relation_{rel}', c))
    # 5. Simultaneous limit and INTERRUPT, channel exhaustion and 746C00.
    c = base_case(data={'a': snd('shot_a', control=1, limit=2, files=['r0', 'r1', 'r2']),
                        'b': snd('shot_b', control=1, limit=3, files=['r3']),
                        'n': snd('shot_n', control=0, limit=1, files=['r4'])}, draws=draws())
    steps = [{'op': 'enable'}]
    for i in range(3):
        steps += [{'op': 'play', 'data': 'a'}, {'op': 'advance', 'ms': 5}, {'op': 'update'}]
    steps += [{'op': 'play', 'data': 'b'}, {'op': 'play', 'data': 'b'}, {'op': 'update'},
              {'op': 'play', 'data': 'n'}, {'op': 'play', 'data': 'n'}, {'op': 'update'}]
    c['steps'] = steps
    out.append(('limits_interrupt', c))
    # 6. VOICE ownership and the refused-voice release.
    objs = {'u1': {'handle': 0x1000a, 'position': [1, 1, 0], 'relation': 0, 'voice': 'v'},
            'u2': {'handle': 0x1000b, 'position': [1, 2, 0], 'relation': 0, 'voice': 'v'}}
    c = base_case(data={'talk': snd('unit_talk', control=8, who=0x21, limit=4, files=['r0'])}, objects=objs,
                  voice_owners=['v'], draws=draws())
    c['steps'] = [{'op': 'enable'}, {'op': 'play', 'data': 'talk', 'a': 'u1'}, {'op': 'play', 'data': 'talk', 'a': 'u2'},
                  {'op': 'update'}, {'op': 'play', 'data': 'talk', 'a': 'u2'}, {'op': 'update'},
                  {'op': 'eos_2d', 'handle': 0x2000}, {'op': 'update'}]
    out.append(('voice', c))
    # 7. CAUSEFADE speech, group fades and recovery; speech stream limit.
    c = base_case(data={'speech': snd('advisor', category=2, control=0x21, files=['s0'], limit=1),
                        'speech2': snd('advisor2', category=2, control=0x21, files=['s1'], limit=1),
                        'ui': snd('ui', files=['r0'])}, draws=draws())
    steps = [{'op': 'enable'}, {'op': 'play', 'data': 'speech'}, {'op': 'update'}, {'op': 'play', 'data': 'speech2'},
             {'op': 'play', 'data': 'ui'}, {'op': 'update'}]
    for i in range(4):
        steps += [{'op': 'advance', 'ms': 300}, {'op': 'update'}]
    steps += [{'op': 'eos_stream', 'handle': 0x5000}, {'op': 'eos_stream', 'handle': 0x5001}, {'op': 'update'}]
    for i in range(10):
        steps += [{'op': 'advance', 'ms': 400}, {'op': 'update'}]
    c['steps'] = steps
    out.append(('causefade_speech', c))
    # 8. Drop timeout, pause/resume by the game, application inactive.
    c = base_case(data={'ui': snd('ui', files=['r0', 'r1']), 'w': snd('world', control=0x10, who=0x21)},
                  draws=draws())
    c['steps'] = [{'op': 'enable'}, {'op': 'play', 'data': 'ui'}, {'op': 'advance', 'ms': 6000}, {'op': 'update'},
                  {'op': 'play', 'data': 'ui'}, {'op': 'play', 'data': 'w', 'position': [1, 1, 0]},
                  {'op': 'advance', 'ms': 4000}, {'op': 'update'},
                  {'op': 'env', 'set': {'running': False}}, {'op': 'update'}, {'op': 'play', 'data': 'w', 'position': [2, 1, 0]},
                  {'op': 'update'}, {'op': 'env', 'set': {'running': True, 'active': False}}, {'op': 'update'},
                  {'op': 'env', 'set': {'vworld_ready': False}}, {'op': 'update'}]
    out.append(('drop_pause', c))
    # 9. Volume writes, user music notification, hold/latency.
    c = base_case(data={'ui': snd('ui', files=['r0'])}, draws=draws())
    c['steps'] = [{'op': 'enable'}, {'op': 'set_volume', 'index': 2, 'value': 0.4, 'duration': 0},
                  {'op': 'set_volume', 'index': 0x43, 'value': 0.25, 'duration': 0},
                  {'op': 'set_volume', 'index': 0x81, 'value': 0.1, 'duration': 500},
                  {'op': 'set_volumes', 'mask': 0x95, 'value': 0.3, 'duration': 0},
                  {'op': 'set_volumes', 'mask': 0x4a, 'value': 0.6, 'duration': 0},
                  {'op': 'play', 'data': 'ui'}, {'op': 'hold'}, {'op': 'update'}, {'op': 'advance', 'ms': 120},
                  {'op': 'update'}, {'op': 'advance', 'ms': 500}, {'op': 'unhold'}, {'op': 'update'},
                  {'op': 'pause', 'mask': 1}, {'op': 'update'}, {'op': 'resume', 'mask': 1}, {'op': 'update'}]
    out.append(('volumes_hold', c))
    # 10. FOGGED world sound, SEQUENTIAL and ALL file selection.
    c = base_case(data={'fog': snd('fogged', control=0x10, world=2, who=0x21, files=['r0']),
                        'seq': snd('seq', control=2, files=['r1', 'r2', 'r1', 'r3'], limit=8),
                        'all': snd('all', control=4, files=['r4', 'r5'], limit=8)}, draws=draws())
    c['steps'] = [{'op': 'enable'}, {'op': 'play', 'data': 'fog', 'position': [3, 3, 0]}, {'op': 'update'},
                  {'op': 'env', 'set': {'fog_visible': False}}, {'op': 'update'},
                  {'op': 'env', 'set': {'fog_tile': None}}, {'op': 'update'}]
    for i in range(5):
        c['steps'] += [{'op': 'play', 'data': 'seq'}, {'op': 'update'}, {'op': 'eos_2d', 'handle': 0x2000 + (i % 2)}]
    c['steps'] += [{'op': 'play', 'data': 'all'}, {'op': 'update'}]
    out.append(('fog_sequential', c))
    # 11. Disable/enable (shutdown keeps +1A0) and FX completion on removal.
    c = base_case(data={'ui': snd('ui', files=['r0'], fade=[0, 0.5]), 'm': snd('m', category=1, files=['s0'], fade=[0, 1.0])},
                  draws=draws())
    c['steps'] = [{'op': 'enable'}, {'op': 'play', 'data': 'ui', 'fx': 'node_a'}, {'op': 'music', 'data': 'm'},
                  {'op': 'update'}, {'op': 'disable'}, {'op': 'update'}, {'op': 'enable'}, {'op': 'play', 'data': 'ui', 'fx': 'node_b'},
                  {'op': 'update'}, {'op': 'eos_2d', 'handle': 0x2002}, {'op': 'env', 'set': {'view_thread': False}},
                  {'op': 'update'}]
    out.append(('disable_enable', c))
    # 12. Random sequences over a mixed library.
    for k in range(6):
        objs = {f'o{i}': {'handle': 0x10010 + i, 'position': [rnd.uniform(-30, 30), rnd.uniform(-30, 30), 0],
                          'relation': rnd.choice([0, 1, 3]), 'voice': rnd.choice(['va', 'vb'])} for i in range(4)}
        data = {
            'd0': snd('rnd0', control=1, limit=2, files=['r0', 'r1', 'r2']),
            'd1': snd('rnd1', control=0x11, who=0x02, files=['r3', 'r4'], range=[2.0, 40.0]),
            'd2': snd('rnd2', control=0x19, who=0x1e, files=['r5'], limit=3),
            'd3': snd('rnd3', category=2, control=0x21, files=['s0', 's1']),
            'd4': snd('rnd4', category=1, files=['s2', 's3'], fade=[1.0, 0.5]),
        }
        c = base_case(data=data, objects=objs, voice_owners=['va', 'vb'], draws=draws(400))
        c['vars']['Miles2DChannels'] = ('i', 3)
        c['vars']['Miles3DChannels'] = ('i', 3)
        steps = [{'op': 'enable'}, {'op': 'update'}]
        for _ in range(40):
            r = rnd.random()
            if r < 0.35:
                name = rnd.choice(['d0', 'd1', 'd2', 'd3'])
                st = {'op': 'play', 'data': name}
                if name in ('d1', 'd2'):
                    st['a'] = rnd.choice(list(objs))
                steps.append(st)
            elif r < 0.4:
                steps.append({'op': 'music', 'data': 'd4'})
            elif r < 0.55:
                steps.append({'op': rnd.choice(['eos_2d', 'eos_3d', 'eos_stream']),
                              'handle': rnd.choice([0x2000, 0x2001, 0x2002, 0x3000, 0x3001, 0x5000, 0x5001])})
            elif r < 0.6:
                steps.append({'op': 'stop', 'mask': rnd.choice([1, 2, 4, 7])})
            elif r < 0.65:
                steps.append({'op': 'env', 'set': {'positions': {o: [rnd.uniform(-40, 40), rnd.uniform(-40, 40), 0] for o in objs}}})
            else:
                steps += [{'op': 'advance', 'ms': rnd.choice([16, 33, 250, 900])}, {'op': 'update'}]
        steps.append({'op': 'update'})
        c['steps'] = steps
        out.append((f'random_{k}', c))
    return out


def run(precision, name, case):
    m = AudioMachine(precision, case)
    m.construct()
    steps = []
    steps.append({'state': m.state(), 'trace': m.trace, 'result': None})
    m.trace = []
    for step in case['steps']:
        result = m.op(step)
        steps.append({'state': m.state(), 'trace': m.trace, 'result': result})
        m.trace = []
    m.verify_v14()
    return m, steps


def main():
    rows, visited = [], {}
    for precision in ('Single24', 'Double53', 'Extended64'):
        for name, case in cases():
            m, steps = run(precision, name, case)
            visited.update(m.visited)
            rows.append({'name': name, 'precision': precision, 'case': case, 'steps': steps})
    d = header(__file__, __doc__)
    d.update(instructions=instructions(visited), rows=rows)
    p = HERE / 'native-audio-manager.json'
    p.write_text(json.dumps(d) + '\n', encoding='utf-8')
    body = [a for a in visited if 0x744e90 <= a < 0x74ad70 + 0x1cb]
    print(json.dumps({'rows': len(rows), 'audio_instructions': len(body), 'instructions': len(visited),
                      'sha256': sha(p.read_bytes())}))


if __name__ == '__main__':
    main()
