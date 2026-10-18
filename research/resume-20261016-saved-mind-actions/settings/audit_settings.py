"""Export read-only PE/source evidence; no native execution or production edits."""
from pathlib import Path
import hashlib
import json
import runpy

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
GAME=ROOT.parent
helper=runpy.run_path(str(ROOT/'research/resume-20261016-element-attach/observer/native_observer_lifecycle.py'))
P,V=helper['SOURCES']['original'],helper['SOURCES']['v14']
capstone=helper['capstone']
sha=lambda raw:hashlib.sha256(raw).hexdigest()
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_32)
windows=[
 ('register_morale',0x5079D1,0x36),('register_fog',0x507CEF,0x37),
 ('float_variable_constructor',0x401710,182),('bounded_integer_constructor',0x401940,208),
 ('settings_index_add',0x4B3470,0x10D),('settings_copy',0x4B3580,0x70),
 ('startup_register',0x48DB60,0x30),('source_settings_owner',0x48F436,0x36),
 ('source_defaults_copy',0x4902A7,0xF),('world_reset',0x4F281C,0xF),
 ('element_read_order',0x530510,0xA4),('regiment_read_order',0x521E10,0x178),
 ('stack_head',0x529F70,0xD),('state_is_head',0x52A000,0x21),
 ('world_rules_update',0x512200,0x130),('world_rule_integer_update',0x512160,0x80),
 ('world_rules_load_and_merge',0x512420,0x270),('world_rules_default_reset',0x511CE0,0x88),
 ('world_rules_initialization',0x511D90,0x52),('world_rules_editable_reset',0x511640,0x5C),
 ('world_rules_locked_label',0x5114D0,0xD0),('world_rule_enum_resolve',0x473440,0x4D),
 ('world_rule_definition_load',0x4905E0,0x660),('world_rule_svar_defaults',0x474C50,0x300),
 ('gworld_rules_before_terrain',0x4FC600,0x330),
]
pins=[]
for name,start,size in windows:
 raw=P.bytes(start,size)
 assert raw==V.bytes(start,size),name
 pins.append(dict(name=name,start=f'{start:08x}',bytes=len(raw),sha256=sha(raw),original_v14_identical=True,assembly=[f'{i.address:08x}: {i.bytes.hex()} {i.mnemonic} {i.op_str}' for i in md.disasm(raw,start)]))
strings=[]
for name,va in [('FogOfWar',0xA2FCE8),('MoraleRetreatPenalty',0xA2FEA8)]:
 raw=P.bytes(va,(len(name)+1)*2)
 assert raw==V.bytes(va,len(raw))==name.encode('utf-16-le')+b'\0\0'
 strings.append(dict(name=name,address=f'{va:08x}',sha256=sha(raw),raw_hex=raw.hex()))
files=[]
for rel in ['reconstruction/assets/original/Data/Game/SVars.tgi','reconstruction/assets/original/Data/Game/SVars_AA.tgi','reconstruction/native-game/runtime-expanded-v14/Data/Game/SVars_AA.tgi','reconstruction/assets/original/Data/Game/world_rules_aa.tgi','reconstruction/assets/original/Data/Templates/template_world_rule.tgi']:
 path=GAME/rel;raw=path.read_bytes()
 lines=[dict(line=i+1,text=line) for i,line in enumerate(raw.decode('cp1252').splitlines()) if 'FogOfWar' in line or 'MoraleRetreatPenalty' in line or ('default = EXPLORED' in line)]
 files.append(dict(path=str(path),sha256=sha(raw),relevant_lines=lines))
fixtures=[]
for rel in ['research/phase7-mind/native-action-mask.json','research/phase7-mind/native_action_mask.py','crates/aa-mind/tests/action_mask_native.rs','research/phase7-definitions/native-settings-order.json']:
 path=ROOT/rel
 fixtures.append(dict(path=rel,sha256=sha(path.read_bytes()),executed_during_this_audit=False))
out=dict(scope='Static settings binding and native instruction/source audit. Separate native-fog-rules.json/native-morale-settings.json record execution.',sources={name:dict(path=str(p.path),sha256=sha(p.raw)) for name,p in helper['SOURCES'].items()},windows=pins,strings=strings,source_files=files,existing_fixtures=fixtures,bindings=[dict(method='setting_cache108',name='FogOfWar',index=66,offset=264,native_type=3,hardcoded_default=2,tgi_svar_default=2,post_world_rule_source_default=1,range=[0,2],stored_flags=32,conversion='current signed integer != 0',per_world_rule_override=True),dict(method='setting_cached8',name='MoraleRetreatPenalty',index=54,offset=216,native_type=2,hardcoded_default_bits='40a00000',source_default_bits='00000000',stored_flags=0,conversion='current binary32 bits')],limits=['No complete GWorld or Session execution.','Source definition LoadData4905E0 is statically inspected; definition/lookup seams are explicit in native oracle.','Existing Bool override oracle does not prove numeric assignments; the new numeric oracle executes setter only, not full TGI parsing.'])
(HERE/'settings-evidence.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(windows=len(pins),source_files=len(files),settings=2,native_execution=False)))
