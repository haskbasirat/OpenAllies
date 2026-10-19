"""Original/v14 Resource release, ResourceCache admission/removal and unload.

Executed native code (both images, Single24/Double53/Extended64):
  5AA550 Resource::Release, 5AEE70 KKC_ResourceManager::ReleaseResource,
  5AED90 RemoveFromTaskQueue, 5B1970 task abort, 5ABB30/5ABB50,
  5AC3D0 CacheOrUnloadResource, 59BF30, 59BC70 EnsureEnoughAvailableMemory,
  59B9F0, 59B8C0, 59BA90, 59BFA0, 5ABFB0, 5B0270 (cached branch), 5ABFF0,
  the class vf0/vf2C/vf40/vf54 bodies reached through the ACTUAL vtables
  (Texture A454A8: 5AA700/5A7C80, Model A45C98: 5A6560/5A7CF0, Sequence
  A45290: 5A6530/5A7CF0), 5A5030, 5B17E0, 58CB30, 5AA290.

Named seams (recording only unless stated):
  lock objects at Manager+4/+20/+3C, Cache+4, renderer+18 (vf4/vf8 and the
  857E10/857DE0 direct entries); IAT InterlockedIncrement/Decrement (perform
  the interlocked op), GetCurrentThreadId, TlsGetValue; 404F60/404F90 logs;
  5AE0E0 queue-list removal (returns the case's result); 59DA00 (5ABB90's
  out-of-cache-memory callback, returns the case's result); 73D4B0; 59DAB0;
  85C2B0; 59D430/5A75D0 deferred releases (record, then store the field NULL
  exactly as 59D4D2/5A7672 do); a location object's vf0C (case value);
  NiObject vf0 deleting destructor. Optional 'hook_loaded' stores bit0 at a
  named address to model the other thread finishing the load.
"""
from pathlib import Path
import hashlib, json, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402
from unicorn import UC_HOOK_MEM_READ  # noqa: E402
from unicorn.x86_const import UC_X86_REG_EAX  # noqa: E402

VT = {'Texture': 0xa454a8, 'Model': 0xa45c98, 'Sequence': 0xa45290, 'Sound': 0xa42c38}
DESCRIPTOR = {'Texture': 0xb3d97c, 'Model': 0xb3d990, 'Sequence': 0xb3d9a4, 'Sound': 0xb3d76c}
CLASS_ID = {'Texture': 0x11, 'Model': 0x13, 'Sequence': 0x14, 'Sound': 0x0c}
TASK_VT = 0xa47790
CUR_TID = 0x1234
IAT = {0x9f01e8: 'InterlockedIncrement', 0x9f01ec: 'InterlockedDecrement',
       0x9f01e4: 'GetCurrentThreadId', 0x9f020c: 'TlsGetValue'}


def wide(read, address):
    out = []
    while True:
        v = struct.unpack('<H', read(address + 2 * len(out), 2))[0]
        if v == 0:
            return ''.join(map(chr, out))
        out.append(v)


