from pathlib import Path
p=Path('research/development10/cai-arrivals/native_sleep_fields.py');s=p.read_text();needle="doc=dict(schema=1,scope=__doc__,rows=rows,getters=getters,pins=pins,excluded_entries="
replacement="""construct_path=ROOT/'research/phase7-mind/native-constructors.json';construct_raw=construct_path.read_bytes();construct=json.loads(construct_raw);metadata=[]
 for kind in ['base','element','regiment','aircraft']:
  row=next(r for r in construct['cases']if r['input']['kind']==kind);vt=int.from_bytes(bytes.fromhex(row['output']['initialized_hex'])[:4],'little');entry=OLD.u32(vt+0x78);assert OLD.range(vt+0x78,4)==V14.range(vt+0x78,4)
  expected={'base':0x96cac2,'element':0x535e70,'regiment':0x6a71f0,'aircraft':0x6a71f0}[kind];assert entry==expected
  metadata.append(dict(kind=kind,vtable=hex(vt),word_va=hex(vt+0x78),word_bytes=OLD.range(vt+0x78,4).hex(),entry=hex(entry)))
 doc=dict(schema=1,scope=__doc__,constructor_fixture_sha256=hashlib.sha256(construct_raw).hexdigest(),static_vtable_words=metadata,static_qualification='These vtable words are read-only image pins, not executed in the getter rows; constructor identities cite the separately executed full constructor fixture.',rows=rows,getters=getters,pins=pins,excluded_entries="""
assert needle in s;s=s.replace(needle,replacement);p.write_text(s)
