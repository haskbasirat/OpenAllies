"""Selected type2/flags0 CV variables: actual401710 constructors,4B3580 reset,
40E0A0 DWORD mutation. Names/default words are explicit source-backed inputs;
no full TGI/profile loader claim. CString storage/assignment/named lookup are
transport seams only. NaN/negative rows certify raw words, not numeric use.
"""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
NAMES=['CombatDamageMinimum','CombatValueRangePower','CombatValueAreaPower','CombatValueOffenseWeight','CombatValueDefenseWeight','CombatValueMovementWeight','CombatValueMovementDefault','CombatValueRangedMinimum','CombatValueDefenseWeightRanged']
SOURCE=[h.f2b(v)for v in [1,0.15,1,0.5,0.5,0.2,1,12,0.25]]
def run(image,pc,c):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);seams=set()
 def seam(a,f):m.seams[a]=f;seams.add(a)
 def text(s):
  p=m.alloc(0x100);m.w32(p+12,9);m.u.mem_write(p+16,s.encode('utf-16le')+b'\0\0');return p+16
 names={n:text(n)for n in NAMES};variables={n:m.alloc(0x30)for n in NAMES};cells={n:m.alloc(4)for n in NAMES}
 for n in NAMES:m.w32(cells[n],variables[n])
 allocator=m.alloc(0x20);vt=m.alloc(0x20);empty=m.alloc(0x20);m.w32(allocator,vt)
 stub=m.stub_next;m.stub_next+=16;m.w32(vt+12,stub)
 seam(stub,lambda _:m.ret(value=empty));seam(0x856370,lambda _:m.ret(value=allocator))
 seam(0x4012C0,lambda _:m.ret(value=m.arg()))
 def assign(_):m.w32(m.ecx(),m.r32(m.arg()));m.ret(4,m.ecx())
 seam(0x405B80,assign)
 def lookup(_):
  ptr=m.r32(m.arg());name=next(n for n,p in names.items()if p==ptr);m.ret(4,cells[name])
 seam(0x405370,lookup);seam(0x404F60,lambda _:m.ret())
 for i,n in enumerate(NAMES):m.call(0x401710,variables[n],[names[n],c['source'][i],0])
 constructed=[dict(value=m.r32(variables[n]),kind=m.r32(variables[n]+0x24),flags=m.r32(variables[n]+0x28))for n in NAMES]
 snapshot=m.alloc(0x20);array=m.alloc(4*len(NAMES))
 for i,n in enumerate(NAMES):
  saved=m.alloc(0x30);m.u.mem_write(saved,bytes(m.u.mem_read(variables[n],0x2C)));m.w32(array+4*i,saved);m.w32(variables[n],0xDEADBEEF)
 m.w32(snapshot+8,array,len(NAMES));m.call(0x4B3580,0xB39AA8,[snapshot])
 reset=[m.r32(variables[n])for n in NAMES]
 for i,n in enumerate(NAMES):m.call(0x40E0A0,variables[n],[c['later'][i]])
 current=[m.r32(variables[n])for n in NAMES]
 return dict(constructed=constructed,reset=reset,current=current),seams
def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 inputs=[dict(source=SOURCE,later=[v]*9)for v in [0,0x80000000,0x7FC01234,0x7F800001,0xFFFFFFFF,h.f2b(-2),h.f2b(70)]]
 inputs += [dict(source=SOURCE,later=[h.f2b((i+1)/8)for i in range(9)]),dict(source=SOURCE,later=list(reversed(SOURCE)))]
 rows=[];seams=set()
 for pc in h.PRECISION:
  for c in inputs:
   a,s=run('original',pc,c);b,t=run('v14',pc,c);assert a==b;seams|=s|t
   rows.append(dict(precision=pc,input=c,output=a))
 doc=dict(schema=1,scope=__doc__,names=NAMES,paired_cases=len(rows),native_executions=len(rows)*2,cases=rows,
  source_images=[dict(name='original',sha256=h.ORIGINAL_SHA),dict(name='v14',sha256=h.V14_SHA)],
  seams_not_executed=[f'{a:08x}'for a in sorted(seams)if a<0x1000000],
  executed_instruction_pins={f'{a:08x}':b for a,b in sorted(h.VISITED.items())if a not in seams})
 p=HERE/'native-current-combat-vars.json'
 if args.write:
  with p.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(p.read_bytes())==doc
 print(json.dumps(dict(paired_cases=len(rows),native_executions=len(rows)*2,executed_pins=len(doc['executed_instruction_pins']),seam_entries=len(doc['seams_not_executed']),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
