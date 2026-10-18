"""Static source/PE dependencies; native-source-context.json records separate execution."""
from pathlib import Path
import sys,json,re
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import original,modified,sha,Machine
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
HERE=Path(__file__).resolve().parent;GAME=ROOT.parent
names=['CombatDamageMinimum','CombatValueRangePower','CombatValueAreaPower','CombatValueOffenseWeight','CombatValueDefenseWeight','CombatValueMovementWeight','CombatValueMovementDefault','CombatValueMeanValuesDiscardPercent']
allnames=names+['CombatValueRangedMinimum','CombatValueDefenseWeightRanged','CombatValueCSV']
cs=Cs(CS_ARCH_X86,CS_MODE_32);ins=list(cs.disasm(original.range(0x5065d0,0x3000),0x5065d0));registrations=[]
for name in names:
 off=original.b.find(name.encode('utf-16-le')+b'\0\0');va=None
 for _,rva,size,pos in original.sections:
  if pos<=off<pos+size:va=original.base+rva+off-pos
 assert va is not None
 for n,i in enumerate(ins):
  if i.mnemonic=='push' and i.op_str==hex(va):
   seq=ins[n-4:n+9];a=seq[0].address;b=seq[-1].address+seq[-1].size;raw=original.range(a,b-a)
   assert raw==modified.range(a,b-a)
   registrations.append({'name':name,'string_va':hex(va),'start':hex(a),'end':hex(b),'sha256':sha(raw),'assembly':[f'{q.address:08x}: {q.bytes.hex()} {q.mnemonic} {q.op_str}' for q in seq]})
source=[]
for tree in ['reconstruction/assets/original/Data','reconstruction/native-game/runtime-expanded-v14/Data']:
 base=GAME/tree
 files=[p for p in base.glob('UVars*.tgi') if p.is_file()]+[p for p in (base/'Game').glob('SVars*.tgi') if p.is_file()]
 for p in files:
  text=p.read_bytes().decode('cp1252');lines=[]
  for n,line in enumerate(text.splitlines(),1):
   if any(re.search(r'\b'+re.escape(name)+r'\b',line) for name in allnames):lines.append({'line':n,'text':line})
  source.append({'path':str(p),'sha256':sha(p.read_bytes()),'relevant_lines':lines})
extra=[]
for label,a,b in [('fixed_ctor',0x401710,0x4017c6),('mean_discard_reads',0x4bc2dc,0x4bc359),('initial_ranged_lookup',0x45e0d9,0x45e117),('CSV_global_lookup',0x4bb5b0,0x4bb5fc),('source_settings_order',0x48f436,0x48f46c),('source_defaults_copy_before_completion',0x4902a7,0x4902b6),('wallset_source_identity',0x47df34,0x47dff8),('settlement_loader',0x4640f0,0x464a70),('settlement_radius_fitting',0x460940,0x4612e0)]:
 raw=original.range(a,b-a);assert raw==modified.range(a,b-a)
 extra.append({'name':label,'start':hex(a),'end':hex(b),'sha256':sha(raw)})
fixture=ROOT/'research/phase8-combat/native-combat-value-fixtures.json';fd=json.loads(fixture.read_text());m=Machine('Single24')
report={'schema':1,'scope':'Static source/PE dependency audit accompanying native-source-context.json. Source rows are evidence, not a production parser or executed full data load.','producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'registrations':registrations,'source_files':source,'additional_native_ranges':extra,'initial_zero_initialized_pe_mapping':{hex(a):m.r(a) for a in [0xb3a66c,0xb3b4f4,0xb3b4f8]},'existing_numeric_evidence':{'path':str(fixture.relative_to(ROOT)),'sha256':sha(fixture.read_bytes()),'mean_cases':len(fd['mean_cases']),'definition_cases':len(fd['definition_cases']),'ability_cases':len(fd['cases']),'executed_again':False},'limits':['Hardcoded registration defaults are overwritten by sources: e.g. range0.1 becomes0.15; offense/defense1 become0.5. Never use registration defaults as effective source values.','Source-load PC remains explicit. Native main-thread context statically implies post-D3D precision; no original Windows/D3D runtime capture was made.','CSV UVar inspection covers shipped extracted files/overlay bytes, not an arbitrary existing user-profile variable state.','Positive Settlement radius depends on460940 exact trig/geometry and source object footprints, not raw Shape.wall_diameter.','No completed source arena, numeric adapter, runtime integration or Rust build is claimed by this audit.']}
(HERE/'source-dependencies.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print('pinned',len(registrations),'registered floats',len(source),'source files',len(extra),'native ranges')