class Machine(h.Machine):
    def __init__(self, pc, image):
        super().__init__(pc)
        if image is not h.P:
            original_end = (h.P.image_size + 4095) & ~4095
            image_end = (image.image_size + 4095) & ~4095
            if image_end > original_end:
                self.u.mem_map(image.base + original_end, image_end - original_end)
            self.u.mem_write(image.base, image.b[:4096])
            for vs, r, raw, off in image.sections:
                if raw:
                    self.u.mem_write(image.base + r, image.b[off:off + min(raw, max(vs, raw))])
        self.events = []
        self.locks = {}
        self.labels = {}
        self.stub_seq = h.STUBS + 0x400
        self.watch = {}
        self.u.hook_add(UC_HOOK_MEM_READ, self.read_watch)
        for kind, d in DESCRIPTOR.items():
            self.w32(d, CLASS_ID[kind])
        for slot, name in IAT.items():
            stub = self.stub(getattr(self, 'iat_' + name))
            self.w32(slot, stub)
        self.seams[0x857e10] = lambda _: self.lock_event('lock', self.ecx())
        self.seams[0x857de0] = lambda _: self.lock_event('unlock', self.ecx())
        self.seams[0x404f60] = lambda _: self.log(10)
        self.seams[0x404f90] = lambda _: self.log(20)

    # ---- generic helpers ---------------------------------------------
    def stub(self, handler):
        address = self.stub_seq
        self.stub_seq += 16
        self.seams[address] = handler
        return address

    def text(self, value):
        units = list(map(ord, value))
        data = self.alloc(16 + 2 * (len(units) + 1))
        self.w32(data, 0, len(units), len(units), 1)
        self.u.mem_write(data + 16, struct.pack('<' + 'H' * (len(units) + 1), *units, 0))
        return data + 16

    def wide(self, p):
        return wide(lambda a, n: bytes(self.u.mem_read(a, n)), p)

    def read_watch(self, u, access, a, size, value, _):
        name = self.watch.get(a)
        if name:
            self.events.append([name])

    def lock_object(self, address, label):
        vt = self.alloc(12)
        self.w32(vt + 4, self.stub(lambda _: self.lock_event('lock', self.ecx())))
        self.w32(vt + 8, self.stub(lambda _: self.lock_event('unlock', self.ecx())))
        self.w32(address, vt)
        self.locks[address] = label

    def lock_event(self, kind, address):
        self.events.append([kind, self.locks[address]])
        self.ret()

    def log(self, level):
        sp = self.esp()
        channel, file, line, function, fmt = self.r32(sp + 4, 5)
        fmt_text = self.wide(fmt)
        args = []
        offset = sp + 24
        i = 0
        while i < len(fmt_text):
            if fmt_text[i] == '%' and i + 1 < len(fmt_text):
                spec = fmt_text[i + 1]
                value = self.r32(offset)
                offset += 4
                args.append(self.wide(value) if spec == 's' else value)
                i += 2
            else:
                i += 1
        self.events.append(['log', level, hex(channel), self.wide(file), line, self.wide(function), fmt_text] + args)
        self.ret()

    def iat_InterlockedIncrement(self, _):
        p = self.arg()
        v = (self.r32(p) + 1) & 0xffffffff
        self.w32(p, v)
        self.ret(4, v)

    def iat_InterlockedDecrement(self, _):
        p = self.arg()
        v = (self.r32(p) - 1) & 0xffffffff
        self.w32(p, v)
        self.ret(4, v)

    def iat_GetCurrentThreadId(self, _):
        self.ret(0, CUR_TID)

    def iat_TlsGetValue(self, _):
        assert self.arg() == self.tls_index
        self.ret(4, self.thread_object)

    # ---- world construction --------------------------------------------
    def build(self, case):
        m = self.manager = self.alloc(0x200)
        self.w32(m + 0xe4, self.alloc(0x40))  # 5ABB90 reads +E4 for 59DA00
        self.w32(0xb36540, m)
        self.w32(0xb36510, 0)
        for off, label in ((4, '04'), (0x20, '20'), (0x3c, '3c')):
            self.lock_object(m + off, label)
        c = self.cache = m + 0x9c
        self.lock_object(c + 4, 'cache')
        buckets = self.alloc(3 * 4)
        self.w32(c + 0x20, buckets, 3)
        self.w32(c + 0x30, case.get('ram_total', 0))
        self.w32(c + 0x3c, case.get('vram_total', 0))
        # UVar table: [B39940]+D8 -> ResourceCaching Bool cell.
        table = self.alloc(0x100)
        cell = self.alloc(4)
        self.w8(cell, case.get('caching', 1))
        self.w32(table + 0xd8, cell)
        self.w32(0xb39940, table)
        self.watch[cell] = 'caching54'
        # Process deferred-release owner created by 70EF50 (B401AC).
        self.w32(0xb401ac, self.alloc(0x60) if case.get('deferred', True) else 0)
        if not case.get('deferred', True):
            renderer = self.alloc(0x260)
            self.w32(0xb36510, renderer)
            self.locks[renderer + 0x18] = 'renderer'
        # TLS current thread object for 5A7CF0's "[%s]" name.
        self.tls_index = 7
        self.thread_object = self.alloc(0x60)
        self.w32(self.thread_object + 0x50, self.text('Main'))
        self.w8(0xb43bbc, 1)
        self.w32(0xb43bc0, self.tls_index)
        self.classes = {}
        for kind, row in case.get('classes', {}).items():
            e = self.alloc(0x34)
            self.w32(e + 4, CLASS_ID[kind])
            self.w32(e + 8, row.get('ram_cap', 0xffffffff), row.get('ram_used', 0), row.get('ram_peak', 0), 0,
                     row.get('vram_cap', 0xffffffff), row.get('vram_used', 0), row.get('vram_peak', 0), 0, c)
            node = self.alloc(12)
            bucket = CLASS_ID[kind] % 3
            self.w32(node, CLASS_ID[kind], e, self.r32(buckets + bucket * 4))
            self.w32(buckets + bucket * 4, node)
            self.classes[kind] = e
        self.resources = []
        for index, r in enumerate(case['resources']):
            p = self.alloc(0xb4)
            self.w32(p, VT[r['class']], r.get('count', 0), r.get('flags', 0), r.get('ram', 0), r.get('vram', 0))
            self.w32(p + 0x38, self.text(r.get('name', f"{r['class']}{index}")))
            self.labels[p] = index
            task = self.alloc(0x40)
            self.w32(task, TASK_VT, 1, 1, 0, 0, r.get('state', 0))
            self.w32(task + 0x1c, p)
            thread = self.alloc(0x60)
            self.w32(thread + 0x48, r.get('task_thread', CUR_TID + 1))
            self.w32(task + 0x18, thread)
            self.w32(p + 0x20, task)
            if r['class'] in ('Model', 'Sequence'):
                if r.get('ni3c'):
                    self.w32(p + 0x3c, self.alloc(0x30))
                if r.get('root40'):
                    self.w32(p + 0x40, self.ni_object(r.get('root_refs', 1)))
            if r['class'] == 'Texture':
                if r.get('source3c'):
                    self.w32(p + 0x3c, self.ni_object(r.get('source_refs', 1)))
                if r.get('child40'):
                    self.w32(p + 0x40, self.alloc(0x40))
                if 'location_writable' in r:
                    loc = self.alloc(8)
                    lvt = self.alloc(0x10)
                    writable = r['location_writable']
                    self.w32(lvt + 0xc, self.stub(lambda _, w=writable: self.ret(0, 1 if w else 0)))
                    self.w32(loc, lvt)
                    arr = self.alloc(4)
                    self.w32(arr, loc)
                    self.w32(p + 0x24, arr, 1)
            self.resources.append(p)
        for kind, members in case.get('lru', {}).items():
            e = self.classes[kind]
            previous = 0
            for index in members:
                p = self.resources[index]
                if previous:
                    self.w32(previous + 0x1c, p)
                    self.w32(p + 0x18, previous)
                else:
                    self.w32(e + 0x2c, p)
                previous = p
            self.w32(e + 0x30, previous)
        # Seams of reached leaves.
        self.seams[0x5ae0e0] = self.remove_queue
        self.seams[0x59da00] = self.out_of_memory
        self.seams[0x73d4b0] = lambda _: (self.events.append(['detach_73d4b0', self.label_of_field(self.r32(self.esp() + 4))]), self.ret())
        self.seams[0x59dab0] = lambda _: (self.events.append(['ni_entry_teardown_59dab0']), self.ret())
        self.seams[0x85c2b0] = lambda _: (self.events.append(['pool_free_85c2b0']), self.ret(4))
        self.seams[0x59d430] = lambda _: self.deferred('deferred_59d430')
        self.seams[0x5a75d0] = lambda _: self.deferred('deferred_5a75d0')
        self.seams[0x6fb190] = self.sleep
        for address, flag in case.get('hook_loaded', {}).items():
            self.seams[int(address, 16)] = lambda _, i=flag: self.set_loaded(i)

    def ni_object(self, refs):
        obj = self.alloc(0x20)
        vt = self.alloc(0x10)
        self.w32(vt, self.stub(lambda _: (self.events.append(['ni_delete']), self.ret(4))))
        self.w32(obj, vt, refs)
        return obj

    def label_of_field(self, value):
        return value != 0

    def set_loaded(self, index):
        p = self.resources[index]
        self.w32(p + 8, self.r32(p + 8) | 1)
        self.events.append(['other_thread_loaded'])

    def remove_queue(self, _):
        self.events.append(['remove_queue_5ae0e0'])
        self.ret(4, 1 if self.case.get('in_queue') else 0)

    def out_of_memory(self, _):
        self.events.append(['out_of_memory_59da00'])
        self.ret(0xc, self.case.get('oom_result', 0))

    def sleep(self, _):
        self.events.append(['sleep_6fb190'])
        task = self.r32(self.resources[0] + 0x20)
        self.w32(task + 0x14, 5)  # the other thread's dispatcher observed state4
        self.ret(4)

    def deferred(self, name):
        field = self.arg()
        self.events.append([name, self.r32(field) != 0])
        self.w32(field, 0)
        self.ret(4)

    def snapshot(self):
        out = dict(resources=[], classes={}, totals=[self.r32(self.cache + 0x30), self.r32(self.cache + 0x3c)])
        for p in self.resources:
            task = self.r32(p + 0x20)
            prev, nxt = self.r32(p + 0x18), self.r32(p + 0x1c)
            out['resources'].append(dict(count=self.r32(p + 4), flags=self.r32(p + 8),
                                         prev=self.labels.get(prev), next=self.labels.get(nxt),
                                         field3c=self.r32(p + 0x3c) != 0, field40=self.r32(p + 0x40) != 0,
                                         task_refs=self.r32(task + 4), task_queue=self.r32(task + 0x10), task_state=self.r32(task + 0x14)))
        for kind, e in self.classes.items():
            lru = []
            p = self.r32(e + 0x2c)
            while p:
                lru.append(self.labels[p])
                p = self.r32(p + 0x1c)
            out['classes'][kind] = dict(ram_used=self.r32(e + 0xc), ram_peak=self.r32(e + 0x10),
                                        vram_used=self.r32(e + 0x1c), vram_peak=self.r32(e + 0x20),
                                        lru=lru, tail=self.labels.get(self.r32(e + 0x30)))
        return out

    def run(self, case):
        self.case = case
        self.build(case)
        target = self.resources[0]
        op = case['op']
        if op == 'release':
            self.call(0x5aa550, target, [case['delta']])
        elif op == 'rerequest':
            self.call(0x5b0270, self.manager, [target, case.get('mode', 1)])
        elif op == 'unload':
            self.call(0x5abff0, self.manager, [target])
        elif op == 'vf40':
            self.call(self.r32(VT[case['resources'][0]['class']] + 0x40), target)
        elif op == 'vf54':
            self.call(self.r32(VT[case['resources'][0]['class']] + 0x54), target)
        else:
            raise ValueError(op)
        return dict(events=self.events, state=self.snapshot())


