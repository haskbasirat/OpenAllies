"""Replay actual held Performer inputs at native57E430 in separate original/v14
machines.45E090, modifier getters and SSE2 CRT pow execute unchanged. The
constructed-closed manager stream virtual predicate is an explicit successful
leaf seam; this does not prove the stream constructor, full world initialization
or visible gameplay. Captured same-manager cached SVar guards remain real input.
"""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-combat'))
import native_combat_value as old
h=old.h
from unicorn import x86_const as X

class Actual(old.Oracle):
 def __init__(self,image,pc):
  self.seam_hits=[];self.required_pow_at=None;self.pow_calls=0
  super().__init__(pc)
  if image=='v14':
   original_end=h.ORIGINAL.base+((h.ORIGINAL.size+0xFFF)&~0xFFF)
   modified_end=h.ACCEPTED.base+((h.ACCEPTED.size+0xFFF)&~0xFFF)
   if modified_end>original_end:self.u.mem_map(original_end,modified_end-original_end)
   self.u.mem_write(h.ACCEPTED.base,h.ACCEPTED.data[:h.ACCEPTED.headers])
   for _,rva,length,offset in h.ACCEPTED.sections:
    if length:self.u.mem_write(h.ACCEPTED.base+rva,h.ACCEPTED.data[offset:offset+length])
  self.w(h.SETTINGS_ROOT,self.settings_table);self.w(old.MANAGER_ROOT,self.manager)
 def _code(self,uc,pc,size,_):
  if pc==0x96D890:
   self.pow_calls+=1
   if self.required_pow_at==self.pow_calls:raise RuntimeError('required current-thread CRT boundary')
  handler=self.hooks.get(pc)
  if handler is not None:
   self.seam_hits.append(pc);handler(self);return
  if h.ORIGINAL.base<=pc<h.ORIGINAL.base+h.ORIGINAL.size:
   raw=bytes(uc.mem_read(pc,size));assert raw==h.ORIGINAL.at(pc,size)==h.ACCEPTED.at(pc,size),hex(pc)
   self.visited[pc]=raw.hex()
  elif self.VIRTUALS<=pc<self.VIRTUALS+0x10000:raise RuntimeError(f'unhooked virtual {pc:#x}')

