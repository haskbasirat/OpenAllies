"""Placement source-field dependency slice; not full source-loader completion.

Executes actual 45DB10 with native virtual24 getter/actual vtables, records the
ordered field reads, executes constructor zero stores in isolation, and runs
guarded empty-Settlement/Upgrade prepasses. Source field initial values in the
real-source rows come from the separately retained resolved-source receipt;
those rows do NOT execute TGI loading. All executed instructions are checked
against the original and v14 images. No production/native files are changed.
"""
from pathlib import Path
import csv
import itertools
import importlib.util
import json
import struct
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-economy'))
from harness import Machine, MODES, ORIGINAL, ACCEPTED, sha, pin
from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_ESI, UC_X86_REG_EBX
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
from capstone.x86_const import X86_OP_MEM

FIELDS = {0x34: 1, 0x430: 4, 0x444: 4, 0x1a8: 1, 0x290: 1, 0x4e0: 4}
TABLES = [0xa20770, 0xa13318, 0xa05540, 0xa0b218, 0xa10210, 0xa20800,
          0xa20890, 0xa20920, 0xa209b0, 0xa20a40, 0xa10e10, 0xa102a0,
          0xa20ad0, 0xa20b60, 0xa210e8, 0xa10330, 0xa20bf0, 0xa10ea0,
          0xa133b0, 0xa09b30, 0xa13448, 0xa0f6e0, 0xa10450, 0xa103c0]
FUNCTIONS = {}
with (ROOT.parent / 'reconstruction/research/native-aa/functions.tsv').open(newline='') as f:
    for row in csv.DictReader(f, delimiter='\t'):
        FUNCTIONS[int(row['address'], 16)] = int(row['bodyBytes'])
FUNCTIONS[0x462e50] = 4  # Vtable-reached byte getter omitted by auto inventory.

class Audit(Machine):
    def __init__(self, cw, image):
        super().__init__(cw)
        if image == 'v14':
            old = (ORIGINAL.size + 4095) & ~4095
            new = (ACCEPTED.size + 4095) & ~4095
            if new > old:
                self.uc.mem_map(ORIGINAL.base + old, new-old)
            for rva, size, offset, _ in ACCEPTED.sections:
                if size:
                    self.uc.mem_write(ACCEPTED.base+rva, ACCEPTED.data[offset:offset+size])
        self.cursor = 0x2001000
        self.regions = {}
        self.trace = []
        self.writes = []
        self.ranges = [(0x400000, 0x1000000)]
        self.uc.hook_add(UC_HOOK_MEM_READ, self.read_trace)
        self.uc.hook_add(UC_HOOK_MEM_WRITE, self.write_trace)

    def fixture(self, name, size):
        address = self.cursor
        self.cursor += (size+15) & ~15
        assert self.cursor < 0x2400000
        self.uc.mem_write(address, b'\0'*size)
        self.regions[name] = (address, size)
        return address

    def actor(self, name):
        actor = self.fixture(name, 0x600)
        self.put(actor, 0xa05540)
        return actor

    def read_trace(self, uc, access, address, size, value, _):
        for name, (start, length) in self.regions.items():
            if length == 0x600 and start <= address < start+length and any(
                    off <= address-start < off+width for off, width in {**FIELDS, 0x174: 4}.items()):
                off = address-start
                self.trace.append([name, off, size])
                break

    def write_trace(self, uc, access, address, size, value, _):
        for name, (start, length) in self.regions.items():
            if start <= address < start+length:
                self.writes.append([name, address-start, size, value & ((1 << (size*8))-1)])
                break

    def field_snapshot(self):
        return {name: {str(off): bytes(self.uc.mem_read(start+off, size)).hex() for off, size in FIELDS.items()}
                for name, (start, length) in self.regions.items() if length == 0x600}

