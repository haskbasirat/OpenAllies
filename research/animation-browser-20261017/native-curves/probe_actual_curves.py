"""Native quadratic/Euler replay from original building death NIF key bytes.

Original extracted NIF is hash-checked against independently RWD-verified source
inventory. PyFFI supplies source fields, real native loader consumes encoded
time/value/forward/backward payloads, not outputs from a preview sampler.
No restricted SDK source is loaded; no asset/cache/crate/build is modified.
"""
from pathlib import Path
import sys,os,time,json,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];GAME=ROOT.parent
sys.path.insert(0,str(HERE));from probe_curves import Machine,h,F,V,H,ORIGINAL
os.environ['NIFXMLPATH']=str(GAME/'reconstruction/tools/convert_models_vendor/pyffi/formats/nif/nifxml')
sys.path.insert(0,str(GAME/'reconstruction/tools/convert_models_vendor'))
if not hasattr(time,'clock'):time.clock=time.perf_counter
from pyffi.formats.nif import NifFormat

def keys(group):
    return [[float(k.time),float(k.value),float(k.forward),float(k.backward)]for k in group.keys]

def main():
    inv_path=ROOT/'research/animation-browser-20261017/inventory/original-source-catalog.json'
    inv=json.loads(inv_path.read_bytes());by_path={a['source']:a for a in inv['assets']}
    selected=['Buildings/GBR/GBRCorps/GBRCorpsDeath.nif','Buildings/GBR/GBRHQInfantry/GBRHQInfantryDeath.nif','Buildings/USA/USAHQInfantry/USAHQInfantryDeath.nif']
    sources=[];cases=[]
    for source in selected:
        path=GAME/'reconstruction/assets/original/Data'/source
        data=path.read_bytes();assert H(data)==by_path[source]['sha256']
        nif=NifFormat.Data()
        with path.open('rb')as f:nif.read(f)
        blocks=list(nif.get_global_iterator());chosen=None
        euler_orders=[]
        for n,block in enumerate(blocks):
            if isinstance(block,NifFormat.NiKeyframeData)and int(block.rotation_type)==4:
                order=F(float(block.unknown_float));euler_orders.append(order)
                if chosen is None and all(int(axis.interpolation)==2 and len(axis.keys)>1 for axis in block.xyz_rotations)and int(block.scales.interpolation)==2 and len(block.scales.keys)>1:
                    chosen=(n,block,order)
        assert chosen is not None
        n,block,order=chosen;assert 0<=order<9
        axes=[keys(axis)for axis in block.xyz_rotations];scales=keys(block.scales)
        lo=max(group[0][0]for group in axes+[scales]);hi=min(group[-1][0]for group in axes+[scales]);assert lo<hi
        inputs=dict(source=source,source_sha256=H(data),data_iterator_index=n,nif_version=int(nif.version),order=order,axes=axes,scales=scales,defined_interval=[lo,hi],all_euler_order_words=euler_orders)
        sources.append(inputs)
        for pc in h.PRECISION:
            for u in [0,.125,.5,.875,1]:
                timestamp=V(F(lo+(hi-lo)*u));outputs=[]
                for image in [ORIGINAL,h.V]:
                    machine=Machine(image,pc)
                    rotation,cursors=machine.euler(axes,timestamp,order)
                    scale_keys=machine.load('scalar',scales)
                    scale,cursor=machine.scalar(scale_keys,len(scales),timestamp)
                    assert all(__import__('math').isfinite(V(word))for word in rotation+[scale])
                    outputs.append(dict(rotation_wxyz_words=rotation,axis_cursor_words=cursors,scale_word=scale,scale_cursor=cursor))
                assert outputs[0]==outputs[1]
                cases.append(dict(source_index=len(sources)-1,pc=pc,time_word=F(timestamp),u=u,native=outputs[0]))
    h.P=ORIGINAL
    doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),support_sha256=H((HERE/'probe_curves.py').read_bytes()),source_inventory_sha256=H(inv_path.read_bytes()),
        original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),sources=sources,cases=cases,paired_rows=len(cases),native_executions=2*len(cases),
        pins=[dict(va=hex(a),bytes=b,kind='heap_transport_entry'if a==0x6f7810 else'executed')for a,b in sorted(h.VISITED.items())],
        limits=['Only selected authentic quadratic scale/Euler data blocks, five defined-range timestamps and3PC; not every controller/node/asset.',
          'Native interpolation loader/helper parity excludes complete NIF/controller lifecycle, event/visibility/material/mesh/skin/render/browser integration.',
          'UnknownFloat rawDWORD retained as real Euler order; sourceorders are inventoried, never forced0.'])
    p=HERE/'native-curves-actual.json'
    with p.open('x',encoding='utf-8',newline='\n')as f:json.dump(doc,f,indent=2);f.write('\n')
    print(json.dumps(dict(passed=True,sources=len(sources),paired_rows=len(cases),native_executions=2*len(cases),pins=len(doc['pins']),sha256=H(p.read_bytes()))))
if __name__=='__main__':main()