def cases():
    loaded_model = dict(**{'class': 'Model'}, flags=0x447, ram=0x1234, vram=0x200, ni3c=True, root40=True)
    tex = {'class': 'Texture'}
    classes = {'Model': dict(ram_used=0x100, ram_peak=0x2000, vram_used=0x80, vram_peak=0x100), 'Texture': {}}
    # 1. 5AA550 count/delta edges; loaded Model reaching zero is cached.
    for delta in (1, 2, 0):
        for count in (0, 1, 2, 3, 0x80000000):
            yield dict(op='release', delta=delta, classes=classes,
                       resources=[dict(loaded_model, count=count)])
    # 2. Cache admission order/accounting, existing LRU of the same class.
    for lru in ([], [1], [1, 2]):
        yield dict(op='release', delta=1, classes=classes, lru={'Model': lru}, ram_total=0x500, vram_total=0x90,
                   resources=[dict(loaded_model, count=1), dict(loaded_model, flags=0x467, ram=7, vram=9),
                              dict(loaded_model, flags=0x467, ram=1, vram=2)])
    yield dict(op='release', delta=1, classes={'Model': dict(ram_used=0xffffff00, ram_peak=0xffffff10, vram_used=5, vram_peak=1)},
               resources=[dict(loaded_model, count=1)])
    yield dict(op='release', delta=1, classes={'Texture': {}},
               resources=[dict(tex, count=1, flags=0x200007, ram=0x40000, vram=0x40000, source3c=True)])
    # 3. Immediate unload: ResourceCaching false, or flags bit3 (cell unread).
    yield dict(op='release', delta=1, caching=0, classes=classes, resources=[dict(loaded_model, count=1)])
    yield dict(op='release', delta=1, caching=1, classes=classes, resources=[dict(loaded_model, count=1, flags=0x44f)])
    yield dict(op='release', delta=1, caching=0, classes={'Texture': {}},
               resources=[dict(tex, count=1, flags=0x200007, source3c=True, location_writable=False)])
    # 4. Not loaded: queued, popped, running on this thread, finishing late.
    nl = dict(**{'class': 'Model'}, flags=0x410, count=1)
    yield dict(op='release', delta=1, in_queue=True, classes=classes, resources=[dict(nl)])
    for state in (0, 2, 3, 4):
        yield dict(op='release', delta=1, in_queue=False, classes=classes, resources=[dict(nl, state=state)])
    yield dict(op='release', delta=1, in_queue=False, classes=classes, resources=[dict(nl, state=1, task_thread=CUR_TID)])
    yield dict(op='release', delta=1, in_queue=False, classes=classes, resources=[dict(nl, state=1)])
    yield dict(op='release', delta=1, in_queue=False, classes=classes, hook_loaded={'0x5aeedf': 0},
               resources=[dict(nl, state=2, ram=3, vram=4)])
    yield dict(op='release', delta=1, in_queue=False, classes=classes, hook_loaded={'0x5aef1a': 0},
               resources=[dict(nl, state=0, ram=3, vram=4)])
    # 5. 5B0270 re-request of a cached entry from every list position.
    cached = dict(**{'class': 'Model'}, flags=0x467, ram=0x10, vram=0x20)
    for lru, target in (([0], 0), ([0, 1, 2], 0), ([1, 0, 2], 0), ([1, 2, 0], 0)):
        yield dict(op='rerequest', classes={'Model': dict(ram_used=0x31, vram_used=0x62, ram_peak=0x40, vram_peak=0x80)},
                   ram_total=0x31, vram_total=0x62, lru={'Model': lru},
                   resources=[dict(cached, count=1), dict(cached, ram=0x11, vram=0x22), dict(cached, ram=0x10, vram=0x20)])
    # 6. 5ABFF0 (5ACA50's eligible entry): loaded+cached and cached only.
    for flags in (0x467, 0x466, 0x20467, 0x8467):
        yield dict(op='unload', classes=classes, lru={'Model': [0]}, ram_total=0x100, vram_total=0x80,
                   resources=[dict(loaded_model, flags=flags, count=0)])
    # 7. Class unload virtuals.
    for flags in (0x447, 0x20447):
        for ni3c in (True, False):
            yield dict(op='vf40', resources=[dict(**{'class': 'Model'}, flags=flags, ni3c=ni3c, root40=True)])
    yield dict(op='vf40', resources=[dict(**{'class': 'Sequence'}, flags=0x7, ni3c=True)])
    yield dict(op='vf40', resources=[dict(tex, flags=0x200007)])
    for deferred in (True, False):
        for root in (True, False):
            for refs in (1, 2):
                yield dict(op='vf54', deferred=deferred,
                           resources=[dict(**{'class': 'Model'}, flags=0x446, count=0, root40=root, root_refs=refs, name='Units/a.nif')])
    for deferred in (True, False):
        for writable, child in ((None, False), (False, False), (True, False), (True, True), (False, True)):
            r = dict(tex, flags=0x200006, source3c=True, child40=child, name='Art/t.dds')
            if writable is not None:
                r['location_writable'] = writable
            yield dict(op='vf54', deferred=deferred, resources=[r])
    # 8. Removed cap evidence: a finite 59C040 class cap evicts the LRU head
    #    (59BC70 "Decaching") or, with an empty list, asks 59DA00.
    yield dict(op='release', delta=1, ram_total=0x30, vram_total=0x30,
               classes={'Model': dict(ram_cap=0x40, ram_used=0x30, vram_cap=0x40, vram_used=0x30, ram_peak=0x30, vram_peak=0x30)},
               lru={'Model': [1, 2]},
               resources=[dict(loaded_model, count=1, ram=0x18, vram=0x8),
                          dict(loaded_model, flags=0x467, ram=0x10, vram=0x10), dict(loaded_model, flags=0x467, ram=0x20, vram=0x20)])
    # 59DA00 true without freeing this class would loop natively; only false is a terminating input.
    for oom in (0,):
        yield dict(op='release', delta=1, oom_result=oom,
                   classes={'Model': dict(ram_cap=0x10, vram_cap=0x10)},
                   resources=[dict(loaded_model, count=1, ram=0x18, vram=0x8)])


