"""Two separate native Fixed variables: Add index69/+1A4, Multiply6A/+1A8.

Original401710 constructor,4B3580 reset and40E0A0 raw current-cell writes execute
using the existing bounded named-lookup transport. Original5086FE..508783 source
registration pushes are pinned separately; this is not a full App constructor.
"""
from pathlib import Path
import sys,json,hashlib,importlib.util,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
spec=importlib.util.spec_from_file_location('current_vars',ROOT/'research/skirmish-20261017-performer-value/native_current_combat_vars.py')
v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)
NAMES=['ModifierAttributeClampAdd','ModifierAttributeClampMultiply'];SOURCE=[0x3f800000,0x3d4ccccd]
v.NAMES=NAMES
def main():
 rows=[];seams=set()
 for pc in v.h.PRECISION:
  for later in [SOURCE,[0x3f000000,0x3f666666],[0x80000000,0x7fc01234],[0x7f800000,0xbf800000]]:
   c=dict(source=SOURCE,later=later)
   a,s=v.run('original',pc,c);b,t=v.run('v14',pc,c);assert a==b
   assert a['reset']==SOURCE and a['current']==later
   assert a['constructed']==[dict(value=n,kind=2,flags=0)for n in SOURCE]
   rows.append(dict(precision=pc,input=c,output=a));seams|=s|t
 static=[]
 for address,size in [(0x5086fe,0x86),(0xa2f630,0x40),(0xa2f670,0x34)]:
  # Original PE helper exposes bytes through get(); use native_machine's
  # separately SHA-verified reader rather than assume another helper API.
  sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
  raw=nm.original.range(address,size);assert raw==nm.modified.range(address,size)
  static.append(dict(va=hex(address),bytes=raw.hex()))
 for address,index,name,default in [(0x5086fe,0x69,NAMES[0],SOURCE[0]),(0x508741,0x6a,NAMES[1],SOURCE[1])]:
  raw=nm.original.range(address,0x43)
  assert raw[:7]==b'\x6a\x00\x68'+struct.pack('<I',default)
  assert b'\x6a'+bytes([index])in raw
  name_va=0xa2f670 if index==0x69 else 0xa2f630
  assert b'\x68'+struct.pack('<I',name_va)in raw
  assert nm.original.range(name_va,len(name.encode('utf-16le'))+2)==name.encode('utf-16le')+b'\0\0'
 pins={f'{a:08x}':raw for a,raw in sorted(v.h.VISITED.items())if a not in seams}
 doc=dict(schema=1,scope=__doc__,names=NAMES,source=SOURCE,registration=[dict(name=n,index=i,cache_offset=i*4)for n,i in zip(NAMES,[0x69,0x6a])],rows=rows,native_executions=len(rows)*2,source_windows=static,executed_instruction_pins=pins,seams=[hex(x)for x in sorted(seams)])
 path=HERE/'native-modifier-variables.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
