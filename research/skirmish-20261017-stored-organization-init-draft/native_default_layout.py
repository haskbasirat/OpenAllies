"""Actual579BD0 then57C450 (including4C2290/4C21E0, vectors and578300).
Inherits disclosed allocator/CString/definition and world-child transports from
the old Organization oracle. Dirty/reused rows are explicit direct57C450 labs;
fresh rows are constructor-empty. Source default data is supplied lab metadata.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'))
import native_machine as h, native_organization as n
H=lambda b:hashlib.sha256(b).hexdigest()
def run(c,image):
 h.P=image;n.P=image
 s=n.base_spec();s['precision']=c['precision'];s['default_layout']=(c['layout'],c['parameters'])
 s['state_group']=[None];s['owner']['modifier']=None
 m=n.OrganizationMachine(s,b'');data=m.defs[s['owner']['def']]
 m.w(m.layouts[c['layout']]+0x40,0)
 org=m.alloc(0xdc,0xcd);m.org=org;m.labels[org]='org';m.call(0x579bd0,org,[m.owner])
 if c['dirty']:
  for base in[0x28,0x34,0x50]:m.w(org+base+4,1);m.w(m.r(org+base),0x1234)
  m.w(org+0x7c,1);m.w(m.r(org+0x78),0x3f800000);m.w(m.r(org+0x78)+4,0x40000000)
 initial=m.state(org);m.calls=[];prefix=None
 prior=m.seams[0x4cf0f0]
 def economy(x):
  nonlocal prefix
  prefix=x.state(org)
  if c['cut']:
   x.outcome={'kind':'RequiredChild','child':'4CF0F0'};raise h.Throw()
  prior(x)
 m.seams[0x4cf0f0]=economy
 m.call(0x57c450,org,[m.r(data+0x2f0)])
 return {'input':c,'initial':initial,'economy_prefix':prefix,'final':m.state(org),'outcome':m.outcome,'calls':m.calls}
def main():
 orig=h.P;rows=[]
 try:
  for label,image in [('original',orig),('v14',h.V)]:
   for pc,(layout,params),dirty,cut in itertools.product(h.CW,[('lay3',['inf','mg','co']),('lay3b',['inf','mg','co']),('lay1',['inf'])],[False,True],[False,True]):
    rows.append(dict(image=label,**run({'precision':pc,'layout':layout,'parameters':params,'dirty':dirty,'cut':cut},image)))
 finally:h.P=orig;n.P=orig
 half=len(rows)//2;assert [{k:v for k,v in r.items()if k!='image'}for r in rows[:half]]==[{k:v for k,v in r.items()if k!='image'}for r in rows[half:]]
 doc={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'executions':len(rows),
  'supplied_actor_data':n.ACTOR_DATA,'supplied_layouts':n.LAYOUTS,
  'inherited_producer_sha256':H(Path(n.__file__).read_bytes()),'inherited_harness_sha256':H(Path(h.__file__).read_bytes()),
  'seams':n.SEAM_DOC,'pins':[{'address':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]}
 doc=json.loads(json.dumps(doc)) #normalize inherited tuple metadata, not outputs
 p=HERE/'native-default-layout.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'result':'PASS','executions':len(rows),'pins':len(doc['pins']),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
