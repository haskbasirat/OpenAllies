"""Type2/flags0 stockpile and type3/flags0 city WorldRule publication.

Actual401710/401650 constructors,474C50 defaults,4B3580 source reset,
511640 editable reset and512200/40E0A0/512160 setters execute unchanged.
Source descriptors/default words are explicit inputs (not a full TGI-load
claim). CString storage, named-variable lookup and logging are transport seams.
Fixed NaN payload rows certify stored words only, not emulator x87 status.
"""
from pathlib import Path
import hashlib,itertools,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P

def actual_cases():
 sys.path.insert(0,str(ROOT/'research/phase8-conversion'));from rmp import Rmp,MAPS
 for name in ('Campaign/tutorial02','DeveloperMaps/(3v3)Firefight','(2v2)Berlin'):
  data=Rmp(MAPS/(name+'.RMP')).chunk('RULES');at=8
  assert struct.unpack_from('<II',data)==(4,0)
  def string():
   nonlocal at
   n=struct.unpack_from('<H',data,at)[0];at+=2;s=data[at:at+2*n].decode('utf-16le');at+=2*n;return s
  game=string();count=struct.unpack_from('<I',data,at)[0];at+=4
  selected={}
  for _ in range(count):
   key=string().split('\0')[0];locked=data[at];at+=1;ty=struct.unpack_from('<I',data,at)[0];at+=4
   if ty==0:value=data[at];at+=1
   else:value=struct.unpack_from('<I',data,at)[0];at+=4
   if (key.lower(),ty)in [('resourcestockpiledefault',2),('cityvictoryratio',1)]:selected[key.lower()]=value
  assert at==len(data)
  yield {'actual_map':name,'rules_sha256':hashlib.sha256(data).hexdigest(),'mode':1,'editable':0,'locked':0,
   'stock_default':h.f2b(500),'city_default':0,'stock_value':selected.get('resourcestockpiledefault',h.f2b(500)),
   'city_value':selected.get('cityvictoryratio',0)}

def run(c,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);seams=set()
 def seam(a,f):m.seams[a]=f;seams.add(a)
 def text(s):
  header=m.alloc(0x100);m.w32(header+12,9);p=header+16;m.u.mem_write(p,s.encode('utf-16le')+b'\0\0');return p
 names={k:text(k)for k in ['ResourceStockpileDefault','CityVictoryRatio']}
 vars={k:m.alloc(0x30)for k in names};cells={k:m.alloc(4)for k in names}
 for k in names:m.w32(cells[k],vars[k])
 allocator=m.alloc(0x20);vt=m.alloc(0x20);m.w32(allocator,vt);empty=m.alloc(0x20)
 stub=m.stub_next;m.stub_next+=16;m.w32(vt+12,stub)
 seam(stub,lambda _:m.ret(value=empty));seam(0x856370,lambda _:m.ret(value=allocator))
 seam(0x4012c0,lambda _:m.ret(value=m.arg()))
 def assign(_):m.w32(m.ecx(),m.r32(m.arg()));m.ret(4,m.ecx())
 seam(0x405b80,assign)
 def lookup(_):
  p=m.r32(m.arg());name=next(k for k,v in names.items()if v==p);m.ret(4,cells[name])
 seam(0x405370,lookup);seam(0x404f60,lambda _:m.ret())
 m.call(0x401710,vars['ResourceStockpileDefault'],[names['ResourceStockpileDefault'],h.f2b(500),0])
 m.call(0x401650,vars['CityVictoryRatio'],[names['CityVictoryRatio'],0,0])
 ctor={k:{'value':m.r32(v),'type':m.r32(v+0x24),'flags':m.r32(v+0x28)}for k,v in vars.items()}
 manager=m.alloc(0x600);defs=m.alloc(8);m.w32(0xb36530,manager);m.w32(manager+0x118,defs,2)
 descriptors=[]
 for i,(name,ty,default)in enumerate([('CityVictoryRatio',1,c['city_default']),('ResourceStockpileDefault',2,c['stock_default'])]):
  d=m.alloc(0x60);m.w32(d+8,names[name]);m.w32(d+0x20,ty);m.w8(d+0x24,c['editable']);m.w32(d+0x40,default);m.w32(defs+i*4,d);descriptors.append(d)
 m.call(0x474c50)
 source={k:m.r32(v)for k,v in vars.items()}
 snapshots=[m.alloc(0x30)for _ in vars];pointers=m.alloc(8);snapshot=m.alloc(0x20)
 for i,((name,v),saved)in enumerate(zip(vars.items(),snapshots)):
  m.u.mem_write(saved,bytes(m.u.mem_read(v,0x2c)));m.w32(pointers+4*i,saved);m.w32(v,0xdeadbeef)
 m.w32(snapshot+8,pointers,2);m.call(0x4b3580,0xb39aa8,[snapshot])
 reset={k:m.r32(v)for k,v in vars.items()}
 rules=m.alloc(0x20);entries=m.alloc(24);m.w32(rules+12,entries,2)
 m.w32(entries,descriptors[0],c['locked'],c['city_value']);m.w32(entries+12,descriptors[1],c['locked'],c['stock_value'])
 if c['mode']&2:m.call(0x511640,rules)
 m.call(0x512200,rules)
 final={k:m.r32(v)for k,v in vars.items()}
 return {'constructors':ctor,'source':source,'reset':reset,'current':final},seams

def main():
 rows=[];seams=set()
 for pc in h.PRECISION:
  for mode,editable,locked,pair in itertools.product((1,2,4),(0,1),(0,255),[(h.f2b(1250),70),(0x80000000,0xffffffff),(0x7fc01234,0x7fffffff),(h.f2b(-1),0x80000000)]):
   c={'mode':mode,'editable':editable,'locked':locked,'stock_default':h.f2b(500),'city_default':0,'stock_value':pair[0],'city_value':pair[1]}
   a,s=run(c,'original',pc);b,t=run(c,'v14',pc);assert a==b;seams|=s|t
   rows.append({'precision':pc,'input':c,'output':a})
  for c in actual_cases():
   a,s=run(c,'original',pc);b,t=run(c,'v14',pc);assert a==b;seams|=s|t
   rows.append({'precision':pc,'input':c,'output':a})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,
  'seams_not_executed':[hex(a)for a in sorted(seams)if a<0x1000000],
  'executed_instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams]}
 p=HERE/'native-mission-variables.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'executed_pins':len(doc['executed_instructions']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
