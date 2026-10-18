"""Native464BD0 SettlementComponent optional-child branch; explicit lookup seam."""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_prefix import Machine,original,modified,sha,PRECISIONS,execute,remap
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EDI

def main():
    rows=[];pins={}
    for name,image in [('original',original),('v14',modified)]:
      for pc in PRECISIONS:
        for present in [False,True]:
            m=Machine(pc);remap(m,image);actor=m.alloc(0x500);node=m.alloc(0x20);child=m.alloc(0x20)
            old_mask=0xa5a50080;m.w(actor+0x174,old_mask);events=[]
            def lookup(z):
                assert z.ecx()==node and z.arg(2)==0
                p=z.arg(1);units=[]
                while (c:=int.from_bytes(z.u.mem_read(p,2),'little')):units.append(c);p+=2
                field=bytes().join(c.to_bytes(2,'little') for c in units).decode('utf-16-le')
                assert field=='SettlementComponent'
                events.append({'operation':'GetChild','name':field,'optional_argument':0,'present':present})
                z.ret(child if present else 0,pop=8)
            def complete(z):
                assert z.ecx()==actor and z.arg(1)==child
                events.append({'operation':'4640F0','mask':z.r(actor+0x174)})
                z.ret(pop=4)
            m.seam(0x5b4410,lookup);m.seam(0x4640f0,complete)
            assert execute(m,0x4679d0,0x4679fc,{UC_X86_REG_EBP:actor,UC_X86_REG_EDI:node}) is None
            assert len(events)==(2 if present else 1)
            assert m.r(actor+0x174)==(old_mask|(0x4000 if present else 0))
            rows.append({'image':name,'precision':pc,'present':present,'initial_mask':old_mask,'result_mask':m.r(actor+0x174),'events':events})
            pins.update(m.visited)
    out={'schema':1,'passed':True,'scope':'Original ActorData464BD0 optional SettlementComponent child branch. Child lookup and invoked4640F0 are declared seams; this proves gating/call/bit order only. Combined with same-source registration census, fitting is not reached by that absent component; it does not complete unrelated source arithmetic.','original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'cases':rows,'instructions':[{'address':hex(a),'bytes':b.hex()} for a,b in sorted(pins.items())]}
    (HERE/'native-presence-branch.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
    print('passed',len(rows),'optional-presence cases;',len(pins),'original/v14 instruction pins')
if __name__=='__main__':main()
