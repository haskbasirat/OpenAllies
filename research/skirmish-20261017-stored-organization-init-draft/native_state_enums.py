"""Real48A440/48A540 enum constructors,47A360/add/hash and497220 lookup.
Inherited counted-CString/allocator transports and named96CAD6 __wcsicmp
ASCII transport (CRT thread locale is not initialized by this isolated lab).
No live manager is copied from captured registration/current-value data.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
H=lambda b:hashlib.sha256(b).hexdigest()
def main():
 machines=[];orig=h.P
 try:
  for label,image in [('original',orig),('v14',h.V)]:
   h.P=image
   for pc in h.CW:
    m=h.Machine(b'',pc)
    def compare(x):
     a=x.read_wide(x.arg());b=x.read_wide(x.arg(1));assert a.isascii()and b.isascii()
     a=a.upper();b=b.upper();x.ret(0,(a>b)-(a<b))
    m.seam(0x96cad6,compare)
    m.call(0x48a440,0xb3ad38,[1]);m.call(0x48a540,0xb3ad5c,[1]);queries=[]
    for kind,base,names in [('formation',0xb3ad48,['DESIRED','BROKEN','COMBAT','RETREAT','desired','Combat','unknown','', 'AGGRESSIVE']),
                            ('behavior',0xb3ad6c,['PASSIVE','AGGRESSIVE','DESIRED','passive','Aggressive','desired','COMBAT','unknown',''])]:
     for name in names:
      p=m.alloc(4);m.w(p,m.cstring(name));r=m.call(0x497220,base,[p])
      value=None if not r or not m.r(r) else m.r(m.r(r));queries.append({'kind':kind,'name':name,'value':value})
    machines.append({'image':label,'precision':pc,'queries':queries})
 finally:h.P=orig
 assert [r['queries']for r in machines[:3]]==[r['queries']for r in machines[3:]]
 seams=set(m.seams)
 doc={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'machines':machines,
  'inherited_harness_sha256':H(Path(h.__file__).read_bytes()),
  'pins':[{'address':hex(a),'bytes':b,'kind':'declared_transport_entry'if a in seams else'executed'}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-state-enums.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'result':'PASS','machines':len(machines),'ctor_calls':12,'queries':108,'pins':len(doc['pins']),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
