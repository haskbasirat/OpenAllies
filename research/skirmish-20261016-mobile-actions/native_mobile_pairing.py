"""Original4605B0 pairing plus473AD0 prepend/475230 deferred traversal.

4BDBA0 is an explicit identity-lookup seam (registered ActorData or null).
Data errors410C40 stop the case; prior native pointer writes remain visible.
Allocator/CString seams come from the conversion machine. No pairing,
validation, arithmetic, callback wrapper or deferred-list traversal is mocked.
"""
from pathlib import Path
import sys,json,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'))
import native_machine as nm
from native_machine import Machine,P,V,VISITED
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()

def f(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def spec(**kw):
    d=dict(body=True,mobile=True,element=False,offset=[0,0],radius=f(1))
    d.update(kw);return d
def run(c):
    m=Machine(c['precision']);g=m.alloc(0x900);m.w(0xb36530,g)
    defs={name:m.alloc(0x600)for name in c['definitions']}
    for name,s in c['definitions'].items():
        p=defs[name];m.w(p+8,m.cstring(name))
        m.w(p+0x174,int(s['body'])|(int(s['element'])<<8)|(int(s['mobile'])<<19))
        m.w(p+0x538,s['offset'][0]);m.w(p+0x53c,s['offset'][1]);m.w(p+0x54,s['radius'])
    calls=[]
    def lookup(m):
        name=m.read_wide(m.arg(0));owner=m.u.reg_read(UC_X86_REG_ESI)
        calls.append({'owner':next((k for k,v in defs.items()if v==owner),None),'lookup':name,'category':m.arg(1)})
        m.ret(8,defs.get(c.get('aliases',{}).get(name,name),0))
    m.seam(0x4bdba0,lookup)
    m.seam(0x410c40,lambda m:m.throw('DataError',format=m.read_wide(m.arg(0))))
    for owner,target in c['links']:
        cell=m.alloc(4);m.w(cell,m.cstring(target));ref=m.alloc(16)
        m.call(0x473ad0,ref,[cell],4)
        m.w(ref,0xa0347c);m.w(ref+12,defs[owner])
    m.outcome=None;m.call(0x475230,g+0x824,[],0)
    label=lambda p:next((n for n,a in defs.items()if a==p),None)
    return {'error':m.outcome,'calls':calls,'definitions':{name:{'unpack_to':label(m.r(p+0x534)),
            'pack_to':label(m.r(p+0x548))}for name,p in defs.items()},'deferred_empty':m.r(g+0x824)==0}
def cases():
    out=[]
    for pc in ['Single24','Double53','Extended64']:
        for label,src,tgt in [
            ('center',spec(),spec()),('edge',spec(offset=[f(.6),f(.8)]),spec()),
            ('outside',spec(offset=[f(2),0]),spec()),('negative_radius',spec(),spec(radius=f(-1))),
            ('missing_body',spec(),spec(body=False)),('missing_mobile',spec(),spec(mobile=False)),
            ('source_element_only',spec(element=True),spec()),('target_element_only',spec(),spec(element=True)),
            ('both_elements',spec(element=True),spec(element=True)),
            ('nan_offset',spec(offset=[0x7fc12345,0]),spec()),('nan_radius',spec(),spec(radius=0x7fc12345)),
            ('inf_offset',spec(offset=[0x7f800000,0]),spec()),('inf_radius',spec(),spec(radius=0x7f800000)),
            ('both_inf',spec(offset=[0x7f800000,0]),spec(radius=0x7f800000))]:
            out.append({'label':pc+'_'+label,'precision':pc,'definitions':{'source':src,'target':tgt},'links':[['source','target']]})
        out.append({'label':pc+'_unknown','precision':pc,'definitions':{'source':spec()},'links':[['source','missing']]})
    for links in [[['first','target'],['second','target']], [['second','target'],['first','alias']],
                  [['first','target'],['second','missing']], [['first','target'],['target','second']]]:
        out.append({'label':'ordered_'+str(len(out)),'precision':'Double53','definitions':{x:spec()for x in ['first','second','target']},
                    'links':links,'aliases':{'alias':'target'}})
    return out
def main():
    rows=[{'input':c,'output':run(c)}for c in cases()]
    for a,b in VISITED.items():assert P.range(a,len(bytes.fromhex(b)))==V.range(a,len(bytes.fromhex(b))),hex(a)
    doc={'schema':1,'passed':True,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),
         'base_machine_sha256':H(Path(nm.__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),
         'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())], 'cases':rows}
    p=HERE/'native-mobile-pairing.json'
    if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
    else:assert json.loads(p.read_text())==doc,'native pairing fixture differs'
    print(json.dumps({'cases':len(rows),'instructions':len(VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
