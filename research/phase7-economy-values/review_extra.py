"""Reviewer-authored branch vectors; outputs remain outside author corpus."""
import copy,json
from pathlib import Path
from native_values import EconomyMachine,cases,bits,original,modified,sha
HERE=Path(__file__).resolve().parent
base=next(cases());rows=[];visited={}
def add(name,op,edit):
 for pc in ['Single24','Double53','Extended64']:
  c=copy.deepcopy(base);c.update(name='review_'+name+'_'+pc,operation=op,precision=pc);edit(c)
  m=EconomyMachine(c);o=m.run();visited.update(m.visited);rows.append({'input':c,'output':o})
for flags in [0,1<<9]:
 for op in ['Cost','Upkeep','Production']:
  add(f'layout_context_{flags}_{op}',op,lambda c,f=flags:(c['actors']['1'].update(organization=1),c['modifiers']['1'].update(flags=f,factor60=bits(-.33333)),c.update(cache_members=True)))
for word in [0,7]:
 add('transport_reference_'+str(word),'Cost',lambda c,w=word:(c['definitions']['1'].update(class178=0,word17c=w),c['modifiers']['1'].update(factor60=bits(-1.111111))))
for progress in [0x80000000,0xff800000,0xffc00001,0x00000001]:
 add('supply_bits_'+hex(progress),'supplied',lambda c,v=progress:c['actors']['1'].update(state_bits=1<<19,constructor18=v))
for op in ['Cost','Upkeep']:
 add('transport_vector_length_'+op,op,lambda c:(c['definitions']['1'].update(flags174=1<<24),c['actors']['1'].update(transport=1,flags=1,element_organization=2),c['transports']['1'].update(Cost=[bits(-5)],Upkeep=[])))
for packed in [None,2,3]:
 add('packed_definition_'+str(packed),'Production',lambda c,p=packed:(c['actors']['1'].update(organization=1,state_bits=1<<19),c['definitions']['1'].update(packed_definition=p),c.update(cache_members=True),c['modifiers']['1'].update(production_adjustments=[bits(-.7),bits(11.17),bits(-20)])))
for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b))
doc={'scope':__doc__,'producer_sha256':sha(Path(__file__).read_bytes()),'cases':rows,'visited_instruction_windows':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())]}
(HERE/'review-extra-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(len(rows))
