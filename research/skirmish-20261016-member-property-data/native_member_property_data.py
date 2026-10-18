"""Native4C4CF0 list stores and45EC60 cursor over explicit typed-source services.

The original/v14 bodies execute separately at PC24/53/64. Seams are child
count/selection/string getter, CString ownership helpers, property-registry
lookup and successful allocation. Their exact ordered calls are recorded;
this fixture does not claim native parsing or registry/Translate internals.
Unknown-property and missing-field stops retain the preceding native list.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
ORIGINAL=h.P
class Required(Exception):pass
class Machine(h.Machine):
 def __init__(self,c,image,pc):
  h.P=ORIGINAL if image=='original' else h.V;super().__init__(pc)
  self.c=c;self.events=[];self.current=-1
  self.actor=self.alloc(0x600);self.list=self.actor+0x2d8
  self.node=self.alloc(4);self.tag=self.alloc(4);self.string_helper=self.alloc(0x20);vt=self.alloc(0x20)
  self.string_record=self.alloc(0x80);self.w32(self.string_record+12,100)
  self.w32(self.string_helper,vt);self.w32(vt+12,h.STUBS)
  self.properties={handle:self.alloc(4) for handle in set(c['lookups'].values())|set(c['initial'])}
  self.reverse={address:handle for handle,address in self.properties.items()}
  self.w32(0xb36530,self.alloc(0x600))
  self.w32(self.r32(0xb36530)+0x464,self.alloc(4))
  for handle in reversed(c['initial']):self.prepend(self.properties[handle])
  self.seams.update({0x5b4220:lambda _:self.count(),0x856370:lambda _:self.ret(value=self.string_helper),
   h.STUBS:lambda _:self.ret(value=self.string_record),0x42be10:lambda _:self.select(),
   0x5b54d0:lambda _:self.get_ids(),0x405b80:lambda _:self.assign(),0x463fa0:lambda _:self.lookup(),
   0x6f7810:lambda _:self.allocation(),0x410c40:lambda _:self.unknown()})
 def prepend(self,value):
  node=self.alloc(8);self.w32(node,value,self.r32(self.list));self.w32(self.list,node)
  if self.r32(self.list+4)==0:self.w32(self.list+4,node)
 def count(self):
  self.events.append({'op':'count','value':self.c['count']});self.ret(pop=4,value=self.c['count'])
 def select(self):
  self.current+=1;self.events.append({'op':'ids','index':self.current});self.ret(pop=4,value=self.node)
 def get_ids(self):
  value=self.c['ids'][self.current]
  if value is None:raise Required('missing_field')
  encoded=value.encode('utf-16-le')+b'\0\0';header=self.alloc(16+len(encoded));self.w32(header+4,len(value));self.w32(header+12,100)
  self.u.mem_write(header+16,encoded);self.ret(value=header+16)
 def assign(self):self.w32(self.ecx(),self.arg());self.ret(pop=4,value=self.ecx())
 def text(self,p):
  out=bytearray()
  while (unit:=bytes(self.u.mem_read(p,2)))!=b'\0\0':out.extend(unit);p+=2
  return out.decode('utf-16-le')
 def lookup(self):
  ids=self.text(self.arg());self.events.append({'op':'lookup','ids':ids})
  handle=self.c['lookups'].get(ids);self.ret(pop=4,value=self.properties[handle] if handle else 0)
 def allocation(self):
  assert self.arg()==8;self.events.append({'op':'allocate','bytes':8});self.ret(value=self.alloc(8))
 def unknown(self):raise Required('unknown_property')
 def run(self):
  outcome='ok'
  try:self.call(0x4c4cf0,args=[self.node,self.tag,self.list])
  except Required as e:outcome=str(e)
  values=[];node=self.r32(self.list)
  while node:values.append(self.reverse[self.r32(node)]);node=self.r32(node+4)
  cursor=self.alloc(8);self.call(0x45ec60,self.actor,[cursor])
  assert self.r32(cursor)==self.list and self.r32(cursor+4)==self.r32(self.list)
  tail=self.r32(self.list+4)
  return {'outcome':outcome,'head_first':values,'tail_value':self.reverse[self.r32(tail)] if tail else None,'events':self.events}
def cases():
 base={'count':0,'ids':[],'lookups':{'a':1,'b':2,'alias':1},'initial':[]}
 def row(name,**kw):return {'name':name,**base,**kw}
 return [row('empty'),row('negative_count',count=-1),row('signed_min',count=-2147483648),
  row('prepend_duplicate_order',count=3,ids=['a','b','a']),
  row('alias_identity_duplicate',count=2,ids=['a','alias']),
  row('existing_tail',count=2,ids=['b','a'],initial=[2,1]),
  row('unknown_first',count=1,ids=['missing']),row('unknown_after_prefix',count=3,ids=['a','missing','b']),
  row('empty_ids',count=2,ids=['b','']),row('missing_first',count=1,ids=[None]),
  row('missing_after_prefix',count=3,ids=['b',None,'a']),
  row('captured_count',count=1,ids=['a','missing'])]
def main():
 rows=[]
 for c in cases():
  for image in ['original','v14']:
   for pc in h.PRECISION:rows.append({'image':image,'precision':pc,'input':c,'output':Machine(c,image,pc).run()})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items())],'cases':rows}
 path=HERE/'native-member-property-data.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'cases':len(rows),'instructions':len(h.VISITED),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
