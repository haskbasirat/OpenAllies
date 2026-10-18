"""Original96C7EC wide%c including WEOF failure; fresh4E93C0 cache PE-zero.
CString storage is not executed here. Each CRT case has a fresh call in each
original/v14 machine at all three PC modes;9738A9 supplies C-locale thread data.
"""
from pathlib import Path
import sys,json,struct,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_kingdom_glyph import GlyphMachine,PE0,PE1,PC,UC_X86_REG_EAX
rows=[];pins={};globals=[]
for image,pe in [('original',PE0),('v14',PE1)]:
 m=GlyphMachine(image);m.record_actual=True
 for pc in PC:
  m.fpu(pc)
  for char in [0,65,128,132,140,0x3a9,0xd800,0xdfff,0xfffe,0xffff]:
   error=False
   try:units=m.fmt('%c',struct.pack('<I',char))
   except AssertionError:
    assert m.u.reg_read(UC_X86_REG_EAX)==0xffffffff;error=True;units=None
   rows.append({'image':image,'precision':pc,'char':char,'text':units,'return':0xffffffff if error else len(units)})
 pins.update(m.actual)
 for va in [0xb3cbf4,0xb3cbf0]:
  rva=va-pe.base
  vs,section,raw,offset=next(s for s in pe.sections if s[1]<=rva<s[1]+s[0]);within=rva-section
  value=pe.b[offset+within:offset+within+4]if within+4<=raw else bytes(4)
  assert value==bytes(4)
  globals.append({'image':image,'va':hex(va),'initial_word':0,'section_rva':section,'virtual_size':vs,'raw_size':raw,'within_section':within,'initialized_raw_bytes':within+4<=raw})
a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
instructions=[]
for va,size in sorted(pins.items()):
 if PE0.base<=va<PE0.base+PE0.image_size:
  raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append({'va':hex(va),'bytes':raw.hex()})
doc={'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rows':rows,'global_image':globals,'instructions':instructions}
p=HERE/'native-glyph-character.json'
if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
else:assert json.loads(p.read_text())==doc
print(json.dumps({'rows':len(rows),'pins':len(instructions),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
