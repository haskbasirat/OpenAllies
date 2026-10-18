"""Native standalone WorldRules vector defaults, typed saved overlay and editable reset.
Actual511CE0/511C30,512420,511640 and511A60 run unchanged. The already-executed
511CE0 vector is retained across the explicit511D90 reset seam; this is not a
complete source TGI/WorldRules constructor proof. Stream reads/version/CString
allocation, validated GameType transport, typed current registry Find and log
transport are explicit seams. No unknown descriptor/default is fabricated.
"""
from pathlib import Path
import sys,json,struct,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P

def wire(rows,game_type):
 g=game_type.encode('utf-16le');b=bytearray(struct.pack('<IIH',4,0,len(g)//2)+g+struct.pack('<I',len(rows)))
 for r in rows:
  s=r['ids'].encode('utf-16le');b+=struct.pack('<H',len(s)//2)+s+bytes([r['locked']])+struct.pack('<I',r['kind'])
  b+=struct.pack('<B'if r['kind']==0 else'<I',r['value'])
 return b

def run(c,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);seams=set();events=[]
 def seam(a,f):m.seams[a]=f;seams.add(a)
 def string(s):
  header=m.alloc(0x1100);m.w32(header+8,2048);m.w32(header+12,100);p=header+16
  m.u.mem_write(p,s.encode('utf-16le')+b'\0\0');m.w32(header+4,len(s));return p
 def text(p):
  b=bytearray()
  while True:
   u=bytes(m.u.mem_read(p,2));p+=2
   if u==b'\0\0':break
   b+=u
  return b.decode('utf-16le')
 seam(0x96cad6,lambda _:m.ret(value=0 if text(m.arg()).lower()==text(m.arg(1)).lower() else 1)) # ASCII source IDS compare transport.
 defs=[];manager=m.alloc(0x600);array=m.alloc(4*len(c['definitions']));m.w32(0xb36530,manager)
 m.w32(manager+0x118,array,len(c['definitions']))
 for i,d in enumerate(c['definitions']):
  ptr=m.alloc(0x60);m.w32(ptr+8,string(d['ids']));m.w32(ptr+12,i);m.w32(ptr+0x20,d['kind'])
  m.w8(ptr+0x24,d['editable']);m.w32(ptr+0x40,d['default']);m.w32(array+4*i,ptr);defs.append(ptr)
 world=m.alloc(0x30);entries=m.alloc(max(12,12*len(defs)));m.w32(world+12,entries,0,len(defs))
 m.call(0x511ce0,world)
 def values():
  out=[]
  assert m.r32(world+0x10)==len(defs)
  for i,d in enumerate(c['definitions']):
   p=entries+12*i;assert m.r32(p)==defs[i]
   out.append({'ids':d['ids'],'locked':m.r8(p+4),'value':m.r32(p+8)&(255 if d['kind']==0 else 0xffffffff)})
  return out
 defaults=values();data=wire(c['saved'],c.get('game_type',''));cursor=0
 stream=m.alloc(16);vt=m.alloc(16);stub=m.stub_next;m.stub_next+=16;m.w32(stream,vt);m.w32(vt,stub)
 def read(_):
  nonlocal cursor
  target,n=m.arg(0),m.arg(1);m.u.mem_write(target,bytes(data[cursor:cursor+n]));cursor+=n;m.ret(8)
 seam(stub,read)
 def version(_):
  nonlocal cursor
  assert m.arg(0)==stream and m.arg(1)==4
  m.w32(m.ecx(),struct.unpack_from('<I',data,cursor)[0]);cursor+=4;m.ret(8)
 seam(0x8589b0,version)
 def game(_):
  nonlocal cursor
  n=struct.unpack_from('<H',data,cursor)[0];cursor+=2;name=data[cursor:cursor+n*2].decode('utf-16le');cursor+=n*2
  assert name==c.get('game_type','');m.w32(m.arg(1),0 if not name else m.alloc(16));m.ret()
 seam(0x512360,game)
 # This is exactly the actual vector initialized above; no fake generic defaults.
 seam(0x511d90,lambda _:m.ret())
 allocator=m.alloc(16);avt=m.alloc(16);allocstub=m.stub_next;m.stub_next+=16;m.w32(allocator,avt);m.w32(avt+12,allocstub)
 seam(0x856370,lambda _:m.ret(value=allocator))
 def alloc(_):
  p=string('')-16;m.ret(value=p)
 seam(allocstub,alloc)
 seam(0x401330,lambda _:m.ret(4)) # Preallocated CString resize transport.
 def find(_):
  name=text(m.arg());translated=c.get('translations',{}).get(name.lower(),name).lower()
  match=next((p for p,d in zip(defs,c['definitions'])if d['ids'].lower()==translated),0)
  events.append(['Find',name,next((d['ids']for p,d in zip(defs,c['definitions'])if p==match),None)])
  m.ret(4,match)
 seam(0x470470,find);seam(0x404f90,lambda _:m.ret())
 m.call(0x512420,world,[stream]);assert cursor==len(data),(cursor,len(data))
 loaded=values()
 if c['load_mode']&2:m.call(0x511640,world)
 final=values()
 # Actual511A60 uses first current descriptor IDS and does not Translate.
 cell=m.alloc(4);lookup={}
 for name in c['find_names']:
  m.w32(cell,string(name));r=m.call(0x511a60,world,[cell]);lookup[name]=None if r==0 else (r-entries)//12
 return {'defaults':defaults,'loaded':loaded,'current':final,'find':lookup,'events':events},seams

def cases():
 base=[{'ids':'Bool','kind':0,'default':0xaabbcc80,'editable':1},
  {'ids':'CityVictoryRatio','kind':1,'default':0,'editable':0},
  {'ids':'ResourceStockpileDefault','kind':2,'default':h.f2b(500),'editable':0},
  {'ids':'Sun','kind':3,'default':0,'editable':1}]
 for mode in (1,2,4):
  for locked in (0,255):
   yield {'definitions':base,'load_mode':mode,'translations':{'oldcity':'CityVictoryRatio'},
    'saved':[{'ids':'Unknown','kind':2,'value':55,'locked':locked},
      {'ids':'CityVictoryRatio','kind':2,'value':77,'locked':locked},
      {'ids':'oldcity','kind':1,'value':42,'locked':locked},
      {'ids':'CITYVICTORYRATIO','kind':1,'value':0x80000001,'locked':locked},
      {'ids':'Bool','kind':0,'value':129,'locked':locked},
      {'ids':'ResourceStockpileDefault','kind':2,'value':0x7fc01234,'locked':locked},
      {'ids':'Sun','kind':3,'value':3,'locked':locked}],
    'find_names':['CityVictoryRatio','cityvictoryratio','oldcity','Unknown','Sun']}
 actual=HERE/'actual-world-rules.json'
 if actual.exists():
  sys.path.insert(0,str(ROOT/'research/phase8-conversion'));from rmp import Rmp,MAPS
  for row in json.loads(actual.read_text())['cases']:
   data=Rmp(MAPS/(row['map']+'.RMP')).chunk('RULES');assert hashlib.sha256(data).hexdigest()==row['rules_sha256'];at=8
   def text():
    nonlocal at
    n=struct.unpack_from('<H',data,at)[0];at+=2;s=data[at:at+n*2].decode('utf-16le');at+=n*2;return s
   game_type=text()
   n=struct.unpack_from('<I',data,at)[0];at+=4;saved=[]
   for _ in range(n):
    ids=text();locked=data[at];at+=1;kind=struct.unpack_from('<I',data,at)[0];at+=4
    value=data[at]if kind==0 else struct.unpack_from('<I',data,at)[0];at+=1 if kind==0 else 4
    saved.append({'ids':ids,'locked':locked,'kind':kind,'value':value})
   assert at==len(data)
   yield {'actual_map':row['map'],'rules_sha256':row['rules_sha256'],'game_type':game_type,'definitions':row['records'],
     'saved':saved,'load_mode':1,'find_names':['FogOfWar','ResourceStockpileDefault','CityVictoryRatio','Sun']}

def main():
 rows=[];seams=set()
 for pc in h.PRECISION:
  for c in cases():
   a,s=run(c,'original',pc);b,t=run(c,'v14',pc);assert a==b;seams|=s|t
   rows.append({'precision':pc,'input':c,'output':a})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,
  'seams_not_executed':[hex(a)for a in sorted(seams)if a<0x1000000],
  'executed_instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams]}
 p=HERE/'native-world-rules.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'pins':len(doc['executed_instructions']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()


