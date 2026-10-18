"""Actual scalar/vector TCB3 loader, derived-control and evaluation helpers.

No restricted SDK source. Named heap and byte-stream transports only; native
registration, source field stores, endpoint/interior control math, key search
and cubic interpolation execute unchanged original/v14 at all3x87 precisions.
"""
from pathlib import Path
import sys,json,struct,math
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from probe_curves import Machine,h,ORIGINAL,F,V,H

def controls(rows,i,component=0,vector=False):
    row=rows[i];v=row[1+component]
    offset=4if vector else 2;t,b,c=row[offset:offset+3]
    if i==0:
        nxt=rows[1][1+component];prev=2*v-nxt;dtprev=dtnext=1
    elif i==len(rows)-1:
        prev=rows[i-1][1+component];nxt=2*v-prev;dtprev=dtnext=1
    else:
        prev=rows[i-1][1+component];nxt=rows[i+1][1+component]
        dtprev=row[0]-rows[i-1][0];dtnext=rows[i+1][0]-row[0]
    dp,dn=v-prev,nxt-v
    base=(1-t)*.5;plus=(1+b)*base;minus=(1-b)*base
    incoming=(minus*(1+c)*dp+plus*(1-c)*dn)*2/(dtprev+dtnext)*dtprev
    outgoing=(plus*(1+c)*dp+minus*(1-c)*dn)*2/(dtprev+dtnext)*dtnext
    return incoming,outgoing

def evaluate(rows,time,vector=False):
    count=3if vector else 1
    if len(rows)==1:return rows[0][1:1+count]
    left=0
    while left+1<len(rows)-1 and time>=rows[left+1][0]:left+=1
    right=left+1;u=(time-rows[left][0])/(rows[right][0]-rows[left][0])
    values=[]
    for n in range(count):
        a,b=rows[left][1+n],rows[right][1+n]
        m0=controls(rows,left,n,vector)[1];m1=controls(rows,right,n,vector)[0]
        d=b-a;values.append((((m0+m1-2*d)*u+(3*d-(2*m0+m1)))*u+m0)*u+a)
    return values

def load(m,rows,vector):
    m.call(0x865c10);m.call(0x864f70)
    m.stream_raw=b''.join(struct.pack('<'+'f'*len(r),*r)for r in rows);m.stream_at=0
    ptr=m.call(m.r32(0xb45de0 if vector else 0xb45dcc),0,[m.stream,len(rows)])
    assert m.stream_at==len(m.stream_raw)
    m.call(0x864910 if vector else 0x865940,0,[ptr,len(rows)])
    #Retain actual derived controls as well as sampled outputs.
    stride=0x50if vector else 0x20
    derived=[]
    for i,row in enumerate(rows):
        if vector:
            incoming=m.r32(ptr+i*stride+0x20,3);outgoing=m.r32(ptr+i*stride+0x2c,3)
        else:
            incoming=[m.r32(ptr+i*stride+0x18)];outgoing=[m.r32(ptr+i*stride+0x1c)]
        if len(rows)>1:
            for n,(a,b)in enumerate(zip(incoming,outgoing)):
                ia,ob=controls(rows,i,n,vector)
                assert abs(V(a)-ia)<2e-5 and abs(V(b)-ob)<2e-5,(rows,i,n,V(a),ia,V(b),ob)
        derived.append(dict(incoming_words=incoming,outgoing_words=outgoing))
    return ptr,derived

def samples(rows):
    if len(rows)==1:return [rows[0][0]-1,rows[0][0],rows[0][0]+1]
    result=[]
    for a,b in zip(rows,rows[1:]):
        result.extend([a[0]+u*(b[0]-a[0])for u in [0,.25,.5,.75,1]])
    return result+list(reversed(result))

