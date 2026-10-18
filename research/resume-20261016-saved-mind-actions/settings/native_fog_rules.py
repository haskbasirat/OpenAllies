"""Original WorldRules Load/default merge/reset/update with explicit definition seams.

The FogOfWar descriptor (kind3/default1/three enums) is source-bound by the
separate static evidence. Native512420,511D90,511CE0,511C30,511640,512200,
512160,4B29B0 execute. Stream/string allocation and definition/hash lookups
are explicit seams inherited from the independently audited header harness.
"""
from pathlib import Path
import sys,struct,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];GAME=ROOT.parent
sys.path.insert(0,str(GAME/'reconstruction/tools/python-packages'))
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
from native_header import Machine as Base,P,V
from inspect_header import read_chunks,Reader,rules as decode_rules
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
def enc(s):
 b=struct.pack('<'+'H'*len(s),*s) if isinstance(s,list) else s.encode('utf-16-le');return struct.pack('<H',len(b)//2)+b
def wire(rows,game=''):
 b=struct.pack('<II',4,0)+enc(game)+struct.pack('<I',len(rows))
 for key,locked,kind,value in rows:b+=enc(key)+bytes([locked])+struct.pack('<I',kind)+(bytes([value]) if kind==0 else struct.pack('<I',value&0xffffffff))
 return b
class Machine(Base):
 def __init__(self,data,editable=False):
  super().__init__(data);self.visited={};self.lookup_events=[];self.unknown_game=False
  self.crt_thread=self.alloc(0x100);self.write(self.crt_thread+0x64,self.read(0xb35444))
  self.write(self.read(self.stream)+0xc,0x2e00060)
  self.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,self.null_guard,begin=0,end=0xfff)
  registry=self.alloc(0x600);self.write(0xb36530,registry)
  game=self.alloc(4);self.write(registry+0x4a0,game)
  self.definition=self.alloc(0x60);self.write(self.definition+8,self.string('FogOfWar'.encode('utf-16-le')));self.write(self.definition+0xc,0);self.write(self.definition+0x20,3);self.write(self.definition+0x24,int(editable));self.write(self.definition+0x38,3);self.write(self.definition+0x40,1)
  array=self.alloc(4);self.write(array,self.definition);self.write(registry+0x118,array);self.write(registry+0x11c,1)
  self.lookup_manager(registry+0xf4,self.definition)
  self.game_definition=self.alloc(0x20);self.write(self.game_definition+8,self.string('NORMAL'.encode('utf-16-le')))
  game_manager=self.alloc(0x50);self.lookup_manager(game_manager,self.game_definition);self.write(0xb3aa70,game_manager)
  self.entries=self.alloc(12*4);self.write(self.rules+0xc,self.entries);self.write(self.rules+0x10,0);self.write(self.rules+0x14,4)
  self.var=self.alloc(0x30);self.write(self.var,1);self.write(self.var+0x14,0);self.write(self.var+0x18,2);self.write(self.var+0x24,3);self.write(self.var+0x28,0x20)
  self.cell=self.alloc(8);self.write(self.cell,self.var)
 def lookup_manager(self,manager,definition):
  buckets=self.alloc(4);link=self.alloc(8);self.write(link,definition);self.write(link+4,0);self.write(buckets,link)
  self.write(manager+0x14,0);self.write(manager+0x30,buckets);self.write(manager+0x34,1)
 def text(self,p):
  units=[]
  while True:
   v=struct.unpack('<H',self.u.mem_read(p,2))[0];p+=2
   if v==0:return bytes(b''.join(struct.pack('<H',v) for v in units)).decode('utf-16-le')
   units.append(v);assert len(units)<1000
 def null_guard(self,u,access,address,size,value,data):
  assert bytes(u.mem_read(u.reg_read(UC_X86_REG_EIP),1))==b'\x64',f'non-FS pagezero at{u.reg_read(UC_X86_REG_EIP):x}'
 def hook(self,u,pc,size,data):
  if P.base<=pc<P.base+P.image_size:
   raw=bytes(u.mem_read(pc,size));assert raw==P.range(pc,size)==V.range(pc,size);self.visited[pc]=raw
  if pc in (0x511d90,0x8589b0,0x512360):return
  if pc==0x9738a9:self.ret(value=self.crt_thread);return # CRT thread storage, original C locale
  if pc==0x2e00060:self.ret(value=0);return # original stream IsSaving=false
  if pc==0x405370:
   name=self.text(self.read(self.arg()));self.lookup_events.append(['setting',name]);assert name in ['ResourceStockpileDefault','FogOfWar'];self.ret(4,value=self.cell);return
  if pc==0x470470:
   self.lookup_events.append(['definition_native',self.text(self.arg())]);return
  if pc==0x48d500:
   self.lookup_events.append(['game_type_native',self.text(self.arg())]);return
  if pc==0x41dfb0:
   self.unknown_game=True;u.emu_stop();return
  if pc in (0x404f60,0x404f90):self.ret();return
  super().hook(u,pc,size,data)
 def invoke(self,pc):
  self.write(self.stack,self.stop);self.write(self.stack+4,self.stream);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,self.rules);self.u.emu_start(pc,self.stop,count=200000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
 def run(self,editor):
  self.invoke(0x512420);assert self.r.at==len(self.r.b)
  before={'locked':self.read(self.entries+4)&255,'value':self.read(self.entries+8)}
  if editor:self.invoke(0x511640)
  self.invoke(0x512200)
  return {'before_editor_reset':before,'locked':self.read(self.entries+4)&255,'value':self.read(self.entries+8),'fog_integer':self.read(self.var),'fog_enabled':self.read(self.var)!=0,'entry_count':self.read(self.rules+0x10),'consumed':self.r.at,'lookups':self.lookup_events}
 def run_game_type(self):
  out=self.alloc(4);self.write(out,0xcccccccc);self.write(self.stack,self.stop);self.write(self.stack+4,self.stream);self.write(self.stack+8,out);self.u.reg_write(UC_X86_REG_ESP,self.stack)
  self.u.emu_start(0x512360,self.stop,count=200000)
  assert self.unknown_game or self.u.reg_read(UC_X86_REG_EIP)==self.stop
  assert self.r.at==len(self.r.b)
  return dict(accepted=not self.unknown_game,unknown_reference_exception=self.unknown_game,stored_null=self.read(out)==0,lookups=self.lookup_events,consumed=self.r.at)
def main():
 cases=[('missing',[]),('unknown',[('Unknown',1,3,0)]),('type_changed',[('FogOfWar',1,1,0)]),('duplicate',[('FogOfWar',0,3,0),('FogOfWar',1,3,2)]),('duplicate_bad_type',[('FogOfWar',1,3,2),('FogOfWar',0,1,0)])]
 for locked in [0,1,255]:
  for value in [-1,0,1,2,3,2147483647,-2147483648]:cases.append((f'locked{locked}_value{value}',[('FogOfWar',locked,3,value)]))
 for name,key in [('mid_nul','FogOfWar\0ignored'),('mixed_case_nul','fOgOfWaR\0ignored'),('first_nul','\0FogOfWar'),('leading_space',' FogOfWar\0ignored'),('invalid_surrogate_after_nul',list(map(ord,'FogOfWar'))+[0,0xd800])]:
  cases.append((name,[(key,0,3,0)]))
 rows=[];visited={}
 for name,records in cases:
  for editor,editable in [(False,False),(True,False),(True,True)]:
   m=Machine(wire(records),editable);out=m.run(editor);rows.append({'name':name,'records':records,'editor':editor,'always_editable':editable,'output':out});visited.update(m.visited)
 corpus=[]
 for path in sorted((GAME/'reconstruction/assets/original/Data/Maps').rglob('*.rmp')):
  data=read_chunks(path)['RULES'][0];reader=Reader(data);decoded=decode_rules(reader);assert reader.at==len(data)
  m=Machine(data);out=m.run(False);visited.update(m.visited)
  corpus.append(dict(path=str(path),rules_sha256=sha(data),rules_hex=data.hex(),decoded=decoded,output=out))
 games=[]
 for name,units in [('empty',[]),('normal',list(map(ord,'NORMAL'))),('mid_nul',list(map(ord,'NORMAL\0ignored'))),('mixed_case_nul',list(map(ord,'nOrMaL\0ignored'))),('invalid_surrogate_after_nul',list(map(ord,'NORMAL'))+[0,0xd800]),('first_nul',[0]),('first_nul_invalid_tail',[0,0xd800]),('unknown',list(map(ord,'Unknown'))),('leading_space',list(map(ord,' NORMAL')))]:
  m=Machine(enc(units));out=m.run_game_type();games.append(dict(name=name,units=units,output=out));visited.update(m.visited)
 report={'scope':__doc__,'source_sha256':sha(P.b),'v14_sha256':sha(V.b),'producer_sha256':sha(Path(__file__).read_bytes()),'cases':rows,'passed':len(rows),'original_map_corpus':corpus,'corpus_passed':len(corpus),'game_type_cases':games,'game_type_cases_passed':len(games),'instruction_pins':[{'address':f'{a:08x}','bytes':b.hex()} for a,b in sorted(visited.items())],'seams':['Original header harness stream read/write/string allocation and logging leaves; IsSaving returns false.','Source-bound FogOfWar descriptor and one-bucket registry are provided, not constructed by4905E0; actual470470/4915F0/405030/CRT lookup executes.','405370 returns existing typed bounded FogOfWar SVar and a harmless ResourceStockpileDefault initialization lookup cell.','Native512360 and48D500 GameType lookup execute against provided NORMAL descriptor; full-length UTF16 stream string allocation is a41D290 seam preserving NUL/tail units.','Unknown GameType exception caught at original41DFB0 branch; no claim to execute complete SEH unwinding.','No native source TGI loader or complete GWorld/Session execution.']}
 (HERE/'native-fog-rules.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8');print('passed',len(rows),'corpus',len(corpus),'game_type',len(games))
if __name__=='__main__':main()
