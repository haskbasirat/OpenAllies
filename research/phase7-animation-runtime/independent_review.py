"""Independent read-only replay; does not regenerate producer outputs."""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import native_runtime as runtime
import native_bindings as bindings
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
observed={};machines=0
original_init=runtime.Base.__init__
def reviewed_init(self,*args,**kwargs):
 global machines
 original_init(self,*args,**kwargs);machines+=1
 def observe(u,pc,n,_):
  if runtime.original.base<=pc<runtime.original.base+runtime.original.image_size:
   raw=bytes(u.mem_read(pc,n));assert raw==runtime.original.range(pc,n)==runtime.modified.range(pc,n),hex(pc);observed[pc]=raw
 self.u.hook_add(UC_HOOK_CODE,observe)
runtime.Base.__init__=reviewed_init
fixture=json.loads((HERE/'native-runtime.json').read_text());definition=json.loads((HERE/'native-bindings.json').read_text())
registers=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
for number,row in enumerate(reversed(fixture['cases'])):
 m=runtime.Machine(row['input']);reset=m.reset
 def reset_with_sentinels(state,reset=reset,m=m):
  reset(state)
  for i,r in enumerate(registers):m.u.reg_write(r,0x10203040+i*0x11111111)
 m.reset=reset_with_sentinels
 out=m.run();assert out==row['output'],(number,row['input']['operation'])
 assert[m.u.reg_read(r)for r in registers]==[0x10203040+i*0x11111111 for i in range(4)],(number,'callee-saved')
assert runtime.selection_cases()==fixture['selection']
assert runtime.playback_cases()==fixture['playback']
assert runtime.actor_dispatch_cases()==fixture['actor_dispatch']
for key,fn in [('stores',bindings.stores),('fallback',bindings.fallback),('cues',bindings.cues),('lookups',bindings.lookups),('kinds',bindings.type_rows)]:assert fn()==definition[key],key
for rows in [fixture['ranges'],list(fixture['extra_ranges'].values()),list(definition['ranges'].values())]:
 for row in rows:
  start,end=int(row['start'],16),int(row['end'],16);raw=runtime.original.range(start,end-start)
  assert raw==runtime.modified.range(start,end-start)==bytes.fromhex(row['hex']);assert sha(raw)==row['sha256']
paths=['crates/aa-animation/src/runtime.rs','crates/aa-animation/src/definitions.rs','crates/aa-animation/tests/native_runtime.rs','crates/aa-animation/tests/native_definitions.rs','research/phase7-animation-runtime/native_runtime.py','research/phase7-animation-runtime/native_bindings.py','research/phase7-animation-runtime/native-runtime.json','research/phase7-animation-runtime/native-bindings.json','research/phase7-animation-runtime/FINDINGS.txt','research/phase7-animation-runtime/validation-record.json']
report={'schema':1,'status':'native replay passed; focused release Rust checks pending','reviewer':'visual_audit',
 'runtime_cases':len(fixture['cases']),'selection_cases':len(fixture['selection']['selection']),
 'mode_cases':len(fixture['selection']['modes']),'idle_cases':len(fixture['selection']['idle']),
 'playback_cases':len(fixture['playback']),'actor_default_dispatch_cases':len(fixture['actor_dispatch']),
 'definition_cases':{k:len(definition[k])for k in ['stores','fallback','cues','lookups','kinds']},
 'all_visited_instruction_windows':len(observed),'machine_instances':machines,
 'added_checks':[
  'Replayed runtime fixtures in reverse order with independent EBX/ESI/EDI/EBP sentinels, preserving all four registers across all720 calls.',
  'Collected every visited original instruction in all replay groups including transitively called RNG/helpers, and compared bytes independently against original and v14 images.',
  'Compared stored producer results in memory only; no producer fixture regenerated.'],
 'source_review':[
  'Checked563770 pending variant remap and old-End/new-Start callback rereads against actual instructions.',
  'Checked563BA0 active/current/scheduled gate, Performer active/actor flags, idle fallback, RNG repeat timing and event0 scheduling.',
  'Checked4DB170 x87 scale-times-walk denominator, pause shift and prior-kind restart semantics;44F8A1 client playback conversion stays separate from authoritative clock.',
  'Checked selection consumes shared RNG even for one entry and preserves empty-list no-draw; complete masked mode comparison matches keep-previous mode3.',
  'Checked source metadata timing/cue head order, canonical type policy, case-insensitive name/type first-match and aspect/base fallback contracts.'],
 'limitations':[
  'Finite numeric domain and positive source duration are explicit portable preconditions, not proof of arbitrary original NaN/overflow behavior.',
  'FX, scheduler, variant application, actor virtuals, definition/resource loading and clocks require real world services.',
  'Full563E10 dispatcher, save/load, rendered blending and dynamic KF acquisition are outside this milestone.',
  'Case-insensitive source names are tested over ASCII fixture/original identifiers; no Unicode CRT collation parity claim.'],
 'findings':[],'pins':{p:sha((ROOT/p).read_bytes())for p in paths}}
p=HERE/'independent-review.json'
p.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'passed':True,'runtime':len(fixture['cases']),'visited':len(observed),'sha256':sha(p.read_bytes())}))
