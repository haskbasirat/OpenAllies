"""Read every original NIF/KF and TGI animation declaration; write evidence only."""
from __future__ import annotations
import collections,hashlib,json,math,os,sys
from pathlib import Path
WORKSPACE=Path(__file__).resolve().parents[3];RECON=WORKSPACE.parent/'reconstruction'
os.environ['NIFXMLPATH']=str(RECON/'tools/convert_models_vendor/pyffi/formats/nif/nifxml')
sys.path.insert(0,str(RECON/'tools'))
from convert_models import NifFormat,ORIGINAL,string
from convert_models_animations import read_nif
import archive_catalog as tgi
OUT=WORKSPACE/'research/phase3-animation';OUT.mkdir(parents=True,exist_ok=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def finite(value):
    value=float(value)
    return value if math.isfinite(value) else None
def key_value(v):
    if isinstance(v,(int,float)):return float(v)
    if hasattr(v,'w'):return [float(v.x),float(v.y),float(v.z),float(v.w)]
    if hasattr(v,'x'):return [float(v.x),float(v.y),float(v.z)]
    return string(v)
def zero(v):return all(x==0 for x in v) if isinstance(v,list) else v==0
def channel(group,kind):
    keys=list(group);values=[key_value(k.value) for k in keys]
    constant=not keys or all(v==values[0] for v in values)
    if kind in (2,3):
        constant=constant and all(zero(key_value(getattr(k,a,0))) for k in keys for a in ['forward','backward']) and kind!=3
    return {'kind':int(kind),'keys':len(keys),'constant_with_zero_tangents':constant,
        'first_time':finite(keys[0].time) if keys else None,'last_time':finite(keys[-1].time) if keys else None}
def controller(c):
    row={'class':type(c).__name__,'target':string(getattr(getattr(c,'target',None),'name',b'')),
        'start':finite(c.start_time),'stop':finite(c.stop_time),'frequency':finite(c.frequency),'phase':finite(c.phase),'flags':int(c.flags)}
    data=getattr(c,'data',None);row['data_class']=type(data).__name__
    if isinstance(data,NifFormat.NiKeyframeData):
        row['translation']=channel(data.translations.keys,int(data.translations.interpolation))
        row['scale']=channel(data.scales.keys,int(data.scales.interpolation))
        if int(data.rotation_type)==4:row['euler_xyz']=[channel(axis.keys,int(axis.interpolation)) for axis in data.xyz_rotations]
        else:row['quaternion']=channel(data.quaternion_keys,int(data.rotation_type))
    elif hasattr(data,'keys'):row['keys']=len(data.keys)
    return row
def main():
    files=sorted((p for p in ORIGINAL.rglob('*') if p.suffix.lower() in ('.nif','.kf')),key=lambda p:p.relative_to(ORIGINAL).as_posix().casefold())
    records=[];errors=[]
    for index,p in enumerate(files):
        row={'source':p.relative_to(ORIGINAL).as_posix(),'size':p.stat().st_size,'sha256':sha(p)}
        try:
            data=read_nif(p);row['version']=data.version;row['blocks']=len(data.blocks)
            row['controllers']=[controller(c) for c in data.blocks if isinstance(c,NifFormat.NiTimeController)]
            names=collections.Counter(string(b.name) for b in data.blocks if isinstance(b,NifFormat.NiAVObject))
            row['node_count']=sum(names.values());row['duplicate_node_names']={k:v for k,v in names.items() if v>1}
            row['sequences']=[{'name':string(r.name),'targets':[string(b.target_name) for b in r.controlled_blocks],
                'events':[{'time':finite(k.time),'text':string(k.value)}for k in r.text_keys.text_keys] if r.text_keys else []}
                for r in data.roots if isinstance(r,NifFormat.NiControllerSequence)]
            records.append(row)
        except Exception as e:errors.append({**row,'error':str(e)})
        if (index+1)%100==0:print(f'Parsed {index+1}/{len(files)}; errors {len(errors)}',flush=True)
    # Original authored declarations; do not imply every recovered template
    # inheritance/list merge has already been proven equivalent to native code.
    declarations=[];tgi_sources={}
    def walk(node,ancestors):
        if any(k in node['attrs'] for k in ('kf','resource_prefix','model')):
            values={k:v for k,v in node['attrs'].items() if k in ('kf','resource_prefix','model','aspect_prefix','ids','type','length','key_ratio','walk_distance','disable_animations')}
            declarations.append({'source':node['source'],'line':node['line'],'tag':node['tag'],'qualifier':node.get('qualifier'),'header':node.get('header',{}),'values':values,
                'ancestors':[{'tag':a['tag'],'header':a.get('header',{}),'line':a['line'],'ids':a['attrs'].get('ids'),'resource_prefix':a['attrs'].get('resource_prefix')}for a in ancestors]})
        for child in node['children']:walk(child,ancestors+[node])
    for p in sorted(ORIGINAL.rglob('*.tgi')):
        old=len(declarations)
        try:
            for node in tgi.parse(p):walk(node,[])
        except Exception as e:errors.append({'source':p.relative_to(ORIGINAL).as_posix(),'error':'TGI '+str(e)})
        if len(declarations)>old:tgi_sources[p.relative_to(ORIGINAL).as_posix()]=sha(p)
    report={'format':'aa-animation-source-inventory-v1','generator_sha256':sha(Path(__file__)),'source_file_count':len(files),
        'kf_count':sum(p.suffix.lower()=='.kf' for p in files),'nif_count':sum(p.suffix.lower()=='.nif' for p in files),
        'parsed_count':len(records),'errors':errors,'assets':records,'tgi_declarations':declarations,'tgi_sources':tgi_sources,
        'limits':['Source controller inventory is not playback support; unsupported controllers/curves are retained explicitly.','TGI declarations are source-linked, not a claim of complete native template/list resolution.','No original model, texture, game rule, profile or release is changed.']}
    dest=OUT/'source-inventory.json';dest.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'parsed':len(records),'errors':len(errors),'kf':report['kf_count'],'nif':report['nif_count'],'sha256':sha(dest)}))
if __name__=='__main__':main()
