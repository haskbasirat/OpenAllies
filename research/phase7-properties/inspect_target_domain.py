"""Original instruction-pinned predefined TargetType order, not TGI sort order."""
from pathlib import Path
import csv,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
sizes={int(r['address'],16):int(r['bodyBytes']) for r in csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t')}
ranges=[]
for a in [0x48a8e0,0x48db60,0x478c20,0x4b4e60,0x4853e0,0x47d880,0x485540,0x485600]:
 raw=IMAGE.at(a,sizes[a]);assert raw==ACCEPTED.at(a,len(raw));ranges.append({'address':hex(a),'size':len(raw),'sha256':hashlib.sha256(raw).hexdigest()})
 (HERE/f'target-domain-{a:08x}.asm.txt').write_text('\n'.join(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,a))+'\n')
rows=[];stack=[]
for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(IMAGE.at(0x48db90,0x48dc6f-0x48db90),0x48db90):
 if i.mnemonic=='push':stack.append(0 if i.op_str=='ebx' else int(i.op_str,0))
 if i.mnemonic=='call':
  assert i.op_str=='0x478c20' and len(stack)==2
  address,index=stack;stack=[];raw=bytearray();offset=0
  while True:
   pair=IMAGE.at(address+offset,2);assert pair==ACCEPTED.at(address+offset,2);raw.extend(pair);offset+=2
   if pair==b'\0\0':break
  rows.append({'index':index,'id':raw[:-2].decode('utf-16le'),'call_address':hex(i.address),'string_address':hex(address),'string_sha256':hashlib.sha256(raw).hexdigest()})
assert [r['index'] for r in rows]==list(range(16))
literal='TargetType\0'.encode('utf-16le');assert IMAGE.at(0xa0ea10,len(literal))==ACCEPTED.at(0xa0ea10,len(literal))==literal
doc={'schema':1,'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':sha(Path(__file__)),'native_ranges':ranges,'predefined':rows,'evidence':['48DB90..48DC6F registers the16 predefined TargetTypes in exact index order before loading data files.','478C20 creates474AC0 then inserts4B4E60 and checks the resulting definition+C equals the requested predefined index.','48DC78..48DC82 asserts Data+3B0 registry count+28 equals16.','48DB60 later registers Data+3B0 under UTF16 TargetType atA0EA10.', '48A8E0 constructs4853E0(13,1) and stores it atData+3B0;47D880 writes policy1 into registry+8.', 'TargetType vtableA0D2DC+1C invokes485540. Its485580..4855A8 nonzero-policy branch searches existing IDS through4915F0 and reuses an existing definition when found, skipping485600 creation; then4855B8 calls that definition loader. Missing IDS creates through485600 and appends. Thus predefined-file entries retain their existing16 indices.'],'boundary':'TGI additions and override/file order remain the aa-rules adapter owner scope; do not assign all target indices by alphabetic or ActorCatalog order.'}
path=HERE/'target-domain-evidence.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'sha256':sha(path),'predefined':rows}))
