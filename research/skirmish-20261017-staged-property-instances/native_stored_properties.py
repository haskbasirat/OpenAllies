"""Unchanged native5817A0/581630 on captured moved stored-owner inputs.

Reuse the sealed constructor harness with two explicit successor changes:
numeric virtual30 executes the real4FE590/4FE260 (the admitted numeric rows
have actual Props-only Kingdoms), and the actual captured current Multiply
SVar cell supplies the unchanged50252F dereference. No successful unavailable
Performer/Settlement callback is inserted. Those remain native stop boundaries.
"""
from pathlib import Path
import sys,json,hashlib,types
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
SOURCE=ROOT/'research/skirmish-20261016-load-property-instances/native_property_instances.py'
old="m.w(vt+0x30,m.stub(numeric_stop));"
new="m.w(vt+0x30,0x4fe590 if v['kind']=='Add'else 0x4fe260);"
source=SOURCE.read_text();assert source.count(old)==1
legacy=types.ModuleType('stored_constructor_successor');legacy.__file__=str(SOURCE)
exec(compile(source.replace(old,new),str(SOURCE)+' [explicit real numeric dispatch successor]','exec'),legacy.__dict__)

def run(row,image,pc):
 if row['input']['phase']=='numeric':
  assert row['input']['native_class']==4 and row['input']['components']==[6]
 factory=nm.Machine
 def machine(p):
  m=factory(p);observe=m.observe
  def register(address,callback):
   if address==0x50252f:
    cache=m.alloc(0x200);cell=m.alloc(4);m.w(cell,row['input']['minimum_multiply'])
    m.w(cache+0x1a8,cell);m.w(0xb39ab0,cache)
   else:observe(address,callback)
  m.observe=register
  return m
 nm.Machine=machine
 try:return legacy.run(row,image,pc)
 finally:nm.Machine=factory

def main():
 inputs=json.loads((HERE/'actual-inputs.json').read_text());rows=[];pins={}
 for row in inputs['rows']:
  a,pa=run(row,'original',row['precision']);b,pb=run(row,'v14',row['precision']);assert a==b
  assert a['state']==row['output'],(row['input']['id'],row['input']['phase'],a['state'],row['output'])
  expected='performer'if row['error'] is not None else None
  assert a['boundary']==expected,(a['boundary'],row['error'])
  rows.append(dict(precision=row['precision'],input=row['input'],output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,legacy_source_sha256=hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
  explicit_dispatch_change=dict(before=old,after=new),rows=rows,native_executions=len(rows)*2,
  instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())])
 path=HERE/'native-stored-properties.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
