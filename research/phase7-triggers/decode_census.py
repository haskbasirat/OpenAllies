"""Research decoder for current v5/v4/v3/v4 trigger records.

Field layout follows native54D3D0/541410/5468A0/54A640 and their writers.
The decoded records are inventory, not executable world state or parity proof.
"""
from pathlib import Path
import collections,hashlib,json,struct,zlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
registry=json.loads((HERE/'native-registrations.json').read_text())
items={r['name']:r for r in registry['items']}
elements={r['name']:r for r in registry['elements']}
class Reader:
 def __init__(self,b):self.b=b;self.at=0
 def read(self,n):
  if n<0 or self.at+n>len(self.b):raise ValueError(f'truncated at{self.at}:need{n}')
  b=self.b[self.at:self.at+n];self.at+=n;return b
 def u32(self):return struct.unpack('<I',self.read(4))[0]
 def u16(self):return struct.unpack('<H',self.read(2))[0]
 def u8(self):return self.read(1)[0]
 def text(self):return self.read(self.u16()*2).decode('utf-16le')
 def param(self):
  begin=self.at;version=self.u32();assert version==4,(begin,version)
  argument=self.text();name=self.text();flag=self.u8()
  # Native aliases; no unknown item is silently classified.
  key={'mi_ui_button':'mi_script_command','mi_actor_data':'mi_actor_data_named'}.get(name,name)
  item=items[key];storage=item['storage'];payload=None
  if storage in [1,2]:payload=self.u32()
  elif storage==5:payload=[self.u32() for _ in range(3)]
  elif storage==6:payload=self.text()
  elif storage==7:
   # Current original RMP stores localized IDS with external MAPSTRINGS.
   # Native49AD70 selects this layout from stream+48/59E520 context; saved games
   # may include an inline second string and require a different explicit mode.
   first=self.text();payload={'key':first,'inline':None}
  elif storage!=0:raise ValueError((begin,key,storage))
  return {'span':[begin,self.at-begin],'version':version,'argument':argument,'item':name,'item_index':item['index'],'storage':storage,'flag':flag,'value':payload}
 def element(self):
  begin=self.at;version=self.u32();assert version==3,(begin,version)
  name=self.text();word=self.u32();count=self.u32();assert count<100,(begin,count)
  params=[self.param() for _ in range(count)]
  return {'span':[begin,self.at-begin],'version':version,'name':name,'opcode':elements[name]['index'],'word_10':word,'parameters':params}
 def trigger(self):
  begin=self.at;version=self.u32();assert version==4,(begin,version)
  id=self.u32();mode=self.u32();enabled=self.u8();repeat=self.u8();activation=self.u32();name=self.text();description=self.text();word=self.u32()
  lists={}
  for label in ['events','conditions','actions']:
   count=self.u32();assert count<1000,(begin,count);lists[label]=[self.element() for _ in range(count)]
  return {'span':[begin,self.at-begin],'version':version,'id':id,'mode':mode,'enabled':enabled,'repeat':repeat,'activation_bits':activation,'name':name,'description':description,'word_2c':word,**lists}
 def manager(self):
  version=self.u32();assert version==5,version
  next_id=self.u32();count=self.u32();triggers=[self.trigger() for _ in range(count)]
  variables=[{'name':self.text(),'bits':self.u32()} for _ in range(self.u32())]
  strings=[self.text() for _ in range(self.u32())]
  assert self.at==len(self.b),(self.at,len(self.b))
  return {'version':version,'next_id':next_id,'triggers':triggers,'variables':variables,'strings':strings}
def chunk(path,name):
 b=path.read_bytes();assert b[:8]==b'TGCK\x02\0\0\0';count=struct.unpack_from('<I',b,8)[0]
 for i in range(count):
  e=b[len(b)-count*96+i*96:len(b)-count*96+(i+1)*96]
  if e[:64].decode('utf-16le').split('\0')[0]!=name:continue
  offset,length,_,size=struct.unpack_from('<4Q',e,64);packed=b[offset:offset+length]
  try:data=zlib.decompress(packed)
  except zlib.error:data=packed
  assert len(data)==size
  return data,{'map_sha256':hashlib.sha256(b).hexdigest(),'chunk_sha256':hashlib.sha256(data).hexdigest(),'offset':offset,'packed_length':length,'decoded_length':size}
 raise ValueError(name)
def main():
 summaries={}
 for name in ['tutorial01','tutorial02']:
  path=ROOT.parent/f'reconstruction/assets/original/Data/Maps/Campaign/{name}.RMP';b,pins=chunk(path,'TRIGGERS');value=Reader(b).manager();counts=collections.Counter()
  for t in value['triggers']:
   for label in ['events','conditions','actions']:counts.update(e['name'] for e in t[label])
  report={'source':pins,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'registration_sha256':hashlib.sha256((HERE/'native-registrations.json').read_bytes()).hexdigest(),'scope':'Decoded source inventory; executable support and native writer validation are separate work.','manager':value}
  (HERE/f'{name}-decoded.json').write_text(json.dumps(report,indent=2)+'\n')
  summaries[name]={'triggers':len(value['triggers']),'elements':dict(counts),'initial_enabled':sum(bool(t['enabled']) for t in value['triggers']),'first':value['triggers'][0]}
 (HERE/'tutorial-census.json').write_text(json.dumps(summaries,indent=2)+'\n');print(json.dumps({k:{'triggers':v['triggers'],'element_kinds':len(v['elements']),'initial_enabled':v['initial_enabled']} for k,v in summaries.items()}))
if __name__=='__main__':main()
