from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=[ROOT/'crates/aa-client/src/view_organization_fields.rs',ROOT/'crates/aa-client/src/view_organization_fields_tests.rs']
paths += [HERE/n for n in ['native_organization.py','native-organization-fields.json','audit_organization.py','pin-audit.json','isolated-organization.exe','tests-01.log','native-final-replay.log','pin-final-replay.log','README.md','seal.py']]
doc=dict(schema=1,status='accepted bounded Organization44FDB0 tail; Directory integration separate',
 boundary='44FDDF..44FE3D after separately retained453F10 base',
 native=dict(paired_rows=24,executions=48,precision=['Single24','Double53','Extended64'],decoded_equal_executed_pins=37,seams=['453F10 separately retained Actor base','6F7810 allocator transport'],object_heap_poison_combinations=4,allocator_entry_prefix=True),
 rust=dict(exact_source_isolated_tests=2,passed=2,whole_client_check='not claimed; SDK owns same actual derived construction/Directory caller'),
 ownership='158 array has sole growable item/count owner and native initial capacity14;164/168 typed constructor-null links;16C byte only;170 scalar; no source/global callbacks or fabricated pointer/padding values',
 limitations=['No native OOM/unwind/destructor/heap ABI equivalence','Portable allocation refusal returns the observed pre-allocation field prefix, not a claim of native allocation failure','Unused native allocation slots and16D..16F padding remain unknown','No whole Actor/base/derived Init, Thing+C publication, GPU, AD or application readiness claim'],
 pins={p.relative_to(ROOT).as_posix():H(p)for p in paths})
data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'findings.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(json.dumps(dict(sha256=hashlib.sha256(data).hexdigest(),pins=len(paths))))
