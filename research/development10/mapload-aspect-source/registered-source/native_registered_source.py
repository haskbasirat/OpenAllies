"""Bounded native 4BDC50 registration result -> vf0 receiver distinction.

No product edits, process launch or complete DataLoad assertion. The callback
entry is an explicit observation/return seam. Category kind0 uses the existing
working ThingData constructor laboratory. Additional actual OBJECT kind1
factory routing rows use a named opaque49C760 constructor-return seam.
"""
from pathlib import Path
import gc
import hashlib
import json
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT / 'research/phase8-definitions'))
import native_definition_registry as registry
import native_machine as machine
import native_tgi_parser as parser
import trace_registry
import original_creation
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_ESP, UC_X86_REG_FPCW

ORIGINAL, V14 = machine.P, machine.V
H = lambda b: hashlib.sha256(b).hexdigest()
PC = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}
PAYLOAD = b'[Thing]\r\n{\r\n IDS = chaparral_wall\r\n category = THING\r\n}\r\n'


class Probe(machine.Machine):
    def __init__(self):
        self.registration_pointer = 0
        self.load_pc = 0
        self.observations = []
        self.constructor_seams = []
        self.in_root = False
        super().__init__()

    def hook(self, u, pc, size, user):
        self.visited[pc] = size
        if self.in_root and pc == 0x49c760:
            # Parent-approved opaque constructor-return seam. Native4BCA00 has
            # selected OBJECTkind1 and allocated its actual0x170-byte block.
            # Bind only identity cells required by native registration, not
            # any ObjectData fields/geometry/resource/typed-loader state.
            pointer = u.reg_read(UC_X86_REG_ECX)
            ids = self.r(self.arg())
            category = self.arg(1)
            vtable = self.alloc(0x90)
            load = self.stub(lambda mm, uu: (_ for _ in ()).throw(AssertionError('unobserved loader')))
            classification = self.stub(lambda mm, uu: mm.ret(0))
            self.w(vtable, load)
            self.w(vtable + 0x88, classification)
            self.w(pointer, vtable)
            self.w(pointer + 8, ids)
            self.w(pointer + 0xc, 0xffffffff)
            self.constructor_seams.append({'pc': hex(pc), 'pointer': pointer,
                                           'ids': self.wstr(ids), 'category': category,
                                           'category_kind': self.r(category+0x18),
                                           'opaque_vtable': vtable, 'vf0': load})
            self.ret(pointer, pop=8)
            return
        if self.in_root and pc == 0x4bdd0e:
            self.registration_pointer = u.reg_read(UC_X86_REG_EAX)
            self.load_pc = self.r(self.r(self.registration_pointer))
            self.observations.append({'event': 'actual_4bca00_return', 'pc': hex(pc),
                                      'pointer': self.registration_pointer,
                                      'vtable': self.r(self.registration_pointer), 'vf0': self.load_pc})
        if self.in_root and pc == 0x4bdd15:
            self.observations.append({'event': 'actual_indirect_vf0_call', 'pc': hex(pc),
                                      'ecx': u.reg_read(UC_X86_REG_ECX),
                                      'node_argument_before_call': self.r(u.reg_read(UC_X86_REG_ESP)),
                                      'target': self.r(u.reg_read(UC_X86_REG_EDX))})
        if self.in_root and self.load_pc and pc == self.load_pc:
            self.observations.append({'event': 'named_LoadData_entry_seam', 'pc': hex(pc),
                                      'ecx': u.reg_read(UC_X86_REG_ECX),
                                      'node_argument_at_entry': self.r(u.reg_read(UC_X86_REG_ESP) + 4)})
            # Stop the typed loader child without substituting registration,
            # direct receiver selection, translation or either registry Find.
            self.ret(1, pop=4)
            return
        super().hook(u, pc, size, user)


