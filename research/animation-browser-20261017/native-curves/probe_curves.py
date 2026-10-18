"""Actual original/v14 Ni curve helpers, independent of restricted SDK code.

Only lab heap/stream-byte transport is intercepted. Actual registration, key
load, quadratic evaluation, Euler constructor/axis quaternion/composition run.
No mesh/render/controller lifecycle or whole NIF load is certified here.
"""
from pathlib import Path
import sys,struct,json,math,hashlib,itertools
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
V=lambda x:struct.unpack('<f',struct.pack('<I',x))[0]
H=lambda x:hashlib.sha256(x).hexdigest()
ORDERS=[[0,1,2],[0,2,1],[1,2,0],[1,0,2],[2,0,1],[2,1,0],[0,1,0],[1,2,1],[2,0,2]]

class Machine(h.Machine):
    def __init__(self,image,pc):
        h.P=image
        super().__init__(pc)
        self.stream_reads=[];self.alloc_sizes=[];self.stream_raw=b'';self.stream_at=0
        self.call(0x860f10) #Actual scalar quadratic registration.
        self.call(0x862040) #Actual vector quadratic registration.
        self.call(0x8635a0) #Actual Euler registration.
        self.seams[0x6f7810]=self.allocate
        self.reader_entry=self.stub_next;self.stub_next+=16
        self.seams[self.reader_entry]=self.read
        self.stream=self.alloc(0x170);child=self.alloc(16);vt=self.alloc(16)
        self.w32(self.stream+0x15c,child);self.w32(child,vt);self.w32(vt+8,self.reader_entry)
        self.float_out=self.alloc(4)
        self.store_entry=self.stub_next;self.stub_next+=16
        #Float return storage transport only; not a substituted native body.
        self.u.mem_write(self.store_entry,b'\xd9\x1d'+struct.pack('<I',self.float_out))
    def allocate(self,m):
        n=m.arg();self.alloc_sizes.append(n);m.ret(value=m.alloc(n))
    def read(self,m):
        destination,size=m.arg(0),m.arg(1)
        raw=self.stream_raw[self.stream_at:self.stream_at+size]
        assert len(raw)==size
        self.u.mem_write(destination,raw);self.stream_at+=size
        self.stream_reads.append(size);m.ret(pop=8,value=size)
    def load(self,kind,rows):
        self.stream_raw=b''.join(struct.pack('<'+'f'*len(row),*row)for row in rows)
        self.stream_at=0
        entry=0x860e30 if kind=='scalar' else self.r32(0xb45ddc)
        keys=self.call(entry,0,[self.stream,len(rows)])
        assert self.stream_at==len(self.stream_raw)
        if kind=='vector':self.call(0x861900,0,[keys,len(rows)])
        return keys
    def scalar(self,keys,count,time,index=0):
        cursor=self.alloc(4);self.w32(cursor,index)
        self.call(0x860890,0,[F(time),keys,2,count,cursor])
        self.u.emu_start(self.store_entry,self.store_entry+6,count=1)
        assert self.fpu_top()==0
        return self.r32(self.float_out),self.r32(cursor)
    def vector(self,keys,count,time,index=0):
        cursor=self.alloc(4);out=self.alloc(12);self.w32(cursor,index)
        self.call(0x861000,0,[out,F(time),keys,2,count,cursor])
        assert self.fpu_top()==0
        return self.r32(out,3),self.r32(cursor)
    def euler(self,axes,time,order):
        key=self.alloc(0x4c);self.call(0x862f50,key)
        for n,rows in enumerate(axes):
            if not rows:continue
            keys=self.load('scalar',rows)
            self.w32(key+0x18+4*n,len(rows));self.w32(key+0x24+4*n,2);self.w32(key+0x34+4*n,keys)
        self.w32(key+0x30,order)
        out=self.alloc(16);cursor=self.alloc(4)
        self.call(0x862170,0,[out,F(time),key,4,1,cursor])
        assert self.fpu_top()==0
        return self.r32(out,4),self.r32(key+0x40,3)

def hermite(u,left,right):
    #Wire order: time,value,forward,backward. Native consumes left.backward,
    #right.forward. No duration multiplier. This is a mathematical comparator,
    #not claimed exact x87 emulation.
    p0,p1=left[1],right[1];m0,m1=left[3],right[2]
    delta=p1-p0
    return (((m0+m1-2*delta)*u+(3*delta-(2*m0+m1)))*u+m0)*u+p0

