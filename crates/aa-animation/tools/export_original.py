"""Read original NIF/KF only; write a new, source-pinned animation cache.

The earlier flattened meshes and approximate 15fps morph exports are not inputs
to playback. Original vertex order, rig links, weights, matrices and float32 key
values are retained. Only the verified linear tracks and constant Euler channels
in the selected infantry clips are accepted; unsupported curves fail explicitly.
"""
from __future__ import annotations
import argparse, bisect, hashlib, json, math, os, sys
from pathlib import Path

WORKSPACE=Path(__file__).resolve().parents[3]
RECON=WORKSPACE.parent/'reconstruction'
os.environ['NIFXMLPATH']=str(RECON/'tools/convert_models_vendor/pyffi/formats/nif/nifxml')
sys.path.insert(0,str(RECON/'tools'))
from convert_models import Converter,NifFormat,ORIGINAL,string
from convert_models_animations import read_nif,quaternion_matrix,euler_matrix

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def vec(v):return [float(v.x),float(v.y),float(v.z)]
def matrix(m):return [[float(x) for x in row] for row in m.as_list()]
def value(v):
    if isinstance(v,(float,int)):return float(v)
    if hasattr(v,'w'):return [float(v.x),float(v.y),float(v.z),float(v.w)]
    return vec(v)
def channel(group,kind):
    keys=[{'time':float(k.time),'value':value(k.value)} for k in group]
    if keys and kind!=1:raise ValueError(f'Unsupported non-linear track: {kind}')
    return {'interpolation':int(kind),'keys':keys}

def collect_model(path):
    data=read_nif(path); converter=Converter();nodes=[];meshes=[];objects=[];mapping={}; seen=set()
    def walk(node,parent=None,inherited=None,team=False):
        if node is None or not isinstance(node,NifFormat.NiAVObject) or id(node) in seen:return
        seen.add(id(node));name=string(node.name);index=len(nodes);mapping[id(node)]=index;objects.append(node)
        nodes.append({'name':name,'parent':parent,'translation':vec(node.translation),'rotation':matrix(node.rotation),'scale':float(node.scale)})
        properties=dict(inherited or {})
        for p in node.properties:
            if p:properties[type(p).__name__]=p
        extras=[string(x.name) for x in getattr(node,'extra_data_list',[]) if x]
        team=team or 'RepColor' in extras
        hidden=bool(node.flags&1) or 'Sg' in extras or 'Sgv' in extras or ' sg' in name.lower() or name.lower().startswith(('selectionbox','selection box','collision'))
        if isinstance(node,NifFormat.NiTriBasedGeom) and node.data and not hidden:
            material,uv_set=converter.material(path,properties.values());material['teamColorMask']=team
            d=node.data
            uvs=[[float(u.u),float(1-u.v)] for u in d.uv_sets[min(uv_set,len(d.uv_sets)-1)]] if d.uv_sets else []
            indices=[int(i) for tri in d.get_triangles() for i in tri]
            if len(d.vertices) and indices:
                meshes.append({'name':name,'node':index,'positions':[vec(v) for v in d.vertices],
                    'normals':[vec(n) for n in d.normals] if d.has_normals else [],'uvs':uvs,'indices':indices,'material':material,'skin':None,'_object':node})
        children=list(getattr(node,'children',[]))
        if isinstance(node,NifFormat.NiLODNode):children=children[:1]
        for child in children:walk(child,index,properties,team)
    for root in data.roots:walk(root)
    for m in meshes:
        obj=m['_object']
        if obj.is_skin():
            skin=obj.skin_instance;sd=skin.data
            m['skin']={'skeleton_root':mapping[id(skin.skeleton_root)],'skin_offset':matrix(sd.get_transform()),
                'bones':[{'node':mapping[id(b)],'inverse_bind':matrix(bd.get_transform()),
                    'weights':[{'vertex':int(w.index),'weight':float(w.weight)} for w in bd.vertex_weights]}
                    for b,bd in zip(skin.bones,sd.bone_list)]}
    names={}
    for i,n in enumerate(nodes):
        if n['name'] in names:raise ValueError('Ambiguous target node '+n['name'])
        names[n['name']]=i
    return data,nodes,meshes,objects,names

