"""Hash-bind the bounded body and shared numeric implementation evidence."""
from pathlib import Path
import hashlib,importlib.util,json
HERE=Path(__file__).parent
s=importlib.util.spec_from_file_location('original_body',HERE/'original_body.py')
b=importlib.util.module_from_spec(s);s.loader.exec_module(b)
sha=lambda data:hashlib.sha256(data).hexdigest()


def pin(relative):
 p=b.ROOT/relative
 return dict(path=relative,size=p.stat().st_size,sha256=sha(p.read_bytes()))


windows={}
for name,start,end in [('body_data_load',0x4662b8,0x466380),('full_unported_damage',0x5625c0,0x562b9c),('start_dying_unported',0x562380,0x5625b5)]:
 data=b.IMAGE.at(start,end-start);assert data==b.ACCEPTED.at(start,end-start)
 windows[name]=dict(start=hex(start),end_exclusive=hex(end),sha256=sha(data),bytes=data.hex())
 (HERE/(name+'.asm.txt')).write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in b.Cs(b.CS_ARCH_X86,b.CS_MODE_32).disasm(data,start))+'\n')
report=dict(schema=1,status='bounded body operations implemented and native-oracle matched; not a completed damage/death system',
 original_sha256=b.pe.ORIGINAL_SHA,accepted_sha256=b.pe.ACCEPTED_SHA,
 api=['Body::from_resolved','Body::set_health','Body::set_health_and_maximum','Body::update_resolved_maximum','Body::heal_plain','Body::check_death','Body::sink_fraction_f32'],
 native_fields={'Body+04':'owning actor','Body+10':'current health binary32','Body+14':'resolved maximum health binary32','Body+18':'resolved defense binary32','Body+1C':'dying-start time, initialized-1','Body+20':'last heal time, initialized-999','Body+24':'sink duration, initialized0','Body+28..40':'opaque seven-word damage info, initialized[0,0,0,0,0,0,10]',
 'Actor+14':'GTID low16slot/high16generation; registry responsibility outside aa-body','Actor+60':'Body component pointer','Actor+168':'linked actor GTID requiring original registry generation check','Data+284':'base health getter input','Data+288':'base defense getter input','Data+291':'destructible flag','Data+29C':'WhenHealth list','Data+27C/+280':'rot_time/sink_time'},
 numeric_contract={'primitive':'aa-numeric','modes':['Single24','Double53','Extended64'],'rounding':'nearest-even only, precision chosen per operation; no assumption of process-wide37F',
 'setter_ratio':'reciprocal computed at selectedprecision, multiply old/currenthealth, then each ratio storedf32 before health-rule predicates',
 'restore_ratio':'direct currenthealth/currentmaximum division compared in selectedprecision against suppliedrestorethreshold; no prematuref32 store',
 'recalculate_ratio':'561D40 stores health/maximum tobinary32 before multiplying binary32 null-kingdom getterresult; positive newmaximum<1 can stillproducehealth1 through originalearlyreturn',
 'heal_sum':'ordinary562BA0 adds to the health read after notification, then storesf32 before canonicalsetter; overflow beyondfinite arithmetic is explicitlyoutside',
 'sink':'5617E0 stores elapsedtimef32; subtractrot/dividesink at selectedprecision, then explicitlyobservedbinary32callerstore. Progressmayexceed1.'},
 callbacks={'synchronous':True,'nontransactional_error':True,'order':['storehealth','optional4D0050actorstance','conditional4D64F0state39request','firstmatchingWhenHealth-trigger429290call','conditional566AC0fullrecovery'],
 'mutation_cases':['stancecallback changeshealth, affecting subsequentdisabledgate','stancecallback changesmaximum, affectingfullrecovery','healthFXcallback changeshealth, affectingrecovery','healnotification changeshealthbeforeaddition andworldclockaftersetter','deathvirtualcallback marksdestructed but subsequent4D7960 stillruns'],
 'leaves_not_implemented':['4D0050stance propagation','4D64F0actorstate machine','429290effectchain execution','566AC0recovery/construction/upgrade notifications','50C380healing notification','actorvirtual+2C and4D7960death scheduling']},
 support_boundary=['Finitebinary32body/context inputs; currentmaximumpositive. Sourcevaluesare explicitinputs andnotrawTGIdefaults.','No validlinkedBody/noattackerFXdirection; nohealerMind+A8damageconversion. These unsupported pathsaretypederrors,not skippedbehavior.','Null-kingdom derivedmax getter returnsbinary32; modifiedkingdom getter potentiallyextendedresult requires separatefutureboundary.','No automaticweapon damage formula, armor calculation, capture, killcredit, wall/organizationneighbor propagation or deathability invented.','Body setter reachingzero is not automaticallykill; original check_death event path is separate.'],
 authored_source_notes={'template':'reconstruction/assets/original/Data/Templates/Components/template_body.tgi','restore_ratio':'Original Game/SVars.tgi has0.1, Game/SVars_AA.tgi overrides0.51. Environmenttakesresolvednativevalue; fixture0.25isexplicittestinput,notagamedefault.','live_default_claim':False},
 checked_static_windows=windows,
 pins=[pin(x) for x in ['crates/aa-body/Cargo.toml','crates/aa-body/src/lib.rs','crates/aa-body/tests/native_parity.rs','crates/aa-body/README.md','research/phase4-body/original_body.py','research/phase4-body/body-native-oracle.json','crates/aa-numeric/src/lib.rs','crates/aa-numeric/tests/native_parity.rs','research/phase4-body/numeric_oracle.py','research/phase4-body/numeric-native-oracle.json','research/phase4-orders/numeric-independent-review.json','research/phase4-orders/body-independent-review.json']],
 checks=[dict(command='python research/phase4-body/original_body.py',passed=True,original_x86_cases=270),dict(command='cargo test -p aa-body',passed=True,tests=3),dict(command='cargo clippy -p aa-body --all-targets -- -D warnings',passed=True),dict(command='cargo test -p aa-numeric',passed=True,tests=3,synthetic_x87_cases=234),dict(command='cargo clippy -p aa-numeric --all-targets -- -D warnings',passed=True)])
p=HERE/'findings.json';p.write_text(json.dumps(report,indent=2)+'\n');print(sha(p.read_bytes()))