def qmul(a,b):
    w,x,y,z=a;v,i,j,k=b
    return [w*v-x*i-y*j-z*k,w*i+x*v+y*k-z*j,w*j+y*v+z*i-x*k,w*k+z*v+x*j-y*i]
def reference_euler(angles,order):
    rotations=[]
    for axis,angle in enumerate(angles):
        q=[math.cos(angle*.5),0,0,0];q[axis+1]=math.sin(angle*.5);rotations.append(q)
    out=[1,0,0,0]
    for axis in ORDERS[order]:out=qmul(rotations[axis],out)
    return out

def main():
    cases=[]
    scalar_inputs=[[[0,1,17,3],[2,5,-2,19]],[[0,-2,7,-1],[8,3,2,11]],[[1,4,5,-3],[1.25,-1,6,-7]]]
    vector_inputs=[[[0,1,2,-1,17,18,19,3,-2,1],[2,5,-1,3,-2,4,6,20,21,22]]]
    for pc in h.PRECISION:
        for rows in scalar_inputs:
            for u in [0,.125,.5,.875,1]:
                time=rows[0][0]+u*(rows[1][0]-rows[0][0]);outputs=[]
                for image in [ORIGINAL,h.V]:
                    m=Machine(image,pc);keys=m.load('scalar',rows)
                    assert m.r32(keys+0xc)==F(rows[0][2]) and m.r32(keys+0x10)==F(rows[0][3])
                    outputs.append(m.scalar(keys,len(rows),time))
                assert outputs[0]==outputs[1]
                assert abs(V(outputs[0][0])-hermite(u,*rows))<1e-5
                cases.append(dict(kind='scalar2',pc=pc,rows=rows,u=u,time=time,output=outputs[0]))
        for rows in vector_inputs:
            for u in [0,.125,.5,.875,1]:
                time=rows[0][0]+u*(rows[1][0]-rows[0][0]);outputs=[]
                for image in [ORIGINAL,h.V]:
                    m=Machine(image,pc);keys=m.load('vector',rows);outputs.append(m.vector(keys,len(rows),time))
                assert outputs[0]==outputs[1]
                expected=[hermite(u,[rows[0][0],rows[0][1+n],rows[0][4+n],rows[0][7+n]],[rows[1][0],rows[1][1+n],rows[1][4+n],rows[1][7+n]])for n in range(3)]
                assert max(abs(V(a)-b)for a,b in zip(outputs[0][0],expected))<1e-5
                cases.append(dict(kind='vector2',pc=pc,rows=rows,u=u,time=time,output=outputs[0]))
        axes=[[[0,.2,1,.4],[2,.8,-.3,2]],[[0,-.3,2,.2],[2,.7,.5,3]],[[0,.4,3,-.4],[2,-.2,.3,4]]]
        for order in range(9):
            for time in [0,.5,1,2]:
                outputs=[Machine(image,pc).euler(axes,time,order)for image in [ORIGINAL,h.V]]
                assert outputs[0]==outputs[1]
                angles=[hermite(time/2,*rows)for rows in axes]
                expected=reference_euler(angles,order)
                assert max(abs(V(a)-b)for a,b in zip(outputs[0][0],expected))<2e-6
                cases.append(dict(kind='euler2',pc=pc,axes=axes,time=time,order=order,output=outputs[0],reference=expected))
    h.P=ORIGINAL
    doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),cases=cases,
        paired_rows=len(cases),native_executions=2*len(cases),pins=[dict(va=hex(a),bytes=b,kind='heap_transport_entry'if a==0x6f7810 else'executed')for a,b in sorted(h.VISITED.items())],
        orders=ORDERS,axis_globals=[dict(va=hex(a),bytes=ORIGINAL.range(a,12).hex())for a in [0xb3206c,0xb32078,0xb32084]],
        semantics=['Scalar/vector serialized type2: time,value,forward,backward; native uses left.backward andright.forward directly, no duration scaling.',
          'Euler key+30 is serialized raw DWORD order, not ignorable float;0..8 table as recorded; order0=qZ*qY*qX in native WXYZ quaternion convention.',
          'Valid finite increasing two-key input domain only; scalar/vector generic wrapper endpoint/clamp/cache/reverse cases will need broader proof before claims.'])
    p=HERE/'native-curves-lab.json'
    with p.open('x',encoding='utf-8',newline='\n')as f:json.dump(doc,f,indent=2);f.write('\n')
    print(json.dumps(dict(passed=True,paired_rows=len(cases),native_executions=2*len(cases),pins=len(doc['pins']),sha256=H(p.read_bytes()))))
if __name__=='__main__':main()
