"""Read-only existing Windows capture + original/v14 code audit.
Launches no process, debugger or executable; changes no production FP policy.
"""
from pathlib import Path
import sys,json,hashlib,struct,collections
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];GAME=ROOT.parent
GRID=ROOT/'research/skirmish-20261017-view-grid'
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as n
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
reference=GRID/'hardware-grid-seed-reference.json'
assert H(reference)=='64039c63149ed59c3698fa7b6fdabd40f901c5ae861d12ff1054cdb60e7f671b'
ref=json.loads(reference.read_text());observations=[]
for name,script in [('capture-02','capture_grid_startup-v1-frozen.py'),('capture-03','capture_grid_startup.py')]:
 p=GRID/name/'capture.json';doc=json.loads(p.read_text());summary=next(r for r in ref['captures']if r['directory']==name)
 assert H(p)==summary['capture_sha256']and H(GRID/script)==doc['script_sha256']==summary['script_sha256']
 source=(GRID/script).read_text();assert 'c.flags=set_context_flags' in source and 'set_context_flags=0x10001'in source.replace(' ','')
 assert doc['fp_state_never_written_by_debugger']and doc['get_context_flags']==0x1003f and doc['set_context_flags']==0x10001
 assert doc['debug_register_set_context_flags']==0x10010 and doc['probe_integer_control_set_context_flags']==0x10003
 assert doc['original_error']is None and doc['completed_grid_constructor']and doc['cleanup']['wait_result']==0 and not doc['cleanup']['errors']
 for file,digest in doc['runtime_pins'].items():assert H(GRID/name/'runtime'/file)==digest
 initial=next(r for r in doc['global_captures']if r['label']=='create_process_before_program_execution');main=initial['tid'];rows=[]
 for label in ['session_ctor_return','grid_ctor_entry','grid_ctor_return']:
  row=next(r for r in doc['global_captures']if r['label']==label);raw=bytes.fromhex(row['raw_context']);assert len(raw)==716
  legacy=struct.unpack_from('<I',raw,28)[0]&65535;fxsave=struct.unpack_from('<H',raw,204)[0]
  sw=struct.unpack_from('<I',raw,32)[0]&65535;fxsw=struct.unpack_from('<H',raw,206)[0]
  assert row['tid']==main and legacy==fxsave==0x027f and sw==fxsw
  rows.append(dict(label=label,tid=row['tid'],eip=hex(row['eip']),legacy_cw=hex(legacy),fxsave_cw=hex(fxsave),sw=hex(sw),precision='Double53',session_phase=row['session_phase'],mxcsr=hex(struct.unpack_from('<I',raw,228)[0])))
 assert rows[1]['eip']=='0x430290'and rows[2]['eip']=='0x43f124'
 other=sorted({r['tid']for r in doc['global_captures']if r['label']=='change_state_entry'})
 assert main not in other
 observations.append(dict(capture=name,capture_sha256=H(p),script_sha256=H(GRID/script),runtime_pins=doc['runtime_pins'],initial_main_tid=main,session_phase_transition_tids=other,observed_contexts=rows,
  qualification='Same CREATE_PROCESS main TID at real constructor; only these exact observations are hardware FP evidence. No full43F0A0 return or later main-frame CW capture.'))

native=GAME/'reconstruction/research/native-aa';functions={}
for line in(native/'functions.tsv').read_text().splitlines()[1:]:
 fields=line.split('\t');functions[int(fields[0],16)]=int(fields[3])
graph=collections.defaultdict(set)
for line in(native/'calls.tsv').read_text().splitlines()[1:]:
 a,b,*_=line.split('\t')
 if not b.startswith('EXTERNAL'):graph[int(a,16)].add(int(b,16))
fp=ROOT/'research/phase8-tick/fpu-sites.json';sites=json.loads(fp.read_text())['sites']
assert not [s for s in sites if int(s['site'],16)<0x7d0000]
closures=[]
for start in[0x5c3000,0x43f0a0,0x700e30,0x7010d0,0x6ff130,0x723e00]:
 seen={start};todo=[start]
 while todo:
  for child in graph[todo.pop()]:
   if child not in seen:seen.add(child);todo.append(child)
 selected=[s for s in sites if int(s['function'],16)in seen]
 assert not {0x96f7aa,0x96f778,0x891890}&seen
 closures.append(dict(root=hex(start),direct_reachable_functions=len(seen),create_device_891890_reachable=False,controlfp_control87_reachable=False,
  control_state_sites=[dict(site=s['site'],function=s['function'],instruction=s['instruction'])for s in selected],
  qualification='Direct-call lower bound only; virtual calls and external DLL FP behavior are not proven absent. Reached control-state sites are CRT helpers, not game-level mode setters.'))