def run(rows,vector,pc,image):
    m=Machine(image,pc);ptr,derived=load(m,rows,vector)
    cursor=m.alloc(4);m.w32(cursor,0);out=m.alloc(12)
    values=[]
    for time in samples(rows):
        if vector:
            m.call(0x861000,0,[out,F(time),ptr,3,len(rows),cursor]);words=m.r32(out,3)
        else:
            m.call(0x860890,0,[F(time),ptr,3,len(rows),cursor]);m.u.emu_start(m.store_entry,m.store_entry+6,count=1);words=[m.r32(m.float_out)]
        expected=evaluate(rows,time,vector)
        assert max(abs(V(a)-b)for a,b in zip(words,expected))<3e-5,(rows,time,words,expected)
        assert m.fpu_top()==0
        values.append(dict(time_word=F(time),output_words=words,cursor=m.r32(cursor)))
    return dict(controls=derived,samples=values)

def main():
    inputs=[
      [[0,2,.25,.5,-.5]],
      [[0,1,.25,.5,-.5],[4,-2,-.25,-.5,.75]],
      [[0,1,.25,.5,-.5],[1,3,-.25,-.5,.75],[2,-2,.5,.25,-.25]],
      [[0,1,.25,.5,-.5],[.25,3,-.25,-.5,.75],[4,-2,.5,.25,-.25],[4.5,5,0,-.25,.5]],
    ]
    cases=[]
    for pc in h.PRECISION:
        for scalar_rows in inputs:
            for vector in [False,True]:
                rows=[[r[0],r[1],-2*r[1]+1,.5*r[1]-2,*r[2:]]for r in scalar_rows]if vector else scalar_rows
                values=[run(rows,vector,pc,image)for image in [ORIGINAL,h.V]]
                assert values[0]==values[1]
                cases.append(dict(kind='vector_tcb3'if vector else'scalar_tcb3',pc=pc,rows=rows,native=values[0]))
    h.P=ORIGINAL
    doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),support_sha256=H((HERE/'probe_curves.py').read_bytes()),
       original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),cases=cases,paired_cases=len(cases),native_executions=2*len(cases),sample_pairs=sum(len(c['native']['samples'])for c in cases),
       pins=[dict(va=hex(a),bytes=b,kind='heap_transport_entry'if a==0x6f7810 else'executed')for a,b in sorted(h.VISITED.items())],
       semantics=['Serialized scalar3(time,value,T,B,C)→native+8value/+CT/+10B/+14C. Vector3 offsets+8value/+14T/+18B/+1CC.',
        'Interior incoming and outgoing controls use distinct dtPrev/dtNext factors2*dt/(dtPrev+dtNext). Endpoint values are reflected with synthetic dtPrev=dtNext=1.',
        'Incoming=((1-B)(1+C)*deltaPrev+(1+B)(1-C)*deltaNext)*(1-T)/2*2*dtPrev/(dtPrev+dtNext).',
        'Outgoing=((1+B)(1+C)*deltaPrev+(1-B)(1-C)*deltaNext)*(1-T)/2*2*dtNext/(dtPrev+dtNext).',
        'Actual cubic uses left.outgoing/right.incoming; no additional segment-duration multiplier. Cached forward/reverse key searches execute.',
        'Reference comparator is real-valued formula tolerance check; native output fixture remains rawwords/allPC, not bit-exact Rust parity.'],
       limits=['Finite strict increasing lab keys only; no authoredsourceTCB or whole NIF/controller proof.', 'No quaternionTCB/SQUAD or scalar/vectorout-of-range/emptykey native-domain acceptance.'])
    p=HERE/'native-tcb3.json'
    with p.open('x',encoding='utf-8',newline='\n')as f:json.dump(doc,f,indent=2);f.write('\n')
    print(json.dumps(dict(passed=True,paired_cases=len(cases),native_executions=doc['native_executions'],sample_pairs=doc['sample_pairs'],pins=len(doc['pins']),sha256=H(p.read_bytes()))))
if __name__=='__main__':main()