def source_witness():
    report_path = ROOT / 'research/phase8-definitions/native-load-report.json'
    report = json.loads(report_path.read_text())
    registered = dict(report['things'])
    translations = dict(report['thing_translations'])
    intersection = sorted(set(registered) & set(translations))
    assert intersection == ['chaparral_wall'], intersection
    assert registered['chaparral_wall'] == 'Doodads/Chaparral/chaparral_wall.tgi'
    assert registered['temperate_walls'] == 'Doodads/Temperate/temperate_walls.tgi'
    assert translations['chaparral_wall'] == 'temperate_walls'
    files = [registered['chaparral_wall'], registered['temperate_walls'],
             'Templates/template_doodad_aa.tgi', 'Templates/template_object.tgi',
             'Templates/template_dynamic_doodad.tgi', 'translate.tgi']
    data = ROOT.parent / 'reconstruction/assets/original/Data'
    pins = [{'path': name, 'sha256': H((data/name).read_bytes())} for name in files]
    return {'report': str(report_path.relative_to(ROOT)).replace('\\', '/'),
            'report_sha256': H(report_path.read_bytes()),
            'registered_translation_intersection': intersection,
            'registered_source': registered['chaparral_wall'],
            'translated_target_source': registered['temperate_walls'],
            'translation': ['chaparral_wall', 'temperate_walls'],
            'qualification': 'Actual catalog report and raw-source hashes only; full source/template/typed loader is not replayed here.',
            'actual_category_chain': 'chaparral_wall -> AAChaparralDoodad -> AADoodad -> Object (OBJECT kind1); temperate_walls -> AATemperateDynamicDoodad -> AADynamicDoodad -> DynamicDoodad (kind21).',
            'files': pins}


def image_witness():
    for _, rva, size, offset in ORIGINAL.sections:
        for i in range(size):
            address = ORIGINAL.base + rva + i
            left = ORIGINAL.b[offset+i:offset+i+1]
            right = V14.range(address, 1)
            if left != right:
                return address, left, right
    raise AssertionError('Two pinned images unexpectedly have no differing section byte')


