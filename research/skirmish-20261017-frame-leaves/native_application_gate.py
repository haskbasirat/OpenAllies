"""Whole native40EAA0 with real766380(name,0),4012C0 CString refcount
copy and release, across null/active Session, F4, null/empty/percent/ordinary
local names and raw DevMode bytes. The only name/allocator service seam is
CString-manager virtual+10 returning that same manager; no allocator/free
branch is reached. Required source cutoffs preserve exact lazy read prefixes;
these cutoffs are explicit caller-binding errors, not native exceptions.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_CODE
from native_ui_selected_kingdom import string
ORIGINAL=h.P
class Cutoff(Exception):pass
def run(image,pc,c):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 manager=m.alloc(4);table=m.alloc(20);leaf=m.stub_next;m.stub_next+=16
 m.w32(manager,table);m.w32(table+16,leaf);m.seams[leaf]=lambda x:x.ret(value=x.ecx())
 session=0 if c['state']is None else m.alloc(0x100);m.w32(0xb36550,session)
 if session:m.w32(session+0xe4,c['state']);m.w8(session+0xf4,c['multiplayer'])
 node=m.alloc(12);m.w32(0xb36558,node)
 if c['name']is not None:
  player=m.alloc(0x10);m.w32(player+8,string(m,c['name'],manager));m.w32(node+8,player)
 uvars=m.alloc(0x100);dev=m.alloc(4);m.w32(0xb39940,uvars);m.w32(uvars+0xf8,dev);m.w8(dev,c['dev'])
 events=[];stop=None
 sites={0x40eaa6:'session',0x40eab9:'multiplayer',0x40eac8:'local',0x40eb1a:'dev'}
 def observe(u,a,n,data):
  nonlocal stop
  if a in sites:
   label=sites[a];events.append(label)
   if c['fail']==label:stop=label;raise Cutoff()
 m.u.hook_add(UC_HOOK_CODE,observe)
 value=None
 try:value=(m.call(0x40eaa0)&255)!=0
 except Cutoff:pass
 return {'value':value,'events':events,'stop':stop}
def cases():
 for state,multi,name,dev in itertools.product((None,0,3),(0,128),(None,[],[85],[37,68]),(0,1,128)):
  yield {'state':state,'multiplayer':multi,'name':name,'dev':dev,'fail':None}
 for fail in ('session','multiplayer','local','dev'):
  yield {'state':3,'multiplayer':128,'name':[37],'dev':128,'fail':fail}
def main():
 rows=[];cutoffs=set()
 for pc,c in itertools.product(h.PRECISION,cases()):
  a=run('original',pc,c);b=run('v14',pc,c);assert a==b
  rows.append({'precision':pc,'input':c,'output':a})
 #All four cutoff addresses execute in other successful rows. Every pin is
 #therefore independently witnessed by a completed native invocation.
 pins=[]
 for a,b in sorted(h.VISITED.items()):
  assert bytes.fromhex(b)==ORIGINAL.range(a,len(bytes.fromhex(b)))==h.V.range(a,len(bytes.fromhex(b)))
  pins.append({'va':hex(a),'bytes':b})
 doc={'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
      'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'cases':rows,'executed_pins':pins}
 path=HERE/'native-application-gate.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':2*len(rows),'genuine_executed_pins':len(pins)}))
if __name__=='__main__':main()