def export_clip(path,nodes,objects,names):
    data=read_nif(path);seqs=[r for r in data.roots if isinstance(r,NifFormat.NiControllerSequence)]
    if len(seqs)!=1:raise ValueError('Expected one original sequence')
    sequence=seqs[0];tracks=[];seen=set()
    for block in sequence.controlled_blocks:
        c=block.controller; target=string(block.target_name)
        if c is None:continue
        if not isinstance(c,NifFormat.NiKeyframeController):raise ValueError('Unsupported '+type(c).__name__)
        if target not in names:raise ValueError('Unresolved target '+target)
        index=names[target]
        if index in seen:raise ValueError('Duplicate controller target '+target)
        seen.add(index);d=c.data
        if not isinstance(d,NifFormat.NiKeyframeData):raise ValueError('Unsupported track data')
        t={'node':index,'target':target,'controller':type(c).__name__,'frequency':float(c.frequency),'phase':float(c.phase),
            'start':float(c.start_time),'stop':float(c.stop_time),'flags':int(c.flags),
            'translation':channel(d.translations.keys,int(d.translations.interpolation)),
            'scale':channel(d.scales.keys,int(d.scales.interpolation)),'rotation':None}
        if int(d.rotation_type)==4:
            angles=[]
            for axis in d.xyz_rotations:
                keys=list(axis.keys)
                if not keys:raise ValueError('Missing Euler axis')
                if any(float(k.value)!=float(keys[0].value) or float(getattr(k,'forward',0))!=0 or float(getattr(k,'backward',0))!=0 for k in keys):
                    raise ValueError('Non-constant Euler spline is unsupported')
                angles.append(float(keys[0].value))
            # For these original rifle tracks the constant Euler orientation
            # agrees with the stored bind matrix. Preserve the actual bind
            # matrix; never replace a nonconstant spline with a linear curve.
            computed=matrix(euler_matrix(angles));authored=nodes[index]['rotation']
            delta=max(abs(a-b) for rowa,rowb in zip(computed,authored) for a,b in zip(rowa,rowb))
            if delta>1e-6:raise ValueError('Constant Euler differs from bind orientation')
            t['rotation']={'kind':'constant_bind','euler_xyz':angles,'max_bind_matrix_delta':delta}
        else:t['rotation']={'kind':'quaternion','channel':channel(d.quaternion_keys,int(d.rotation_type))}
        if t['frequency']!=1 or t['phase']!=0:raise ValueError('Non-default controller timing needs native audit')
        tracks.append(t)
    events=[{'time':float(k.time),'text':string(k.value)} for k in sequence.text_keys.text_keys] if sequence.text_keys else []
    duration=max([t['stop'] for t in tracks]+[e['time'] for e in events]+[0])
    if duration<=0:raise ValueError('No positive duration')
    return {'name':string(sequence.name),'source':path.relative_to(ORIGINAL).as_posix(),'source_sha256':sha(path),
        'duration':duration,'cycle_type':int(sequence.cycle_type) if data.version>=0x0A01006A else None,'tracks':tracks,'events':events}

def sample(keys,t,quat=False):
    if not keys:return None
    j=bisect.bisect_right([p['time'] for p in keys],t)
    if j==0:return keys[0]['value']
    if j==len(keys):return keys[-1]['value']
    a,b=keys[j-1],keys[j];u=(t-a['time'])/(b['time']-a['time']);x,y=a['value'],b['value']
    if isinstance(x,float):return x+(y-x)*u
    if not quat:return [v+(w-v)*u for v,w in zip(x,y)]
    dot=sum(v*w for v,w in zip(x,y))
    if dot<0:y=[-v for v in y];dot=-dot
    dot=min(1,max(-1,dot))
    if dot<.9995:
        theta=math.acos(dot);s=math.sin(theta)
        q=[(math.sin((1-u)*theta)*v+math.sin(u*theta)*w)/s for v,w in zip(x,y)]
    else:q=[v+(w-v)*u for v,w in zip(x,y)]
    length=math.sqrt(sum(v*v for v in q));return [v/length for v in q]