def main():
    visited, rows, selections = {}, [], []
    witness, original_byte, v14_byte = image_witness()
    for image, selected in [('original', ORIGINAL), ('v14', V14)]:
        # The deepest base registry Machine owns actual PE section loading.
        trace_registry.P = original_creation.P = parser.P = machine.P = registry.P = selected
        for precision, control in PC.items():
            for source_kind, timing in [(0, 'translation_after_registration'),
                                        (0, 'translation_before_registration_lab'),
                                        (1, 'translation_after_registration')]:
                m = Probe()
                expected = original_byte if image == 'original' else v14_byte
                assert bytes(m.u.mem_read(witness, 1)) == expected
                m.u.reg_write(UC_X86_REG_FPCW, control)
                mgr = registry.make_thing_manager(m)
                categories = registry.make_thing_manager(m)
                category = registry.register_def(m, categories, 'THING')
                m.w(category + 0x18, 0)
                m.w(category + 0xc, 0)
                object_category = registry.register_def(m, categories, 'OBJECT')
                m.w(object_category + 0x18, 1)
                m.w(object_category + 0xc, 1)
                gd = m.alloc(0x1000)
                m.w(0xb36530, gd)
                m.w(gd + 0x3b4, categories)
                target = m.thiscall(0x4bca00, mgr, [m.cstring('temperate_walls'), category])['eax']
                if timing == 'translation_before_registration_lab':
                    registry.insert_translation(m, mgr, 'chaparral_wall', 'temperate_walls')
                payload = PAYLOAD if source_kind == 0 else PAYLOAD.replace(b'category = THING', b'category = OBJECT')
                parsed = m.reparse(payload, 'registered-pointer-lab.tgi')
                assert parsed['error'] is None
                assert m.r(parsed['root'] + 0x38) == 1
                node = m.r(m.r(parsed['root'] + 0x34))
                m.in_root = True
                returned = m.thiscall(0x4bdc50, mgr, [node])
                m.in_root = False
                assert 'thrown' not in returned, returned
                created = m.registration_pointer
                assert created and created != target
                assert m.wstr(m.r(created + 8)) == 'chaparral_wall'
                assert m.wstr(m.r(target + 8)) == 'temperate_walls'
                call, entry = m.observations[1:]
                assert call['ecx'] == entry['ecx'] == created
                assert call['node_argument_before_call'] == entry['node_argument_at_entry'] == node
                assert call['target'] == m.load_pc
                assert len(m.constructor_seams) == source_kind
                before = m.thiscall(0x427bd0, mgr, [m.cstring('chaparral_wall')])['eax']
                if timing == 'translation_after_registration':
                    assert before == created
                    registry.insert_translation(m, mgr, 'chaparral_wall', 'temperate_walls')
                else:
                    assert before == target
                after = m.thiscall(0x427bd0, mgr, [m.cstring('chaparral_wall')])['eax']
                target_find = m.thiscall(0x427bd0, mgr, [m.cstring('temperate_walls')])['eax']
                assert after == target_find == target
                # Direct registry read is 4BCA00's own duplicate-check operation,
                # not the translation-bearing 427BD0 API.
                name_cell = m.alloc(4)
                m.w(name_cell, m.cstring('chaparral_wall'))
                direct_entry = m.thiscall(0x4915f0, mgr+0x30, [name_cell])['eax']
                direct = m.r(direct_entry)
                assert direct == created
                assert m.u.reg_read(UC_X86_REG_FPCW) & 0x300 == control & 0x300
                m.check_null()
                rows.append({'image': image, 'precision': precision, 'schedule': timing,
                             'manager': mgr, 'category_kind_transport': source_kind,
                             'node': node, 'registration_pointer': created,
                             'LoadData_ecx': entry['ecx'], 'translated_target_pointer': target,
                             'lookup_before_installing_translation': before,
                             'final_Find_chaparral_wall': after, 'final_Find_temperate_walls': target_find,
                             'direct_registry_chaparral_wall': direct,
                             'observations': m.observations,
                             'constructor_return_seams': m.constructor_seams,
                             'null_accesses': m.null_accesses})
                selections.append({'image': image, 'precision': precision, 'schedule': timing,
                                   'source_kind': source_kind,
                                   'address': hex(witness), 'loaded_byte': expected.hex()})
                visited.update(m.visited)
                del m
                gc.collect()
    pins = []
    for address, size in sorted(visited.items()):
        if not ORIGINAL.base <= address < ORIGINAL.base + ORIGINAL.image_size:
            continue
        if address in (0x4bb310, 0x49c760):
            # Hook-entry marker only: the typed child instructions do not run.
            continue
        raw = ORIGINAL.range(address, size)
        assert raw == V14.range(address, size), hex(address)
        pins.append({'va': hex(address), 'bytes': raw.hex()})
    required = [0x4bdd09, 0x4bdd0e, 0x4bdd10, 0x4bdd12, 0x4bdd13, 0x4bdd15, 0x4bca33,
                0x4bcaa2, 0x4bcac8, 0x427cd1]
    assert all(any(int(pin['va'],16) == address for pin in pins) for address in required)
    output = {'schema': 1, 'scope': 'Actual4BDC50 -> actual4BCA00 returned allocation -> actual indirectvf0 receiver, contrasted with later actual427BD0 translation lookup; pointer-selection proof only.',
              'original_sha256': H(ORIGINAL.b), 'v14_sha256': H(V14.b),
              'source_witness': source_witness(), 'cases': rows,
              'loaded_image_witnesses': selections, 'instructions': pins,
              'pin_count_qualification': 'Visited native address pins include inherited parser/allocator/log transport entry markers. The new LoadData and opaque constructor entry markers are excluded; required call/receiver-selection instructions execute. Do not call the aggregate count nonseam executed instructions.',
              'seams': ['Existing phase8 native Machine parser line source, heap/string-pool allocation/release, empty manager/global fixture storage and logs.',
                        'Category manager uses the original4BD9C0 map layout with fixture registered THING and OBJECT category descriptors; kind0/+C0 and kind1/+C1 are explicit descriptor projections. Native4BCA00 actually reads kind1 and selects its original OBJECT branch/call49C760 in six routing cases.',
                        'OBJECT49C760 is a named opaque constructor-return seam over the SAME native-allocated0x170 block, writing only observed registration identity/vtable cells (IDS8, unregistered indexC=-1) and returning it. It does not emulate ObjectData fields or establish constructor readiness. The distinct temperate_walls target remains a controlled nativekind0 allocation; its realkind21 constructor is outside scope.',
                        'Parsed laboratory node contains real shadow IDS chaparral_wall with explicit category=THING or OBJECT; raw real source/category/template chain are separately hash-pinned, not substituted as a full parsed source load.',
                        'Actual indirectvf0 call targets native4BB310 forkind0 or the explicit opaque observation vtable forkind1. Its entry records receiver and node, then returns1/pop4. No LoadData body, source field side effects or full GameData load claim.',
                        'Translation inserted with actual478E10/5043F0; direct registry lookup4915F0 and final Find427BD0 execute. No lookup answer or registered returned pointer is supplied by a seam.'],
              'limits': ['The actual-order schedule registers canonical data before installing final translation; the translation-before schedule is a controlled stronger shadow lab, not native startup chronology.',
                         'No Rust/product edits/builds, game process, GUI, Resource/Aspect/GPU request, Bank readiness or whole DataLoad acceptance.',
                         'A separate bounded full OBJECTkind1 constructor attempt encountered missing global dependency49C98E +30 in inherited lab. The additional routing cases name their opaque49C760 seam explicitly; no guessed owner or successful full OBJECT constructor is claimed.']}
    path = HERE / 'native-registered-source.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(output, indent=1)+'\n')
    else:
        assert json.loads(path.read_text()) == output
    print(json.dumps({'cases': len(rows), 'visited_native_address_pins': len(pins),
                      'sha256': H(path.read_bytes()), 'scope': output['scope']}))


if __name__ == '__main__':
    main()