def run(image,pc,c,required_pow_at=None):
 m=Actual(image,pc);words=c['current_words'];controls=c['environment_controls']
 m.required_pow_at=required_pow_at
 assert controls['sse2'] and controls['mxcsr']==0x1F80 and controls['x87_control']&0xC00==0
 m.u.reg_write(X.UC_X86_REG_MXCSR,controls['mxcsr']);m.w(0xB4B340,int(controls['sse2']))
 m.log_flag=0
 for i,offset in enumerate([0xA4,0x144,0x148,0x14C,0x150,0x154,0x158]):m.setting(offset,words[i])
 m.setting(0x1A4,words[9])
 m.svars={}
 for name,bits,pointer in [('CombatValueRangedMinimum',words[7],0xB3A668),('CombatValueDefenseWeightRanged',words[8],0xB3A664)]:
  cell=m.alloc(4);m.w(cell,bits);m.svars[name]=cell;m.w(pointer,cell)
 guards=sum((1<<i)for i,bound in enumerate(c['cached_svar_guards'])if bound);m.w(old.GUARDS,guards)
 for offset,key in [(0x74,'maximum_74'),(0x78,'attack'),(0x7C,'attack_rate'),(0x80,'defense'),(0x84,'movement'),(0x88,'health'),(0x8C,'lifetime'),(0x90,'offense')]:m.w(m.manager+offset,c['means'][key])
 actor=m.alloc(0x100);definition=m.alloc(0x220);m.w(actor+4,definition);m.w(definition+0x210,c['bonus'])
 if c['body'] is not None:
  body=m.alloc(0x40);m.w(body+0x18,c['body'][0]);m.w(body+0x14,c['body'][1]);m.w(actor+0x60,body)
 if c['speed'] is not None:
  mover=m.alloc(0x40);m.w(mover+0x28,c['speed']);m.w(actor+0x6C,mover)
 if c['property_values'] is not None:
  props=m.alloc(0x100);array=m.alloc(4*max(1,len(c['property_values'])))
  for i,bits in enumerate(c['property_values']):
   entry=m.alloc(0x20);propdef=m.alloc(0x40);m.w(propdef+0x28,bits);m.w(entry+0xC,propdef);m.w(array+4*i,entry)
  m.w(props+0x10,array);m.w(props+0x14,len(c['property_values']))
  assert c['modifier'] is not None and len(c['modifier'])==25
  for i,bits in enumerate(c['modifier']):m.w(props+0x1C+4*(i+1),bits)
  m.w(actor+0x78,props)
 def table(rows,tag):
  array=m.alloc(4*max(1,len(rows)))
  for i,row in enumerate(rows):
   entry=m.alloc(0x20);source=row['source'];local=dict(source['local']);local['form']='absent'if local['first_entry_10']is None else'entry'
   m.w(entry+8,old.build_link(m,source['definition'],local,tag+i));m.wb(entry+0x18,int(row['enabled']));m.w(array+4*i,entry)
  return array
 performer=m.alloc(0x80);m.w(performer+4,actor);m.w(performer+0x14,table(c['abilities'],0x100));m.w(performer+0x18,len(c['abilities']));m.w(performer+0x60,c['before'])
 if c['secondary']is not None:
  defender=m.alloc(0x40);m.w(defender+0x10,table(c['secondary'],0x200));m.w(defender+0x14,len(c['secondary']));m.w(actor+0xB0,defender)
 try:m.call(old.PERFORMER,ecx=performer)
 except RuntimeError:
  if required_pow_at is None or m.reg(X.UC_X86_REG_EIP)!=0x96D890 or m.pow_calls!=required_pow_at:raise
  assert all(a==old.LOOKUP for a in m.seam_hits)
  return dict(value=m.r(performer+0x60),cached_svar_guards=[bool(m.r(old.GUARDS)&1),bool(m.r(old.GUARDS)&2)],required_pow_at=required_pow_at,required_boundary='0096d890',successful_seams=['named SVar lookup transport'for _ in m.seam_hits]),m.visited
 stream_stub=m.r(m.r(m.manager+0x4C)+0x14)
 labels={stream_stub:'constructed-closed manager+4C virtual+14',old.LOOKUP:'named SVar lookup transport',old.CSV:'CSV writer',old.ALLOC:'allocator',old.FREE:'free',old.ASSERT:'assert',old.LOG:'debug log'}
 assert all(a in labels for a in m.seam_hits)
 assert old.CSV not in m.seam_hits and old.ASSERT not in m.seam_hits
 return dict(value=m.r(performer+0x60),cached_svar_guards=[bool(m.r(old.GUARDS)&1),bool(m.r(old.GUARDS)&2)],seam_hits=[labels[a]for a in m.seam_hits],events=m.events),m.visited

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 source=HERE/'actual-performer-values.json';captured=json.loads(source.read_bytes());rows=[];pins={}
 for row in captured['rows']:
  a,pa=run('original',row['precision'],row['input']);b,pb=run('v14',row['precision'],row['input'])
  assert a==b and pa==pb and a['value']==row['output'],(row['phase'],row['precision'],row['input']['definition'],a['value'],row['output'])
  rows.append(dict(phase=row['phase'],precision=row['precision'],input=row['input'],output=a));pins.update(pa)
 out=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_executions=2*len(rows),cases=rows,
  executed_instruction_pins={f'{a:08x}':b for a,b in sorted(pins.items())},capture_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
  source_images=[dict(name=n,sha256=hashlib.sha256(p.data).hexdigest())for n,p in [('original',h.ORIGINAL),('v14',h.ACCEPTED)]])
 path=HERE/'native-actual-performer.json'
 if args.write:
  with path.open('x')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_bytes())==out
 print(json.dumps(dict(paired_cases=len(rows),native_executions=2*len(rows),executed_pins=len(pins),successful_seam_names=sorted(set(s for r in rows for s in r['output']['seam_hits'])),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
