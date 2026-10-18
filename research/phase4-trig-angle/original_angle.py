"""Original signed angle with explicit incoming sticky precision-exception flag.

Uses a pinned local copy of the root machine; no original source mutation. The
original angle/CRT executes without decision hooks. Extra code hooks observe
the CRT qword rounding branch only.
"""
from pathlib import Path
import hashlib,json,math,random,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
from native_trig_base import TrigMachine,original,modified,bits
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_FPSW
sha=lambda b:hashlib.sha256(b).hexdigest()


def main():
 source=json.loads((ROOT/'research/phase4-trig/native-trig-vectors.json').read_text())
 assert source['producer_sha256']==sha((HERE/'native_trig_base.py').read_bytes())
 pairs=[(row['from_bits'],row['to_bits']) for row in source['angles'] if row['precision']=='Single24']
 # Asymmetric signs, subnormals, endpoint neighbors and oversized finite
 # vectors test native clamping and status-setting arithmetic explicitly.
 pairs += [([0x3f800000,0],[0xbf800000,x]) for x in [0,0x80000000,1,0x80000001,0x3f7fffff,0xbf7fffff,0x3f800000,0xbf800000,0x3f800001,0xbf800001]]
 pairs += [([bits(1),bits(1)],[bits(2),bits(-2)]),([bits(-1),bits(-1)],[bits(2),bits(-2)])]
 randomizer=random.Random(0x445400)
 for _ in range(200):
  a=randomizer.uniform(-math.pi,math.pi);b=randomizer.uniform(-math.pi,math.pi)
  pairs.append(([bits(math.cos(a)),bits(math.sin(a))],[bits(math.cos(b)),bits(math.sin(b))]))
 rows=[]
 for name,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  for pending in [False,True]:
   m=TrigMachine(cw);m.u.context_restore(m.initial);m.u.reg_write(UC_X86_REG_FPSW,0x20 if pending else 0);m.initial=m.u.context_save()
   trace=[]
   m.u.hook_add(UC_HOOK_CODE,lambda u,pc,size,user:trace.append(pc) if pc in [0x977837,0x977953] else None)
   for a,b in pairs:
    trace.clear();result=m.angle(a,b)
    rows.append(dict(precision=name,precision_exception_pending=pending,from_bits=a,to_bits=b,result_bits=result,rounded_through_f64=0x977837 in trace))
 diffs=[]
 for mode in ['Single24','Double53','Extended64']:
  clean=[r for r in rows if r['precision']==mode and not r['precision_exception_pending']]
  dirty=[r for r in rows if r['precision']==mode and r['precision_exception_pending']]
  for a,b in zip(clean,dirty):
   if a['result_bits']!=b['result_bits']:diffs.append({'clean':a,'pending':b})
 ranges=dict(angle=(0x445400,0x445495),asin=(0x96d780,0x96d843),helpers=(0x977837,0x977955),half_pi80=(0xb359ba,0xb359c4),f32_pi=(0x9ffff8,0xa00000))
 pinned={}
 for name,(a,b) in ranges.items():
  data=original.range(a,b-a);assert data==modified.range(a,b-a);pinned[name]=dict(start=hex(a),end_exclusive=hex(b),sha256=sha(data),bytes=data.hex())
 report=dict(schema=1,passed=True,producer_sha256=sha(Path(__file__).read_bytes()),root_producer_sha256=source['producer_sha256'],original_sha256=sha(original.b),accepted_sha256=sha(modified.b),ranges=pinned,cases=rows,
             incoming_pe_output_differences=diffs,limits=['No whole-engine FPU lifecycle inference; incomingprecisionflag is explicit.','Finite supported angle inputs; outputobservedthroughfinalbinary32store.','FPATAN/FSQRT in Unicorn still require hostCPU comparison; this report alone is not hardwareparity.'])
 p=HERE/'native-angle-status-vectors.json';p.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'cases':len(rows),'pending_pe_output_differences':len(diffs),'sha256':sha(p.read_bytes())}))


if __name__=='__main__':main()
