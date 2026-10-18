"""Original TGI parser and 4C4CF0 TechnologyProperty list producer.
Property Find is a recorded canonical fixture mapping; all node selection,
GetString reads, lookup failure throws, allocations and prepend order are real.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-layouts'))
from layout_machine import Machine,P,V,H,native_windows
from unicorn.x86_const import *

TEMPLATE='[template Thing]\n{ string IDS\n string category = ACTOR\n [optional StructureComponent]\n { [list TechnologyProperty]\n string property_IDS\n }\n}\n'
class TechMachine(Machine):
    def __init__(self):
        self.properties={};super().__init__()
        for name in ('first','second','third'):
            self.properties[name]=self.alloc(0x20)
    def hook(self,u,pc,size,user):
        if pc==0x463fa0:
            name=self.raw_wstr(self.arg()); target=self.properties.get(name.lower(),0)
            self.recorded.append({'event':'property_find','name':name,'found':bool(target)})
            self.ret(target,pop=4);return
        if pc==0x405d60:
            self.recorded.append({'event':'exception_text_format','format':self.raw_wstr(self.arg(1))})
            self.w(self.arg(),self.cstring('<diagnostic formatter seam>'));self.ret();return
        super().hook(u,pc,size,user)

def main():
    rows=[];visited={}
    for names in ([],['first'],['first','second','third'],['first','FIRST','second'],['first','missing','third']):
        source='[Thing template=Thing]\n{ IDS = subject\n [StructureComponent]\n {\n'+''.join('[TechnologyProperty]\n property_IDS = '+n+'\n' for n in names)+'}\n}\n'
        m=TechMachine();m.parse_file('template.tgi',TEMPLATE.encode()); parsed=m.parse_file('subject.tgi',source.encode())
        actor=m.children(parsed['root'])[0]; component=m.thiscall(0x5b4410,actor,[m.cstring('StructureComponent'),0])['eax']
        dst=m.alloc(8);m.w(dst,0);m.w(dst+4,0)
        label=m.alloc(4);m.w(label,m.cstring('TechnologyProperty'))
        try:
            result=m.cdecl(0x4c4cf0,[component,label,dst])
        except AssertionError:
            # cdecl helper expects normal return; original throw stops at its
            # shared C++ exception observation seam instead.
            assert m.u.reg_read(UC_X86_REG_EIP)==0x96bef5 and m.last_throw
        ids=[];p=m.r(dst);rev={v:k for k,v in m.properties.items()}
        while p:ids.append(rev[m.r(p)]);p=m.r(p+4)
        row={'source':source,'template':TEMPLATE,'names':names,'native_ids':ids,'thrown':m.last_throw,'seams':m.recorded};rows.append(row);visited.update(m.visited)
        print(names,ids,m.last_throw,flush=True)
    path=HERE/'native-technology-source.json';path.write_text(json.dumps({'schema':1,'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),'cases':rows,'native_windows':native_windows(visited)},indent=2)+'\n')
    print(path,H(path.read_bytes()))
if __name__=='__main__':main()
