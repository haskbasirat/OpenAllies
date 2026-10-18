"""Original776B20 ordinary gameType2 suffix: RNG/seed, reverse31/32 stores,
and real511AF0 MULTIPLAYER/DynamicCampaign. Explicit SessionF8=13 skips the
separately proved implicitAI prefix. ScenarioFind59F570 is a named supplied
resource seam, not evidence that its global resource/depot protocol is bound.
Only suffix output is used as a Rust parity fixture. No task/map-ready claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent/'implicit-ai'))
from native_implicit import Machine,PE0,PE1,PC,proof_ranges,NODE
def main():
 rows=[];visited={}
 parameters=[{'id':'Human','profile':'','independent':0,'enabled':1,'attached':'','initial31':128,'initial32':128},
  {'id':'AI','profile':'','independent':0,'enabled':1,'attached':'','initial31':128,'initial32':255},
  {'id':'Disabled','profile':'','independent':0,'enabled':0,'attached':'','initial31':128,'initial32':128},
  {'id':'Attached','profile':'','independent':0,'enabled':0,'attached':'source','initial31':128,'initial32':255},
  {'id':'Independent','profile':'','independent':1,'enabled':0,'attached':'','initial31':128,'initial32':128}]
 for image in('original','v14'):
  m=Machine(image)
  for pc,e4,override,multiplayer,appmode in itertools.product(PC,(0,3),(0,0xffffffff),(0,1),(1,4)):
   m.active=False;m.fpu(pc);m.begin(1,list(range(16)),0);m.call(0x76ec70,this=m.S);m.first_player('Human','identity')
   human=m.r(m.r(m.S+0xbc));m.cs(human+0x24,'Human');m.w(NODE+0x2c4,NODE);m.w(NODE+0x2c8,NODE)
   m.call(0x773710,[2,NODE,1,1],this=m.S,callee_pop=16);ai=m.r(m.S+0xc4);m.cs(ai+0x24,'AI')
   creator=m.parameters(parameters);m.w(m.S+0x70,creator);data=m.r(creator+0x14)
   for i,r in enumerate(parameters):
    p=data+i*0x6c;m.cs(p+0x58,r['attached']);m.b(p+0x31,r['initial31']);m.b(p+0x32,r['initial32'])
   rules=creator+0x20;records=m.alloc(24);m.w(rules+0xc,records);m.w(rules+0x10,2);m.w(rules+0x14,2);m.w(rules+0x18,0xffffffff)
   for i,ids in enumerate(('MULTIPLAYER','DynamicCampaign')):
    definition=m.alloc(0x44);m.cs(definition+8,ids);m.w(definition+0x20,0);m.w(records+12*i,definition);m.w(records+12*i+8,0xaabbcc80)
   m.w(m.S+0x58,2);m.w(m.S+0x54,2);m.w(m.S+0xe4,e4);m.w(m.S+0xf8,13);m.w(m.S+0x100,override);m.w(m.S+0x128,0x12345678);m.b(m.S+0xf4,multiplayer)
   m.cs(m.S+0x5c,'Selected.RMP');m.names={'scenario':{'Selected.RMP':'Selected.RMP'}}
   app=m.alloc(0x14);m.w(app+0xc,appmode);m.w(0xb39690+0xfc,app)
   m.active=True;m.call(0x776b20,this=m.S)
   result=[{'byte31':m.rb(data+i*0x6c+0x31),'byte32':m.rb(data+i*0x6c+0x32)}for i in range(len(parameters))]
   rows.append({'image':image,'precision':pc,'state_e4':e4,'override':override,'multiplayer':multiplayer,'app_mode':appmode,
    'rng_before':0x12345678,'parameters':parameters,'players':[{'kingdom':'Human','ai':0},{'kingdom':'AI','ai':1}],
    'seed':m.r(creator+0x4c),'rng_after':m.r(m.S+0x128),'after':result,
    'rule_words':[m.r(records+8),m.r(records+20)],'rule_changes':m.r(rules+0x18)})
   visited.update(m.visited)
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-finalize.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'ordinary finalize suffix rows')
if __name__=='__main__':main()