def reference_pose(nodes,meshes,objects,clip,t):
    for n,o in zip(nodes,objects):
        o.translation.x,o.translation.y,o.translation.z=n['translation'];o.scale=n['scale']
        for i,row in enumerate(n['rotation']):
            for j,v in enumerate(row):setattr(o.rotation,f'm_{i+1}{j+1}',v)
    if clip is not None:
        for track in clip['tracks']:
            o=objects[track['node']];timestamp=min(track['stop'],max(track['start'],t))
            p=sample(track['translation']['keys'],timestamp)
            if p is not None:o.translation.x,o.translation.y,o.translation.z=p
            s=sample(track['scale']['keys'],timestamp)
            if s is not None:o.scale=s
            if track['rotation']['kind']=='quaternion':
                q=sample(track['rotation']['channel']['keys'],timestamp,True)
                if q is not None:o.rotation=quaternion_matrix(q)
    globals=[]
    for n,o in zip(nodes,objects):
        m=o.get_transform()
        if n['parent'] is not None:m*=globals[n['parent']]
        globals.append(m)
    result=[]
    for mesh in meshes:
        o=mesh['_object'];vertices,normals=o.get_skin_deformation() if o.is_skin() else (o.data.vertices,o.data.normals)
        transform=globals[mesh['node']];_,rotation,_=transform.get_scale_rotation_translation()
        positions=[];normals_out=[]
        for v in vertices:
            p=v*transform;positions.append([float(p.x),float(p.z),float(-p.y)])
        for n in normals:
            p=n*rotation;length=math.sqrt(p.x*p.x+p.y*p.y+p.z*p.z)
            normals_out.append([float(p.x/length),float(p.z/length),float(-p.y/length)])
        result.append({'name':mesh['name'],'positions':positions,'normals':normals_out})
    return result

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--nations',default='USA,GER,GBR,JPN,RUS');args=parser.parse_args()
    if any(n not in {'USA','GER','GBR','JPN','RUS'} for n in args.nations.split(',')):raise ValueError('Only the five reviewed original infantry rigs are supported')
    out=WORKSPACE/'assets-cache/animation';out.mkdir(parents=True,exist_ok=True);records=[]
    for nation in args.nations.split(','):
        path=ORIGINAL/f'Units/{nation}/{nation}Infantry/{nation}Infantry.nif';original_sha=sha(path)
        data,nodes,meshes,objects,names=collect_model(path)
        clips=[export_clip(ORIGINAL/f'Shared/Animations/Units/Infantry/{clip}.kf',nodes,objects,names) for clip in ['Idle','Jog','Attack1','Die1']]
        refs=[{'clip':None,'time':0,'meshes':reference_pose(nodes,meshes,objects,None,0)}]
        for i,clip in enumerate(clips):
            for fraction in [0,.25,.5,.75,1]:refs.append({'clip':i,'time':fraction*clip['duration'],'meshes':reference_pose(nodes,meshes,objects,clip,fraction*clip['duration'])})
        cleanmeshes=[{k:v for k,v in m.items() if k!='_object'} for m in meshes]
        result={'format':'aa-original-rig-v1','source':path.relative_to(ORIGINAL).as_posix(),'source_sha256':original_sha,
            'coordinates':'native Z-up local data; sample output (x,z,-y); matrices are NIF row-major / Rust column-major transpose',
            'nif_version':'.'.join(str((data.version>>n)&255) for n in (24,16,8,0)),
            'nodes':nodes,'meshes':cleanmeshes,'clips':clips,
            'limitations':['Only linear translation/scale and shortest-arc quaternion SLERP supported; constant rifle Euler orientation retains verified bind matrix.',
            'No animation blending, root-motion extraction, visibility/material controllers, LOD changes or authoritative game-event execution.',
            'Preview interpolation/skinning is compared with an independent Python/PyFFI reference, not yet an executed native Gamebryo interpolator.']}
        destination=out/f'{nation}Infantry.rig.json';destination.write_text(json.dumps(result,separators=(',',':'))+'\n')
        reference=out/f'{nation}Infantry.reference.json';reference.write_text(json.dumps({'source_sha256':original_sha,'poses':refs},separators=(',',':'))+'\n')
        assert sha(path)==original_sha
        records.append({'source':result['source'],'source_sha256':original_sha,'file':destination.name,'sha256':sha(destination),'reference':reference.name,'reference_sha256':sha(reference),
            'nodes':len(nodes),'meshes':len(meshes),'vertices':sum(len(m['positions']) for m in meshes),'clips':len(clips),'reference_poses':len(refs)})
    manifest={'format':'aa-original-rig-cache-v1','generator_sha256':sha(Path(__file__)),'parser_sha256':sha(RECON/'tools/convert_models_vendor/pyffi/formats/nif/__init__.py'),
        'parser_schema_sha256':sha(RECON/'tools/convert_models_vendor/pyffi/formats/nif/nifxml/nif.xml'),
        'reference_helpers_sha256':sha(RECON/'tools/convert_models_animations.py'),
        'material_resolver_sha256':sha(RECON/'tools/convert_models.py'),'assets':records}
    (out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps(manifest,indent=2))
if __name__=='__main__':main()