def main():
    rows = []
    for case in cases():
        for pc in h.PRECISION:
            outputs = [Machine(pc, image).run(case) for image in (h.P, h.V)]
            assert outputs[0] == outputs[1], case
            rows.append(dict(input=case, precision=pc, output=outputs[0]))
    static = []
    for address, size in [(0x5aa550, 0x9a), (0x5aee70, 0xe4), (0x5aed90, 0xd9), (0x5b1970, 0x9c), (0x5ac3d0, 0x104),
                          (0x59bf30, 0x6a), (0x59bc70, 0x180), (0x59b9f0, 0x92), (0x59b8c0, 0x61), (0x59ba90, 0xe0),
                          (0x59bfa0, 0x92), (0x5abfb0, 0x40), (0x5b0270, 0x90), (0x5abff0, 0x87), (0x5abb30, 0x40),
                          (0x5aa700, 0x10), (0x5a6560, 0x45), (0x5a6530, 0x2d), (0x5a7cf0, 0xb6), (0x5a7c80, 0x6b),
                          (0x5a5030, 0x40), (0x5a6300, 0x3d), (0x4b2830, 1)]:
        raw = h.P.range(address, size)
        assert raw == h.V.range(address, size)
        static.append(dict(address=hex(address), size=size, sha256=hashlib.sha256(raw).hexdigest()))
    vtables = {k: [hex(h.P.u32(v + 4 * i)) for i in range(22)] for k, v in VT.items()}
    executed = [dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())]
    out = dict(schema=1, scope=__doc__, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA, vtables=vtables,
               cases=rows, static_pins=static, executed_address_byte_pins=executed)
    raw = (json.dumps(out, indent=1) + '\n').encode()
    path = HERE / 'native-release.json'
    if '--write' in sys.argv:
        path.write_bytes(raw)
    else:
        assert path.read_bytes() == raw, 'fixture differs from fresh native replay'
    print(json.dumps(dict(rows=len(rows), machines=2 * len(rows), executed_pins=len(executed),
                          sha256=hashlib.sha256(raw).hexdigest())))


if __name__ == '__main__':
    main()
