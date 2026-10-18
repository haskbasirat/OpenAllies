"""Actual5A1120 existing-entry lookup and4B9E60 prepend/tail/reference
effects over real4BAD50 +60 nil and parsed TextData entries. Missing keys
stop before5A1175's required warning/create graph; that graph is not replaced
by success. Actual parser/allocator/CString transports reuse documented
native_mapstrings seams. Logical keys normalize native pointers only in output.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'))
from native_mapstrings import MapStrings,L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stopped(Exception):pass
RAW=b'[Text language=Default]\n{\nids=map_strings_table\nname="Map"\na="Alpha"\nb="Beta"\n}\n'
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=MapStrings();m.u.reg_write(UC_X86_REG_FPCW,PC[pc])
 manager,entries=m.load(RAW);assert manager
 native={}
 #Native lookup returns the exact manager entry, not a text-value snapshot.
 for key in ['a','b']:
  pair=m.alloc(8);m.w(pair,m.cstring(key));m.w(pair+4,m.cstring('unused'))
  dest=m.alloc(4);m.call(0x4baa20,manager,[dest,pair,0,pair+4]);entry=m.r(dest);assert entry;native[entry]=key
  m.w(entry+4,m.r(entry+4)-1)
 native[m.r(manager+0x60)]='';baseline={p:m.r(p+4)for p in native}
 resource=m.alloc(0xb4);steps=[];stopped=None
 def missing(u,at,size,_):raise Stopped('required_warning_create')
 m.u.hook_add(UC_HOOK_CODE,missing,begin=0x5a1175,end=0x5a1175)
 def snapshot():
  out=[];node=m.r(resource+0x7c);last=0
  while node:
   out.append(native[m.r(node)]);last=node;node=m.r(node+4)
  assert m.r(resource+0x80)==last
  return dict(head_first=out,reference_deltas={native[p]:m.r(p+4)-baseline[p]for p in native})
 for key in case['keys']:
  pair=m.alloc(8);m.w(pair,m.cstring(key));m.w(pair+4,m.cstring('THIS MUST NOT REPLACE EXISTING TEXT'))
  try:m.call(0x5a1120,resource,[pair])
  except Stopped as e:stopped=str(e);break
  steps.append(snapshot())
 out=dict(steps=steps,stopped=stopped,final=snapshot())
 assert (m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 return out,m.windows()
def cases():
 for keys in [[],['a'],[''],['a','A','b','','a',''],['a','missing'],['missing'],['a']*40]:yield dict(keys=keys)
def main():
 rows=[];blocks={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   rows.append(dict(input=case,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 doc=dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-references.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
