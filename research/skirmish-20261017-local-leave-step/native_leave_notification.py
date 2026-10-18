"""Whole770500 fresh/default branch, real notification ctor/copy/queue and74DA10.
Projected Session and same NativePeer are explicit inputs; actual752A60
initializes64/68. Named OS/lock/CString/allocation seams are inherited.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-local-bootstrap'))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,NODE,proof_ranges
class Machine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'bootstrap_ready',False) and pc==0x6f7430 and self.arg()in getattr(self,'vote_links',{}):
   self.retired.append(self.vote_links[self.arg()]);self.ret();return
  super().hook(u,pc,size,user)
def run(c,image,pc):
 m=Machine(image);m.fpu(pc);m.begin(0,list(range(16)),c['application_kind']);m.call(0x752a60,this=NODE)
 m.w(m.S+0x54,c['mode']);m.w(m.S+0x58,c['game_type']);m.w(m.S+0xe4,0)
 table=m.alloc(40);descriptor=m.alloc(8);m.cs(descriptor+4,'FixtureNotificationKind3');m.w(table+12,descriptor);m.w(0xb40bb0,table)
 own=m.alloc(8);m.w(own,NODE);m.w(own+4,0);m.w(NODE+0x2ac,own);m.w(NODE+0x2b0,own)
 head=0;tail=0;m.vote_links={};m.retired=[]
 for i in reversed(range(c['votes'])):
  node=m.alloc(8);m.w(node,NODE);m.w(node+4,head);head=node;m.vote_links[node]=i
  if not tail:tail=node
 m.w(NODE+0x64,head);m.w(NODE+0x68,tail);m.native_events=[];m.notification=[]
 m.call(0x770500,this=m.S)
 queued=[]
 for i in range(m.r(m.queue+0x38)):
  slot=(m.r(m.queue+0x34)+i)%m.r(m.queue+0x30);queued.append(m.read_notification(m.r(m.r(m.queue+0x2c)+slot*4)))
 assert m.r(NODE+0x64)==0 and m.r(NODE+0x68)==0
 assert m.r(NODE+0x2ac)==own and m.r(NODE+0x2b0)==own
 return dict(constructed=m.notification,queued=queued,retired=m.retired,head=0,tail=0,peer_list_same=True),m.visited
def main():
 rows=[];pins={}
 for pc,app,mode,game,votes in itertools.product(PC,[0,3],[1,2],[2,4],[0,2]):
  c=dict(application_kind=app,mode=mode,game_type=game,votes=votes);a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=2*len(rows),executed_windows=proof_ranges(pins))
 path=HERE/'native-leave-notification.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),windows=len(doc['executed_windows']),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
