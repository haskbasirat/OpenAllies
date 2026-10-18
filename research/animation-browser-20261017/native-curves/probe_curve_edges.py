"""Separate successor proof: empty Euler axes and actual scale store clamp.

Prior168/45-row fixtures are unchanged. Empty-axis rows execute full Euler
helper; scale rows execute actual8C02CE..8C02EE with a lab child float return.
That scalar-return injection is explicit transport, not whole-controller proof.
"""
from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from probe_curves import Machine,h,ORIGINAL,F,V,H,reference_euler,hermite
from unicorn.x86_const import UC_X86_REG_ESI

def main():
    cases=[]
    original_axes=[[[0,.2,1,.4],[2,.8,-.3,2]],[[0,-.3,2,.2],[2,.7,.5,3]],[[0,.4,3,-.4],[2,-.2,.3,4]]]
    for pc in h.PRECISION:
        for empty in [[0,1,2],[0],[1],[2]]:
            axes=[[]if n in empty else rows for n,rows in enumerate(original_axes)]
            for order in [0,3,8]:
                for time in [0,1,2]:
                    outputs=[Machine(image,pc).euler(axes,time,order)for image in [ORIGINAL,h.V]]
                    assert outputs[0]==outputs[1]
                    angles=[0if not rows else hermite(time/2,*rows)for rows in axes]
                    reference=reference_euler(angles,order)
                    assert max(abs(V(word)-v)for word,v in zip(outputs[0][0],reference))<2e-6
                    cases.append(dict(kind='empty_euler',pc=pc,axes=axes,order=order,time=time,native=outputs[0],reference=reference))
        for value in [-10,-.5,-0.0,0,.5,2]:
            outputs=[]
            for image in [ORIGINAL,h.V]:
                m=Machine(image,pc);controller=m.alloc(0x80);target=m.alloc(0x80);word=m.alloc(4)
                m.w32(controller+0x34,target);m.w32(word,F(value));m.w32(target+0x68,0xa5a5a5a5)
                m.u.mem_write(m.store_entry,b'\xd9\x05'+struct.pack('<I',word))
                m.u.emu_start(m.store_entry,m.store_entry+6,count=1)
                m.u.reg_write(UC_X86_REG_ESI,controller)
                m.u.emu_start(0x8c02ce,0x8c02ee,count=20)
                outputs.append(m.r32(target+0x68));assert m.fpu_top()==0
            assert outputs[0]==outputs[1]==F(max(0.0,value))
            cases.append(dict(kind='controller_scale_store',pc=pc,input_word=F(value),native_word=outputs[0]))
    h.P=ORIGINAL
    doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),support_sha256=H((HERE/'probe_curves.py').read_bytes()),
        cases=cases,paired_rows=len(cases),native_executions=2*len(cases),pins=[dict(va=hex(a),bytes=b,kind='heap_transport_entry'if a==0x6f7810 else'executed')for a,b in sorted(h.VISITED.items())],
        conclusions=['Empty Euler axis count is literal0 angle, not bind fallback. All-three empty axes produce identity quaternion.',
          'Actual keyframe controller finite scale store is max(0,value); -0 becomes+0. Curve helper output itself is not clamped.'],
        limits=['Finite scalar values only; no NaN/Inf acceptance or whole-controller readiness.', 'No original source curve values inferred from controlled edge probes.'])
    out=HERE/'native-curve-edges.json'
    with out.open('x',encoding='utf-8',newline='\n')as f:json.dump(doc,f,indent=2);f.write('\n')
    print(json.dumps(dict(passed=True,paired_rows=len(cases),native_executions=doc['native_executions'],pins=len(doc['pins']),sha256=H(out.read_bytes()))))
if __name__=='__main__':main()
