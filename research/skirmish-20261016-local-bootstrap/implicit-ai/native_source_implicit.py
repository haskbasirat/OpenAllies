"""Full776580 native source-projection graph from accepted Rust source owners.
Actual parameter+2C, current-rule descriptor bytes, existing player fields and
same source ProfileService Find results are captured, then both PEs/allPC
execute the actual caller/constructor/rated/naming/assignment code. Registry
Find and current localized duplicate format use named value-transport seams.
This is not a claim of a live native Windows RMP load or a playable battle.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_implicit import Machine,PE0,PE1,PC,proof_ranges,NODE
def text(units):return ''.join(chr(u)for u in units)
def main():
 capture_path=HERE/'actual-source-input.json';capture=json.loads(capture_path.read_text());data=capture['input'];before=data['before']
 rows=[];visited={}
 assert len(before['players'])==1,'ordinary fresh actual source scope has one created human before implicitAI'
 for image in('original','v14'):
  m=Machine(image);m.nonorder.discard(0x9e7400);m.formats['networking_duplicate_name_formate|sd']='%s (%d)'
  for pc in PC:
   m.active=False;m.fpu(pc);m.begin(1,list(range(16)),0);m.call(0x9e7400);m.call(0x76ec70,this=m.S)
   human=before['players'][0];m.first_player(text(human['name']),'actual-source-identity')
   p=m.r(m.r(m.S+0xbc))
   for off,key in((0x24,'kingdom'),(0x14,'text_14'),(0x30,'text_30'),(0x34,'text_34')):m.cs(p+off,text(human[key]))
   for off,key in((0x2c,'flag_2c'),(0x2d,'flag_2d'),(0x38,'flag_38')):m.b(p+off,int(human[key]))
   for off,key in((0x3c,'field_3c'),(0x40,'field_40')):m.w(p+off,human[key])
   assert m.r(m.S+0xe8)==before['last_player_id']
   m.w(NODE+0x2c4,NODE);m.w(NODE+0x2c8,NODE)
   parameters=[{'id':text(p['id']),'profile':text(p['string_2c']),'independent':p['byte_10'],'enabled':p['byte_29']}for p in before['parameters']['kingdoms']]
   creator=m.parameters(parameters);m.w(m.S+0x70,creator);m.w(m.S+0x58,2);m.w(m.S+0x54,2)
   rules=creator+0x20;records=m.alloc(max(1,len(before['rules']))*12)
   m.w(rules+0xc,records);m.w(rules+0x10,len(before['rules']));m.w(rules+0x14,len(before['rules']));m.w(rules+0x18,before['rules_changes'])
   for i,r in enumerate(before['rules']):
    definition=m.alloc(0x44);m.cs(definition+8,r['ids']);m.w(definition+0x20,r['kind']);m.w(records+12*i,definition);m.w(records+12*i+8,r['word'])
   launch=m.alloc(0x28);m.call(0x40f7d0,this=launch);m.w(launch,before['launch_state']['kind']);m.w(launch+0x18,before['launch_state']['field_18']);m.w(m.S+0x80,launch)
   gd=m.alloc(0x600);registry=m.alloc(0x30);count=m.alloc(4);m.w(count,data['profile_count']);m.w(registry+0x24,count);m.w(gd+0x564,registry);m.w(0xb36530,gd)
   m.profiles={}
   for lookup in data['finds']:
    key=text(lookup['key']).lower();selected=lookup['selected']
    if selected is None:m.profiles[key]=0
    elif key not in m.profiles:
     profile=m.alloc(0x30);m.cs(profile+8,text(selected['ids']));m.cs(profile+0x10,text(selected['name']));m.profiles[key]=profile
   for off,i in((0xc8,0),(0xc9,1),(0xca,2)):m.b(m.S+off,int(before['changed'][i]))
   m.finds=[];m.active=True;m.call(0x776580,this=m.S)
   players=m.read_after()['players']
   for player in players:
    assert player['field_28']==0;player['field_28']=None
   after={'players':players,'last_player_id':m.r(m.S+0xe8),'changed_c8':bool(m.rb(m.S+0xc8)),'changed_c9':bool(m.rb(m.S+0xc9))}
   assert after==capture['rust_after'],(image,pc,after,capture['rust_after'])
   assert m.finds==[text(r['key'])for r in data['finds']]
   rows.append({'image':image,'precision':pc,'after':after});visited.update(m.visited)
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'input':data,'capture_sha256':hashlib.sha256(capture_path.read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-source-implicit.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'actual source implicitAI rows')
if __name__=='__main__':main()
