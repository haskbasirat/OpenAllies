"""Read-only instruction-byte/decode audit, separate from oracle generation."""
from pathlib import Path
import json,hashlib,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(HERE));import native_audio_class as n
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
doc=json.loads((HERE/'native-audio-class.json').read_text());md=Cs(CS_ARCH_X86,CS_MODE_32)
decoded=[]
for address,encoded in doc['executed'].items():
    address=int(address,16);raw=bytes.fromhex(encoded)
    assert n.h.P.range(address,len(raw))==n.h.V.range(address,len(raw))==raw
    instruction=list(md.disasm(raw,address));assert len(instruction)==1 and instruction[0].size==len(raw)
    i=instruction[0];decoded.append(dict(address=hex(address),bytes=encoded,instruction=i.mnemonic+' '+i.op_str))
for entry in [0x9d02f0,0x409750,0x9d0900,0x59b010,0x59b020,0x59cb00,0x59c360,0x59c040]:
    assert hex(entry)in doc['executed'],hex(entry)
assert len(doc['rows'])==18
for row in doc['rows']:
    expected=10 if row['ram_current']is None else row['ram_current']
    assert row['ram_weight']==row['ram_total']==expected and row['vram_weight']==row['vram_total']==10
    assert row['class_count']==1 and row['lookup_count']==(1 if row['ram_current']is None else 2) and row['cached_lookup_count']==0
    assert row['resource_bytes_unchanged']and row['cached_same_allocation']
for row in doc['registrations']:
    assert row['descriptor']==dict(class_id=0,name='KKC_AudioResource',base='0xb38e24',factory='0x59b400',manager_count=1)
    assert row['extensions']==[dict(extension='WAV',descriptor='0xb3d76c'),dict(extension='MP3',descriptor='0xb3d76c')]
result=dict(schema=1,fixture_sha256=hashlib.sha256((HERE/'native-audio-class.json').read_bytes()).hexdigest(),
    executed_instruction_count=len(decoded),decoded=decoded,
    qualification='Actual native descriptor and class constructors in valid empty laboratory registries. Ordinal0 is relative to that fixture, never a claimed full application RTTI ordinal. Current Int and CRT formatting are declared transports; fixed fixture budget0 excludes eviction. Product intentionally removes fixed budgets. No Sound decode, Task completion, playback, World/View readiness, or full resource-cache eviction acceptance.')
path=HERE/'pin-audit.json';path.write_text(json.dumps(result,indent=1)+'\n')
print(json.dumps(dict(executed=len(decoded),paired_rows=len(doc['rows']),audit_sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
