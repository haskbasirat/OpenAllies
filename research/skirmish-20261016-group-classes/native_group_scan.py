"""Original578A30: ordered parameter AbilityData flags and lazy locked gate.

No callback seams. Null skipped parameters/late abilities are unmapped so an
accidental eager read cannot silently appear successful. Native faults are
reported as invalid pointer reads, not gameplay outcomes or invented defaults.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified,sha
from unicorn import UC_HOOK_CODE,UcError
from unicorn.x86_const import UC_X86_REG_ECX,UC_X86_REG_EAX,UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_FPCW
ORG=0x2002000;LAYOUT=0x2003000;GROUPS=0x2004000;PARAMS=0x2005000
def run(c,image,pc):
 m=Machine();old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 m.w(ORG+0x18,LAYOUT);m.w(ORG+0x1c,PARAMS);m.w(LAYOUT+0x1c,GROUPS);m.w(LAYOUT+0x20,len(c['groups']))
 for i,g in enumerate(c['groups']):
  m.u.mem_write(GROUPS+i*16+13,bytes([g['locked']]))
  param=0x2006000+i*0x1000;arr=param+0x300
  if g['abilities'] is None:m.w(PARAMS+i*4,0);continue
  m.w(PARAMS+i*4,param);m.w(param+0x1ec,arr);m.w(param+0x1f0,len(g['abilities']))
  for j,flag in enumerate(g['abilities']):
   local=arr+0x100+j*0x100;definition=local+0x10
   m.w(arr+j*4,local if flag is not None else 0)
   if flag is not None:m.w(local,definition);m.w(definition+0x54,flag)
 visited={}
 def hook(u,address,size,_):visited[address]=bytes(u.mem_read(address,size))
 m.u.hook_add(UC_HOOK_CODE,hook,begin=0x578a30,end=0x578aff)
 m.reset(0);m.u.reg_write(UC_X86_REG_FPCW,pc);m.u.reg_write(UC_X86_REG_ECX,ORG);m.w(m.STACK+4,c['locked_only'])
 try:
  m.u.emu_start(0x578a30,m.STOP,count=1000)
  assert m.u.reg_read(UC_X86_REG_EIP)==m.STOP and m.u.reg_read(UC_X86_REG_ESP)==m.STACK+8
  out={'result':bool(m.u.reg_read(UC_X86_REG_EAX)&255)}
 except UcError:out={'invalid_read_at':hex(m.u.reg_read(UC_X86_REG_EIP))}
 return out,visited
def main():
 rows=[];visited={}
 groups=[[],[{'locked':False,'abilities':None}],[{'locked':True,'abilities':[0,8,None]}],
  [{'locked':True,'abilities':[0,None]}],[{'locked':True,'abilities':[8]},{'locked':True,'abilities':None}],
  [{'locked':False,'abilities':[8]},{'locked':True,'abilities':[0,1,4]}],
  [{'locked':True,'abilities':[]}],[{'locked':True,'abilities':[8,8]}]]
 for index,g in enumerate(groups):
  for locked in [False,True]:
   c={'name':f'group{index}_locked{locked}','groups':g,'locked_only':locked}
   for precision,pc in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
    for name,image in [('original',original),('v14',modified)]:
     result,code=run(c,image,pc);visited.update(code);rows.append({'image':name,'precision':precision,'input':c,'output':result})
 for a,code in visited.items():assert code==original.range(a,len(code))==modified.range(a,len(code))
 document={'scope':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'cases':rows,'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())]}
 (HERE/'native-group-scan.json').write_text(json.dumps(document,indent=2)+'\n')
 print(json.dumps({'cases':len(rows),'instructions':len(visited)}))
if __name__=='__main__':main()