md=Cs(CS_ARCH_X86,CS_MODE_32);windows=[]
for start,length,label in [(0x410e70,functions[0x410e70],'main-frame consumes NetEvents before UIUpdate/render'),(0x40fbe0,functions[0x40fbe0],'queue Pop followed by current AppState virtual1C'),
 (0x682fd0,0x9e,'Loading derived notification then base414620'),(0x414620,functions[0x414620],'base OnNetEvent switch; case1 direct40CF00'),
 (0x40cf00,functions[0x40cf00],'synchronous5C3000 then current screen virtualE8'),(0x5c3000,functions[0x5c3000],'reset700E30 then43F0A0; resource task scheduled AFTER world-view Init'),
 (0x43f0a0,0x150,'VWorld prefix contains direct430290 constructor call'),(0x700e30,functions[0x700e30],'camera Reset direct body'),(0x7010d0,functions[0x7010d0],'scene camera reset direct body')]:
 raw=n.P.range(start,length);assert raw==n.V.range(start,length)
 instructions=[dict(address=hex(i.address),bytes=i.bytes.hex(),instruction=i.mnemonic+' '+i.op_str)for i in md.disasm(raw,start)]
 assert not any(i['instruction'].split()[0]in('fldcw','fldenv','frstor','fxrstor','fninit','finit','ldmxcsr')for i in instructions)
 windows.append(dict(address=hex(start),length=length,label=label,sha256=hashlib.sha256(raw).hexdigest(),instructions=instructions))
required_edges={0x410e70:[0x40fbe0],0x40fbe0:[0x785060],0x682fd0:[0x414620],0x414620:[0x40cf00],0x40cf00:[0x5c3000],0x5c3000:[0x700e30,0x43f0a0,0x5b17f0,0x5aed10],0x43f0a0:[0x430290]}
for window in windows:
 calls={i['instruction']for i in window['instructions']}
 for child in required_edges.get(int(window['address'],16),[]):assert f'call {hex(child)}'in calls
for address,target in[(0xa6e6c0,0x682fd0)]:assert n.P.range(address,4)==n.V.range(address,4)==target.to_bytes(4,'little')
out=dict(schema=1,scope=__doc__,observations=observations,direct_closures=closures,pinned_equal_code_windows=windows,
 conclusion='Use explicit Double53 for the captured v14/wrapper view-startup reference. The main-thread label alone does not imply PC24. Existing production Double53 was not changed.',
 route='410E70→40FBE0→785060 notification Pop→current AppState virtual1C→414620 (directly or through Loading682FD0)→case1 40CF00→5C3000→700E30→43F0A0→430290.5B17F0/5AED10 resource task is scheduled after43F0A0 returns, not a thread transfer that executes43F0A0. Actual selected AppState vtable was not recorded by these captures.',
 limits=['Hardware observations stop at Grid constructor return43F124; later scene/terrain FP follows explicit caller context as a supported inference, not a new whole-Init CW observation.',
 'Old static D3D9-main-PC24 expectation is not hardware evidence for the configured wrapper. The mechanism producing observed main PC53 is not identified here.',
 'Bare original-system Direct3D, other wrappers/device recreation and all later main frames are not assigned a precision by this report.',
 'CRT helpers may temporarily alter/restore control state; finite normal paths are distinct from unmasked exception handler behavior.',
 'Source Bank precision remains separately sourced; its53 value is not used to prove view execution precision.'],
 provenance={str(p.relative_to(ROOT)).replace('\\','/'):H(p)for p in[reference,fp,ROOT/'research/phase8-tick/fpu-precision.md']})
data=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'findings.json'
if '--write'in sys.argv:p.write_bytes(data)
else:assert p.read_bytes()==data
print(json.dumps(dict(captures=len(observations),code_windows=len(windows),main_precision='Double53',sha256=hashlib.sha256(data).hexdigest())))
