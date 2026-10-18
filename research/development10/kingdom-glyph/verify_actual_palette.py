"""Compare captured actual registered Color/Glyph source outputs to96C7EC.
This is a source/formatter differential, not a completed live4E93C0 film call.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_kingdom_glyph import GlyphMachine,PE0,PE1,PC
source=HERE/'actual-palette-input.json'
actual=json.loads(source.read_text());rows=[];pins={}
for image in ['original','v14']:
 m=GlyphMachine(image);m.record_actual=True
 for pc in PC:
  m.fpu(pc)
  for entry in actual['palette']:
   floats=[float(struct.unpack('<f',struct.pack('<I',b))[0])for b in entry['rgb']]
   output=m.fmt('<color=%g,%g,%g>%c',struct.pack('<3dI',*floats,actual['glyph']))
   assert output==entry['text'],entry
   rows.append({'image':image,'precision':pc,'id':entry['id'],'rgb':entry['rgb'],'text':output})
 pins.update(m.actual)
instructions=[]
for va,size in sorted(pins.items()):
 if PE0.base<=va<PE0.base+PE0.image_size:
  raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append({'va':hex(va),'bytes':raw.hex()})
data=[]
for va,value in [(0xa299d8+0x24,0x4e5340),(0xa0b744,0x47e0f0)]:
 raw=PE0.range(va,4);assert raw==PE1.range(va,4)==struct.pack('<I',value)
 data.append({'va':hex(va),'bytes':raw.hex(),'target':hex(value)})
doc={'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'input_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'source_precision':actual['source_precision'],'palette_count':len(actual['palette']),'rows':rows,'instructions':instructions,'data_pins':data}
p=HERE/'native-actual-palette.json'
if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
else:assert json.loads(p.read_text())==doc
print(json.dumps({'palette':len(actual['palette']),'rows':len(rows),'pins':len(instructions),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
