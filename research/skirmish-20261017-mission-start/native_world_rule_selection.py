"""Exact4FC804..4FC857 Creator-to-GWorld Rules selection before512200 publication.
Both PE images/all x87 PCs execute unchanged branch, scalar stores,41D130 actual
record copy and511640 editable reset. Preallocated valid vector storage avoids
allocator/SEH paths; no source loader/Creator constructor or publication is
substituted or claimed. Canonical definition pointers are explicit input IDs.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P

def run(image,pc,c):
 h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(pc)
 defs=[]
 for i,row in enumerate(c['loaded']):
  p=m.alloc(0x44);m.w8(p+0x24,row['editable']);m.w32(p+0x40,row['default']);defs.append(p)
 world=m.alloc(0x300);resource=m.alloc(0xb0);creator=m.alloc(0x54)
 def vector(at,rows):
  data=m.alloc(max(1,len(rows))*12)
  for i,row in enumerate(rows):m.w32(data+i*12,defs[i],row['locked'],row['value'])
  m.w32(at,data,len(rows),len(rows));return data
 dst=vector(world+0x18,c['loaded']);src=vector(creator+0x2c,c['selected'])
 m.w32(world+0x10,c['loaded_mode'],0x11112222);m.w32(world+0x24,c['loaded_changes'])
 m.w32(world+0x28,resource);m.w32(resource+0x88,creator);m.w32(resource+0xa8,c['mode'])
 m.w32(creator+0x24,c['selected_mode'],0x33334444);m.w32(creator+0x38,c['selected_changes'])
 m.u.reg_write(UC_X86_REG_EDI,world);m.u.reg_write(UC_X86_REG_ESI,world+12);m.u.reg_write(UC_X86_REG_ESP,h.STACK)
 m.u.emu_start(0x4fc804,0x4fc857,count=100000)
 assert m.u.reg_read(UC_X86_REG_EIP)==0x4fc857
 assert m.r32(world+0x18)==dst and m.r32(creator+0x2c)==src and dst!=src
 return {'mode':m.r32(world+0x10),'game_type_token':m.r32(world+0x14),'changes':m.r32(world+0x24),
  'records':[{'definition':defs.index(m.r32(dst+i*12)),'locked_word':m.r32(dst+i*12+4),'value':m.r32(dst+i*12+8)}for i in range(m.r32(world+0x1c))],
  'source_unchanged':[{'locked_word':m.r32(src+i*12+4),'value':m.r32(src+i*12+8)}for i in range(len(defs))]}

def main():
 cases=[]
 for image,pc,mode,n in itertools.product(('original','v14'),h.PRECISION,(1,2,4),(0,1,5)):
  c={'mode':mode,'loaded_mode':2,'selected_mode':0x87654321,'loaded_changes':0x12345678,'selected_changes':0xffffffff,
   'loaded':[{'editable':i%2,'default':0xaabbcc80+i,'locked':0x11223380+i,'value':0x7fa00001+i}for i in range(n)],
   'selected':[{'locked':0x44332280+i,'value':0x80000000+i}for i in range(n)]}
  cases.append({'image':image,'precision':pc,'input':c,'output':run(image,pc,c)})
 pins=[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'seams_not_executed':[],
  'executed_instructions':pins,'cases':cases}
 p=HERE/'native-world-rule-selection.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'cases':len(cases),'executed_pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
