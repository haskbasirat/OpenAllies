"""Native header7/WorldRules4/WorldParameters13/WorldCreator3 writers.
Real field loops, low16 CString truncation and49AA30 key-only calls execute.
Only raw stream writes and reached5A1120 text-entry collection are seams.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P

def sample(seed):
 text=lambda s:[ord(v)for v in s]
 rule=lambda k,kind,v:dict(key=text(k),enabled=seed+1,kind=kind,value=v)
 rules=dict(version=4,mode=seed%5,game_type=text('NORMAL'),rules=[rule('BOOL',0,0x81),rule('ENUM',1,0xffffffff),rule('FIXED',2,0x7f812345),rule('INTEGER',3,0x80000000)] if seed else [])
 teams=[dict(id=text('team'),display_key=text('team_name'))] if seed else []
 kingdom=dict(id=text('kingdom'),byte_10=seed,nation=text('USA'),faction=text('ALLIES'),color=text('BLUE'),handicap=text('NORMAL'),display_key=text('kingdom_name'),string_0c=text('control'),string_2c=text('player'),byte_29=0x82,byte_28=0x41,string_14=text('tag'),localized_entries=[text('localA'),text('localB')],definitions=[text('definitionA'),[],text('definitionB')],string_58=text('script'),properties=[text('propertyA'),text('propertyB')],byte_31=0xfe,byte_32=0x80,resources=[(text('MONEY'),0x7f812345),(text('OIL'),0x80000000)])
 params=dict(version=13,teams=teams,kingdoms=[kingdom] if seed else [])
 creator=dict(class_name=text('KKC_WorldCreator'),version=3,rules=rules,parameters=params,world_extents_bits=[0x7fc12345,0x80000000],biome=text('grass'),weather=text('rain'),rng_seed=0xdeadbeef)
 header=dict(source=dict(sha256='',bytes=0),version=7,rules=rules,parameters=params,words_0c_10=[0x1020304,0x99887766],name_key=text('map_name'),description_key=text('map_description'),display_extents_bits=[0x80000000,0x7fc00001],word_50=0x11223344,word_54=0xaabbccdd,string_58=[0,0xd800,0xdcff],string_5c=text('build'),string_60=[],byte_64=255,string_38=text('recording'),string_3c=text('scene'),word_40=0xffffffff,words_18_1c=[0x12345678,0x9abcdef0],entries=[dict(flag=1,string_20=text('display'),string_18=text('kingdom'),flag_v5=1),dict(flag=0,string_20=[],string_18=text('other'),flag_v5=0)] if seed else [],creator_offset=0,creator=creator)
 if seed==2:header['string_5c']=[65]*65537
 return header

def run(header,image,pc,load_source=None,pair_texts=None,parameters_only=False,stop=None):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 resource=m.alloc(0xb4);stream=resource+0x3c;vt=m.alloc(0x50);m.w32(stream,vt)
 events=[];payload=[];registered=[]
 def string(value):
  p=m.alloc(16+2*(len(value)+1));m.w32(p+4,len(value));m.w32(p+12,2)
  m.u.mem_write(p+16,b''.join(int(v).to_bytes(2,'little')for v in value)+b'\0\0');return p+16
 empty=string([]);m.w32(0xb43b40,empty)
 def definition(value):
  if not value:return 0
  p=m.alloc(0x60);m.w32(p+8,string(value));return p
 def rules(value,p=None):
  p=m.alloc(0x30)if p is None else p;m.w32(p,0xa31990,value['mode'],definition(value['game_type']))
  rows=value['rules'];array=m.alloc(12*len(rows));m.w32(p+12,array,len(rows))
  for i,row in enumerate(rows):
   d=definition(row['key']);m.w32(d+0x20,row['kind']);m.w32(array+12*i,d,row['enabled'],row['value'])
  return p
 manager=m.alloc(0x100);m.w32(0xb36530,manager)
 def parameters(value,p=None):
  p=m.alloc(0x30)if p is None else p;m.w32(p,0xa317a8)
  teams=value['teams'];arr=m.alloc(len(teams)*12);m.w32(p+4,arr,len(teams))
  for i,row in enumerate(teams):m.w32(arr+12*i,string(row['id']),string(row['display_key']),empty if pair_texts is None else string(pair_texts['teams'][i]))
  kingdoms=value['kingdoms'];arr=m.alloc(len(kingdoms)*0x6c);m.w32(p+16,arr,len(kingdoms))
  for i,row in enumerate(kingdoms):
   dest=arr+i*0x6c
   m.w32(dest+8,empty if pair_texts is None else string(pair_texts['kingdoms'][i]['text']))
   for off,key in [(0,'id'),(4,'display_key'),(12,'string_0c'),(20,'string_14'),(44,'string_2c'),(88,'string_58')]:m.w32(dest+off,string(row[key]))
   for off,key in [(24,'nation'),(28,'faction'),(32,'color'),(36,'handicap')]:m.w32(dest+off,definition(row[key]))
   for off,key in [(16,'byte_10'),(40,'byte_28'),(41,'byte_29'),(49,'byte_31'),(50,'byte_32')]:m.w8(dest+off,row[key])
   rows=row['localized_entries'];items=m.alloc(8*len(rows));m.w32(dest+64,items,len(rows))
   for j,v in enumerate(rows):m.w32(items+8*j,string(v),empty if pair_texts is None else string(pair_texts['kingdoms'][i]['entries'][j]))
   for off,key in [(76,'definitions'),(92,'properties')]:
    rows=row[key];items=m.alloc(4*len(rows));m.w32(dest+off,items,len(rows))
    for j,v in enumerate(rows):m.w32(items+4*j,definition(v))
   rows=row['resources'];items=m.alloc(4*len(rows));ids=m.alloc(4*len(rows));m.w32(dest+52,items,len(rows));m.w32(manager+52,ids)
   for j,(key,bits)in enumerate(rows):m.w32(items+4*j,bits);m.w32(ids+4*j,definition(key))
  return p
 obj=m.alloc(0x68);m.w32(obj+4,rules(header['rules']),parameters(header['parameters']))
 for off,key in [(12,'words_0c_10'),(24,'words_18_1c'),(48,'display_extents_bits')]:m.w32(obj+off,*header[key])
 for off,key in [(32,'name_key'),(40,'description_key')]:m.w32(obj+off,string(header[key]),empty)
 for off,key in [(56,'string_38'),(60,'string_3c'),(88,'string_58'),(92,'string_5c'),(96,'string_60')]:m.w32(obj+off,string(header[key]))
 for off,key in [(64,'word_40'),(80,'word_50'),(84,'word_54')]:m.w32(obj+off,header[key])
 m.w8(obj+100,header['byte_64']);rows=header['entries'];arr=m.alloc(12*len(rows));m.w32(obj+68,arr,len(rows))
 for i,row in enumerate(rows):
  p=arr+12*i;m.w32(p,string(row['string_18']));m.w8(p+4,row['flag']);m.w8(p+5,row['flag_v5']);m.w32(p+8,string(row['string_20']))
 creator=m.alloc(0x60);value=header['creator'];rules(value['rules'],creator+32);parameters(value['parameters'],creator+4)
 m.w32(creator+60,*value['world_extents_bits']);m.w32(creator+68,definition(value['biome']),definition(value['weather']),value['rng_seed'])
 def write(_):
  assert m.ecx()==stream
  if stop==len(events):raise InterruptedError('write')
  raw=bytes(m.u.mem_read(m.arg(),m.arg(1)));payload.append(raw);events.append(dict(write=raw.hex()));m.ret(8)
 stub=m.stub_next;m.stub_next+=16;m.u.mem_write(stub,b'\xc3');m.seams[stub]=write
 m.w32(vt+4,stub);m.w32(vt+12,0x6a71d0);m.w32(vt+0x48,0x6a71d0)
 def register(_):
  assert m.ecx()==resource
  if stop==len(events):raise InterruptedError('register')
  ptr=m.r32(m.arg());length=m.r32(ptr-12);units=list(int.from_bytes(bytes(m.u.mem_read(ptr+2*j,2)),'little')for j in range(length))
  registered.append(units);event=dict(register=units)
  if pair_texts is not None:
   ptr=m.r32(m.arg()+4);length=m.r32(ptr-12);event['text']=[int.from_bytes(bytes(m.u.mem_read(ptr+2*j,2)),'little')for j in range(length)]
  events.append(event);m.ret(4)
 m.seams[0x5a1120]=register
 if parameters_only:
  stopped=None
  try:m.call(0x50d730,m.r32(obj+8),[stream])
  except InterruptedError as e:stopped=str(e)
  return dict(events=events,payload_hex=b''.join(payload).hex(),registered=registered,stopped=stopped)
 m.call(0x59e7e0,obj,[stream]);assert m.final_esp==h.STACK+8 and m.fpu_top()==0
 prefix=len(b''.join(payload))
 #59E530's class CString pair itself is the shared41CF30 wire primitive.
 pair=m.alloc(4);m.w32(pair,string(value['class_name']))
 m.call(0x41cf30,0,[stream,pair]);assert m.final_esp==h.STACK+4
 if load_source is not None:
  source=0
  if load_source['present']:
   source=m.alloc(0x40);m.w32(source+0x38,string(load_source['ids']))
  m.w32(creator+0x50,source)
 m.call(0x512a00 if load_source is None else 0x512c30,creator,[stream]);assert m.final_esp==h.STACK+8 and m.fpu_top()==0
 return dict(events=events,payload_hex=b''.join(payload).hex(),header_bytes=prefix,registered=registered)

def main():
 rows=[]
 for seed in range(3):
  header=sample(seed)
  for pc in h.PRECISION:
   a=run(header,'original',pc);b=run(header,'v14',pc);assert a==b
   rows.append(dict(input=header,pc=pc,output=a))
 doc=dict(schema=1,scope=__doc__,cases=rows,instructions=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-header.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(h.VISITED),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
