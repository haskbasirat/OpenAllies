from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
fixture=json.loads((HERE/'native-member-properties.json').read_text())
files=['crates/aa-runtime/src/saved_member_properties.rs','crates/aa-runtime/src/saved_member_properties_tests.rs']
files+=['research/skirmish-20261016-member-properties/'+name for name in ['native_member_properties.py','actual-saved-inputs.json','native-member-properties.json','README.txt','pin_findings.py']]
document=dict(schema=1,scope='Native5784C0 live guard/read coordinator; actual saved empty-list and required-instance boundary integration',
 production_frozen=True,native_cases=len(fixture['cases']),instruction_pins=len(fixture['instructions']),
 original_sha256=fixture['original_sha256'],v14_sha256=fixture['v14_sha256'],
 scope_counts={scope:sum(c['scope']==scope for c in fixture['cases']) for scope in ['coordinator','lookup','actual_saved']},
 proven=['ordered lazy Properties/definition/Experience/kind/disabled guards','captured owner Properties with live slots/count across explicit callback seams','fresh org+4 source owner and instance+8 kind reads','4CEF80/581280/5032D0 first-match query including disabled records, optional source filter and generation validity','actual tutorial02 slot4 owner/member prefix and constructor-empty Properties','missing instance/member services required only when native reaches their reads','same owner shell/Organization/Properties allocation retained'],
 not_proven=['actual nonempty saved effects','canonical PropertyData loading','real4CEF10 admission/insertion/effects','immediate Init/PostCreate or playable battle','malformed native pointer exception equivalence','arbitrary component-destroying reentry'],
 files={name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in files})
(HERE/'findings.json').write_text(json.dumps(document,indent=2)+'\n')
print(json.dumps(dict(sha256=hashlib.sha256((HERE/'findings.json').read_bytes()).hexdigest(),native_cases=document['native_cases'])))
