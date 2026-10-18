"""Native48D500 over the real terrain Variation registry/translation facts.

The original shared DataManager layout is constructed by4BD9C0; original
hashing/insertion/find execute, including source alias chains that stop after
one translation. Definition type loaders are outside this narrow lookup proof.
"""
from pathlib import Path
import gc,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'))
import native_definition_registry as registry
import native_machine as nm
import native_tgi_parser as parser
import trace_registry
from unicorn.x86_const import UC_X86_REG_FPCW
ORIGINAL=parser.P;V14=parser.V
def selection_witness():
 for _,r,size,off in ORIGINAL.sections:
  for i in range(size):
   va=ORIGINAL.base+r+i
   a=ORIGINAL.b[off+i:off+i+1];b=V14.range(va,1)
   if a!=b:return va,a,b
 raise AssertionError('independent PE images unexpectedly have no distinct section byte')
def main():
 source=json.loads((HERE/'source-metadata.json').read_text());maps=json.loads((HERE/'actual-map-inputs.json').read_text())['maps'];rows=[];visited={}
 witness_va,witness_original,witness_v14=selection_witness();selections=[]
 for image in ('original','v14'):
  selected=ORIGINAL if image=='original' else V14
  # trace_registry owns the base Machine PE-loading implementation. Changing
  # only a derived helper's global P would silently retain the original image.
  parser.P=nm.P=registry.P=trace_registry.P=selected
  for data in source['precisions']:
   pc=data['precision'];m=nm.Machine();m.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[pc]);manager=registry.make_thing_manager(m)
   expected=witness_original if image=='original' else witness_v14
   assert bytes(m.u.mem_read(witness_va,1))==expected
   selections.append(dict(image=image,precision=pc,va=hex(witness_va),loaded_byte=expected.hex()))
   definitions={item['id']:registry.register_def(m,manager,item['id']) for item in data['variations']};names={p:name for name,p in definitions.items()}
   for old,new in data['variation_translations']:registry.insert_translation(m,manager,old,new)
   queries={name for row in maps if row['precision']==pc for name in row['palette']}
   queries.update(definitions);queries.update(old for old,_ in data['variation_translations']);queries.add('')
   for name in sorted(queries):
    result=m.thiscall(0x48d500,manager,[m.cstring(name)])['eax'];m.check_null()
    assert m.u.reg_read(UC_X86_REG_FPCW)&0x300=={'Single24':0,'Double53':0x200,'Extended64':0x300}[pc]
    rows.append(dict(image=image,precision=pc,query=name,result=names.get(result) if result else None))
   visited.update(m.visited);del m;gc.collect()
 pins=[]
 for va,size in sorted(visited.items()):
  if not ORIGINAL.base<=va<ORIGINAL.base+ORIGINAL.image_size:continue
  raw=ORIGINAL.range(va,size);assert raw==V14.range(va,size),hex(va);pins.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope='Native48D500/4154D0/4915F0 over actual410 registered Variation identities and authored translation map, including one-hop chained misses',original_sha256=hashlib.sha256(ORIGINAL.b).hexdigest(),v14_sha256=hashlib.sha256(V14.b).hexdigest(),source_sha256=hashlib.sha256((HERE/'source-metadata.json').read_bytes()).hexdigest(),loaded_image_witnesses=selections,lookups=rows,instructions=pins)
 path=HERE/'native-source-find.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(lookups=len(rows),instructions=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