def predicate(m, actor, required, own, settlement, owner, requirement):
    m.uc.mem_write(actor+0x34, bytes([own]))
    m.put(actor+0x174, settlement << 14)
    m.put(actor+0x430, required if owner else 0)
    m.put(actor+0x4e0, required if requirement is not None else 0)
    m.put8(required+0x34, requirement or 0)
    m.trace.clear(); m.writes.clear()
    result, _ = m.call(0x45db10, [], actor)
    assert result in (0, 1)
    assert not m.writes
    return {'result': result, 'reads': list(m.trace)}

def prepasses(image, precision, cw):
    m = Audit(cw, image)
    manager = m.fixture('manager', 0x100)
    category = m.fixture('category_heads', 0x100)
    kind = m.fixture('kind_heads', 0x100)
    m.put(manager+0x5c, category); m.put(manager+0x68, kind)
    a, b = m.actor('building_a'), m.actor('building_b')
    nodes = m.fixture('building_nodes', 24)
    m.put_words(nodes, [a, 0, nodes+12, b, 0, 0])
    m.put(category+0x50, nodes)
    # Actual shipped receipt has zero category21 registrations. Actor+4C0
    # remains constructor-empty because the complete Upgrade census is empty.
    before = m.field_snapshot()
    m.call(0x4bb480, [], manager)
    m.call(0x4bb130, [], manager)
    assert m.field_snapshot() == before
    assert not m.writes
    m.call(0x45f7a0, [], a)
    assert m.field_snapshot() == before
    assert not m.writes
    # Positive child-list propagation writes only +454/+458. Nonempty source
    # construction lists are allowed; no fake 'all construction absent' guard.
    node = m.fixture('construction_node', 8)
    m.put_words(node, [b, 0]); m.put(a+0x44c, node)
    m.seam(0x6f7810, lambda machine, esp: (machine.alloc(machine.arg(0)), 0))
    m.call(0x45f7a0, [], a)
    assert m.field_snapshot() == before
    assert all(name == 'building_a' and off in (0x454, 0x458) for name, off, size, value in m.writes)
    return {'image': image, 'precision': precision,
            'case': 'actual-guarded-prepasses-and-positive-child-propagation',
            'placement_fields_unchanged': True, 'writes': m.writes,
            'executed_addresses': sorted(m.visited)}

def stores():
    cs = Cs(CS_ARCH_X86, CS_MODE_32); cs.detail = True
    code = list(cs.disasm(ORIGINAL.at(0x4624b0, FUNCTIONS[0x4624b0]), 0x4624b0))
    out = []
    for insn in code:
        if insn.mnemonic != 'mov' or len(insn.operands) != 2 or insn.operands[0].type != X86_OP_MEM:
            continue
        dest = insn.operands[0]
        if insn.reg_name(dest.mem.base) == 'esi' and dest.mem.disp in (0x430, 0x444, 0x1a8, 0x290, 0x4e0):
            # Actual constructor has EBX == 0 from 4624E1; isolate the exact
            # store only. This is not a fabricated full-constructor execution.
            assert insn.op_str.endswith(', ebx') or insn.op_str.endswith(', bl')
            assert dest.size == FIELDS[dest.mem.disp]
            assert ORIGINAL.at(insn.address, insn.size) == ACCEPTED.at(insn.address, insn.size)
            out.append((insn.address, insn.size, dest.mem.disp, dest.size, insn.mnemonic+' '+insn.op_str))
    assert {r[2] for r in out} == {0x430, 0x444, 0x1a8, 0x290, 0x4e0}
    return out

