"""Executed native ordered-list copy, identity dedup and child ancestry guards."""
from pathlib import Path
import json,sys
from probe_lists import Machine
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import original,modified,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *

def list_case(counts):
 m=Machine();base=m.node('Base',1);m.add(base,m.node('Aspect',4,ordinal=0))
 for i in range(counts[0]):m.add(base,m.node('Aspect',16,ordinal=10+i))
 derived=m.node('Derived',1,base=base)
 for i in range(counts[1]):m.add(derived,m.node('Aspect',16,ordinal=20+i))
 m.invoke(0x5b57c0,base,[derived,0])
 actual=m.node('Thing',base=derived)
 for i in range(counts[2]):m.add(actual,m.node('Aspect',16,ordinal=30+i))
 m.invoke(0x5b57c0,derived,[actual,0])
 once=[line for name,line,flags in m.snapshot(actual)]
 #A repeated copy sees the original serialized node identity and fills missing
 #values instead of appending another copy of the same inherited list entries.
 m.invoke(0x5b57c0,derived,[actual,0])
 twice=[line for name,line,flags in m.snapshot(actual)]
 expected=[10+i for i in range(counts[0])]+[20+i for i in range(counts[1])]+[30+i for i in range(counts[2])]
 assert once==expected and twice==expected,(counts,once,twice,expected)
 assert all(flags&4==0 for _,_,flags in m.snapshot(actual))
 return {'counts':counts,'ordered_source_tags':once,'after_duplicate_copy':twice}

def ancestry_case(depth,match_at):
 m=Machine();source=m.node('Required',0x20);destination=m.node('Destination');nodes=[]
 for i in range(depth):nodes.append(m.node('Template'+str(i),0x20))
 if match_at is not None:nodes[match_at]=source
 for first,second in zip(nodes,nodes[1:]):m.w(first+0x18,second)
 if nodes:m.w(destination+0x18,nodes[0])
 def hook(u,pc,size,user):
  if pc in(0x5b5837,0x5b5952):u.emu_stop()
 m.u.hook_add(UC_HOOK_CODE,hook)
 m.w(m.stack,m.stop);m.w(m.stack+4,destination);m.w(m.stack+8,0)
 m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ECX,source)
 m.u.emu_start(0x5b57c0,m.stop,count=1000)
 pc=m.u.reg_read(UC_X86_REG_EIP);assert pc in(0x5b5837,0x5b5952)
 accepted=pc==0x5b5952;assert accepted==(depth==0 or match_at is not None)
 return {'depth':depth,'required_template_index':match_at,'accepted':accepted}

def main():
 lists=[list_case([a,b,c])for a in range(4)for b in range(4)for c in range(4)]
 ancestry=[ancestry_case(n,i)for n in range(6)for i in [None,*range(n)]]
 ranges={'copy':(0x5b57c0,0x5b5c4a),'clone':(0x5b5fc0,0x5b618c),'child_merge':(0x5b6190,0x5b62c3),'node_insert':(0x5b5c80,0x5b5fb7),'node_ctor':(0x5b38f0,0x5b39f1),'list_header_flags':(0x5b7330,0x5b74ea),'list_identity_lookup':(0x5b3680,0x5b37a0),'list_mark':(0x5b3e30,0x5b3eff),'list_iteration':(0x42be10,0x42bf46),'node_ordinal_format':(0x42bc40,0x42bd93)}
 for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a),hex(a)
 doc={'schema':1,'passed':True,'producer_sha256':sha(Path(__file__).read_bytes()),'machine_sha256':sha(Path(__file__).with_name('probe_lists.py').read_bytes()),'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'list_cases':lists,'ancestry_cases':ancestry,'ranges':{n:{'start':hex(a),'end':hex(b),'sha256':sha(original.range(a,b-a))}for n,(a,b)in ranges.items()},'seams':['Native Node structures start with the known5B38F0 constructor layout and unique source identities. List-instance bit16 is supplied according to separately pinned5B7330 parent/template-list classification.','Full5B57C0/5B6190 list recursion and native child cloning/insertion/hash/renaming execute; only successful allocation/free and string-pool resize are memory leaves.','Native generated list-name formatting42BC40 is a string formatting seam using its pinned ?%07d_%s format.','Node field hash tables are empty in these order fixtures. Field default precedence remains the independent Phase3 field-copy proof; complete TGI parser/registry execution is not claimed.','Ancestry cases stop at either the exact accepted branch or the error-construction entry; they do not execute diagnostic exception machinery.']}
 p=Path(__file__).with_name('native-lists.json');p.write_text(json.dumps(doc,indent=2));print('passed',len(lists),'lists',len(ancestry),'ancestry')
if __name__=='__main__':main()
