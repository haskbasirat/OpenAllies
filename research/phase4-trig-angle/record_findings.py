"""Bind the finite original-angle compatibility path and bounded host tests."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda b:hashlib.sha256(b).hexdigest()


def pin(path):
 p=ROOT/path
 return dict(path=path,size=p.stat().st_size,sha256=sha(p.read_bytes()))


data=json.loads((HERE/'native-angle-status-vectors.json').read_text())
assert data['passed'] and len(data['cases'])==1608
counts=[]
for pc in ['Single24','Double53','Extended64']:
 for pending in [False,True]:
  rows=[r for r in data['cases'] if r['precision']==pc and r['precision_exception_pending']==pending]
  counts.append(dict(precision=pc,incoming_pe=pending,cases=len(rows),qword_roundtrips=sum(r['rounded_through_f64'] for r in rows)))
tables=json.loads((ROOT/'research/phase4-trig/native-trig-vectors.json').read_text())['tables']
binary=(ROOT/'research/phase4-trig/lookup-tables.bin').read_bytes()
import struct
assert binary==struct.pack('<8192I',*tables['cos'],*tables['sin'])
report=dict(schema=1,status='finite original x87 compatibility implemented; current-host instruction parity passed; independent review pending',
 source='crates/aa-trig/src/angle.rs',api='signed_angle(from:[u32;2],to:[u32;2],precision:aa_numeric::Precision,precision_exception_pending:bool)->Result<u32,crate::Error>',
 native_algorithm=['445400 computes cross at selectedx87precision and storesbinary32. Dot is compared againstzero without a binary32store. No vectornormalization isadded.',
                   'Cross clamps[-1,1]. Native96D780 interior asin computes(1+x)*(1-x),FSQRT,FPATAN; endpointsloadexact80bitconstantB359BA (35c26821a2da0fc9ff3f).',
                   '97792B skipsqwordroundtrip forPC53; otherwise currentstickyPE triggers977837 FSTPqword+reload. The incomingflag is preservedasexplicitinput and can also be setbyangle arithmetic.',
                   'Negative dot applies originalf32+PI or-PI subtraction. Finalbinary32store corresponds to caller5775DA, not a retainedextendedreturnvalue.'],
 status_flag_evidence={'branch_counts':counts,'sampled_output_differences':len(data['incoming_pe_output_differences']),
                       'conclusion':'No outputdifferences inthesepairedfixtures. This is notproofincomingPEisunobservablefor allpossibleinputs; implementationhonorsoriginalbranch andkeepsflaginput.'},
 safety=['x86_64-onlycompatibility path; otherarchitecturesreturnUnsupportedPlatform.',
         'Aligned512byteFXSAVE64/FNINIT/FXRSTOR64 isolates selectedRNprecision/exceptionmask andrestorescallerx87executionenvironment/livex87registerstack andMXCSR. NoOS/thread-setting or globalcodechanges.',
         'All asm branches reach FXRSTOR; inputNaN/Inf rejectbeforeasm; all memoryoperands point to fixedlocalbuffers. No externalcalls or stack-pointer changes insideasm.',
         'AX/generalflagsclobber declared. Rust ABI owns normalvolatileXMM registerusage outsideasm; this is floating-environment preservation, not a promise to preserveeveryvolatilegeneral/SIMDregister.'],
 checks=[dict(command='cargo test -p aa-trig angle::tests',passed=True,tests=5,original_clean_cases=168,original_sticky_flag_cases=1608,hardware_table_comparisons=24576),
         dict(command='cargo clippy -p aa-trig --all-targets -- -D warnings',passed=True),
         dict(check='lookup-tables.bin equals recovered JSONcos4096 then sin4096 little-endianbinary32',passed=True)],
 host={'architecture':'x86_64','cpu':'11th Gen Intel Core i7-11700KF','manufacturer':'GenuineIntel','os':'Windows'},
 limits=['No host transcendental library or approximation is used, but hardwareparitywasverifiedonthishost, not everyx86_64CPU.',
         'No fullengine FPUlifecycle/flagtracking inferred. Caller suppliesprecisionand incomingPE; aa-numeric doesnottrackexceptionflags.',
         'Only the supported finite asin path and finalbinary32result are exposed; CRTmatherr/errno/exceptioncallback sideeffects are not reproduced.',
         'Full movement/pathfinding behavior remainsoutside this module.'],
 pins=[pin(p) for p in ['crates/aa-trig/src/angle.rs','research/phase4-trig-angle/original_angle.py','research/phase4-trig-angle/native_trig_base.py','research/phase4-trig-angle/native-angle-status-vectors.json','research/phase4-trig/native-trig-vectors.json','research/phase4-trig/lookup-tables.bin','research/phase4-rng/original_rng.py','research/engine-baseline/audit_baseline.py']],
)
p=HERE/'findings.json';p.write_text(json.dumps(report,indent=2)+'\n');print(sha(p.read_bytes()))
