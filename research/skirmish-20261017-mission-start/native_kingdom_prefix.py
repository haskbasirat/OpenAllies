"""Native4E6DB0 no-Commander/null-profile/empty-technology whole path.

Runs actual4E39D0 and actual4730D0(null) when a supplied Nation has factions.
No game-code seam is used. The required maximum-read rows deliberately stop
on the real current maximum memory read; native prior writes are retained.
Constructor/pointer/vector inputs are explicit synthetic valid allocations,
not a claim that complete saved map restoration or source registration ran.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
ORIGINAL=h.P
class Required(Exception):pass

def run(c,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 k=m.alloc(0x400);stock=m.alloc(4*max(1,len(c['stock'])));deltas=m.alloc(4*max(1,len(c['delta'])))
 if c['stock']:m.w32(stock,*c['stock'])
 if c['delta']:m.w32(deltas,*c['delta'])
 m.w32(k+0x1c4,stock,len(c['stock']),len(c['stock']));m.w32(k+0x258,deltas,len(c['delta']),len(c['delta']))
 m.w8(k+0x23a,c['reset']);m.w32(k+0x19c,9);m.w8(k+0x23b,0)
 cache=m.alloc(0x300);m.w32(0xb39ab0,cache);value=m.alloc(4);maximum=m.alloc(4)
 m.w32(value,c['starting']);m.w32(maximum,c['maximum']);m.w32(cache+0x110,value);m.w32(cache+0x1b8,maximum)
 if c['nation']:
  nation=m.alloc(0x60);m.w32(k+0x234,nation);m.w32(nation+0x4c,2)
 reads=0
 def read(u,access,a,n,v,_):
  nonlocal reads
  if a<=maximum<a+n:
   current=reads;reads+=1
   if c['fail_max']==current:raise Required('maximum_required')
 m.u.hook_add(UC_HOOK_MEM_READ,read)
 outcome='ok'
 try:m.call(0x4e6db0,k)
 except Required as e:outcome=str(e)
 return {'outcome':outcome,'stock':m.r32(stock,len(c['stock'])),'maximum_reads':reads,
  'status':m.r32(k+0x19c),'started':m.r8(k+0x23b)}

def main():
 rows=[]
 for pc in h.PRECISION:
  for reset,nation,fail in itertools.product((0,1),(0,1),(None,0,1)):
   c={'stock':[h.f2b(5),h.f2b(8)],'delta':[h.f2b(2),h.f2b(3)],'reset':reset,'starting':h.f2b(10),'maximum':h.f2b(9),'nation':nation,'fail_max':fail}
   a=run(c,'original',pc);b=run(c,'v14',pc);assert a==b
   rows.append({'precision':pc,'input':c,'output':a})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,
  'executed_instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-kingdom-prefix.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'executed_pins':len(h.VISITED),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
