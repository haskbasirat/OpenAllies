"""Original4C2D90 saved-index lookup against supplied real layout memory.

Negative indices do not dereference missing layout memory. Nonnegative indices
need the actual table; out-of-range indices enter the original fatal branch.
Seams: inherited stream/CString allocator; fatal-message CString creation and
858AD0 exception sink. The range check/table lookup execute originally.
Page-zero ordinary accesses fault; observed instructions match original/v14.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-regiment-load'))
from native_regiment import Machine as Base,W,P,V,VISITED,H,Outcome
from unicorn.x86_const import UC_X86_REG_ECX

class Machine(Base):
    def hook(self,u,pc,size,arg):
        if pc==0x401b00:
            self.write(u.reg_read(UC_X86_REG_ECX),self.empty);self.ret(4);return
        super().hook(u,pc,size,arg)
    def run(self,index,table):
        owner=self.alloc(16)
        if table is not None:
            layout=self.alloc(64);ptr=self.alloc(max(4,len(table)*4))
            for i,x in enumerate(table):self.write(ptr+4*i,x)
            self.write(layout+0x34,ptr);self.write(layout+0x38,len(table));self.write(owner+8,layout)
        result=self.alloc(4);self.write(result,0xdeadbeef)
        try:
            self.call(0x4c2d90,owner,[self.world,result]);outcome='loaded'
        except Outcome as e:outcome=str(e)
        except Exception:
            if not self.null_accesses:raise
            outcome='missing_layout_fault'
        return {'outcome':outcome,'consumed':self.r.at,'result':self.read(result),'null_accesses':self.null_accesses}

def main():
    cases=[]
    for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
        for table in [None,[],[0],[101,0,303],[11,11]]:
            for index in [-2147483648,-2,-1,0,1,2,3,2147483647]:
                m=Machine(W(index),cw,0x41480000);native=m.run(index,table)
                assert native['consumed']==4
                cases.append({'precision':pc,'index':index,'formations':table,'native':native})
    doc={'scope':__doc__,'original_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'cases':cases,
        'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(VISITED.items())]}
    p=HERE/'native-formation-context.json';p.write_text(json.dumps(doc,indent=1)+'\n');print(json.dumps({'cases':len(cases),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