def verifiers(image, precision, cw):
    m = Audit(cw, image)
    game = m.fixture('game_data', 0x400)
    manager = m.fixture('manager', 0x100)
    category = m.fixture('category_heads', 0x100)
    kind_heads = m.fixture('kind_heads', 0x100)
    m.put(0xb36530, game); m.put(game+0xc, manager)
    m.put(manager+0x5c, category); m.put(manager+0x68, kind_heads)
    descriptor = m.fixture('category_descriptor', 0x20)
    array = m.fixture('all_registered_things', 24*4)
    actors = []
    for kind, table in enumerate(TABLES):
        actor = m.actor(f'kind{kind}')
        m.put(actor, table); m.put(actor+0x18, descriptor)
        actors.append(actor)
    m.put_words(array, actors); m.put(manager+0x24, array); m.put(manager+0x28, len(actors))
    before = m.field_snapshot()
    m.call(0x4bc7f0, [], manager)
    assert m.field_snapshot() == before
    assert not m.writes
    return {'image': image, 'precision': precision,
            'case': 'all-24-actual-vtable-verifiers-with-empty-replacement-heads-and-valid-empty-FX',
            'placement_fields_unchanged': True, 'writes': m.writes,
            'executed_addresses': sorted(m.visited)}

def main():
    inputs = json.loads((HERE / 'actual-source-requirements.json').read_text())
    synthetic, actual, constructor, prepass_rows, verifier_rows, leaf_rows = [], [], [], [], [], []
    leaf_producer = ROOT / 'research/skirmish-20261016-combat-source-completion/review/native_prepass_write_audit.py'
    spec = importlib.util.spec_from_file_location('placement_source_independent_leaf_audit', leaf_producer)
    leaf = importlib.util.module_from_spec(spec); spec.loader.exec_module(leaf)
    prior_protected = list(leaf.PROTECTED)
    constructor_stores = stores()
    for image in ('original', 'v14'):
        for precision, cw in MODES:
            m = Audit(cw, image)
            actor, required = m.actor('owner'), m.actor('required')
            for own, settlement, owner, requirement in itertools.product((0, 1), (0, 1), (0, 1), (None, 0, 1)):
                row_input = {'own34': own, 'settlement_mask14': settlement, 'owner430_present': owner,
                             'required4E0_own34': requirement}
                synthetic.append({'image': image, 'precision': precision, 'input': row_input,
                                  'output': predicate(m, actor, required, own, settlement, owner, requirement)})
            values = next(row['values'] for row in inputs['cases'] if row['source_precision'] == precision)
            for value in values:
                assert not value['owner430'] and not value['head444'] and not value['wall1A8'] and value['required4E0'] is None
                m.put(actor, 0xa09b30 if value['native_kind'] == 19 else 0xa05540)
                output = predicate(m, actor, required, value['own_settlement_space'], 0, 0, None)
                actual.append({'image': image, 'precision': precision, 'id': value['id'], 'output': output})
            for address, length, offset, size, instruction in constructor_stores:
                m.uc.mem_write(actor+offset, b'\xAA'*size)
                m.uc.reg_write(UC_X86_REG_ESI, actor); m.uc.reg_write(UC_X86_REG_EBX, 0)
                m.uc.emu_start(address, address+length, count=1)
                assert bytes(m.uc.mem_read(actor+offset, size)) == b'\0'*size
                constructor.append({'image': image, 'precision': precision, 'offset': offset, 'size': size,
                                    'address': address, 'instruction': instruction, 'result': 0})
            prepass_rows.append(prepasses(image, precision, cw))
            verifier_rows.append(verifiers(image, precision, cw))
            for case in ('upgrade_direct', 'upgrade_indirect', 'upgrade_existing_model', 'upgrade_duplicate',
                         'affiliation', 'affiliation_duplicate', 'selection_layout_clone_append',
                         'weighted_choice', 'weighted_choice_duplicate'):
                # Native affiliation really DOES write +444; retain that as an
                # explicit positive writer, excluded only by the global source
                # Settlement/Upgrade guards. All other tested leaves preserve it.
                leaf.PROTECTED = prior_protected + [0x34, 0x430, 0x1a8, 0x290] + ([] if case.startswith('affiliation') else [0x444])
                row = leaf.run(image, precision, cw, case)
                row['protected_placement_offsets'] = leaf.PROTECTED[len(prior_protected):]
                leaf_rows.append(row)
    addresses = [0x45db10, 0x49b490, 0x462e50, 0x4624b0, 0x469980, 0x49c760,
                 0x4640f0, 0x464bd0, 0x49dc40, 0x4bb480, 0x4bb130, 0x4bb730,
                 0x45fbb0, 0x45fae0, 0x45f7a0, 0x45ee40, 0x4bd440,
                 0x4bc7f0, 0x4bc5e0, 0x461340, 0x49c460, 0x4b2830, 0x4606d0]
    pins = pin({f'{a:08x}': (a, a+FUNCTIONS[a]) for a in addresses})
    pins.update(pin({'required_object_callback_thunk': (0x461850, 0x461863),
                     'random_actor_parent_loader': (0x468d20, 0x468d4d),
                     'native_boolean_body_store': (0x4662f6, 0x46630f),
                     'native_boolean_object_store': (0x49dc6b, 0x49dc81),
                     'required_defer_creation_nonempty_guard': (0x467ba9, 0x467c0e)}))
    vtables = {}
    for kind, table in enumerate(TABLES):
        for offset in (0x24, 0x28, 0x2c, 0x8c):
            raw = ORIGINAL.at(table+offset, 4)
            assert raw == ACCEPTED.at(table+offset, 4)
            vtables[f'kind{kind}+{offset:X}'] = {'slot': table+offset, 'value': struct.unpack('<I', raw)[0], 'bytes': raw.hex()}
    # Keep the entire direct memory-write/call inventory for the typed static
    # audit. A numerical offset in an unrelated object's layout is not by
    # itself evidence of an ActorData writer.
    cs = Cs(CS_ARCH_X86, CS_MODE_32); cs.detail = True
    inventories = {}
    for address in addresses:
        instructions = list(cs.disasm(ORIGINAL.at(address, FUNCTIONS[address]), address))
        inventories[f'{address:08x}'] = [{'address': i.address, 'bytes': i.bytes.hex(),
            'text': i.mnemonic+' '+i.op_str} for i in instructions if i.mnemonic.startswith('call') or
            (i.operands and i.operands[0].type == X86_OP_MEM and i.mnemonic not in ('cmp', 'test', 'push'))]
    doc = {'schema': 1, 'scope': __doc__, 'original_sha256': sha(ORIGINAL.data), 'v14_sha256': sha(ACCEPTED.data),
           'producer_sha256': sha(Path(__file__).read_bytes()),
           'source_receipt_sha256': sha((HERE/'actual-source-requirements.json').read_bytes()),
           'reused_leaf_producer_sha256': sha(leaf_producer.read_bytes()),
           'synthetic_cases': synthetic, 'actual_source_projection_cases': actual,
           'isolated_constructor_stores': constructor, 'guarded_prepasses': prepass_rows,
           'all_kind_virtual_verifiers': verifier_rows,
           'positive_prepass_leaf_write_sets': leaf_rows,
           'function_pins': pins, 'vtables': vtables, 'typed_write_audit_inventory': inventories,
           'limitations': ['Source receipt parsing is independently guarded, not full native TGI startup replay.',
             'Only mask bit14 participates in 45DB10; real-source rows initialize other mask bits zero and claim no full-mask equivalence.',
             'Positive Settlement, Upgrade and required-object graphs remain unsupported by the Rust producer.',
             'Postpass writer exclusions beyond the bounded guarded executions require the accompanying typed static audit.',
             'Successful allocation is the only application seam in the positive child propagation check; allocation failures and external callback reentry excluded.']}
    path = HERE / 'native-source-requirements.json'
    path.write_text(json.dumps(doc, indent=2)+'\n')
    print(json.dumps({'synthetic':len(synthetic),'actual_source':len(actual),'ctor_stores':len(constructor),
                      'guarded_prepasses':len(prepass_rows),'sha256':sha(path.read_bytes())}))

if __name__ == '__main__':
    main()
