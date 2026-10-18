"""Stage source-associated original animation families without modifying Phase2 cache.

Dynamic non-linear curves/controllers fail explicitly. A quadratic scalar/vector
channel with identical values and zero tangents is provably constant; its runtime
channel is reduced to a constant linear channel while retaining source metadata.
"""
from __future__ import annotations
import argparse,collections,hashlib,json,math,os,sys
from pathlib import Path
import export_original as base
from convert_models import NifFormat,Converter,ORIGINAL,string
from convert_models_animations import read_nif,euler_matrix
WORKSPACE=base.WORKSPACE;OUT=WORKSPACE/'assets-cache/animation';RESEARCH=WORKSPACE/'research/phase3-animation'
STAGE=OUT/'phase3-staging';STAGE.mkdir(parents=True,exist_ok=True)
sha=base.sha;vec=base.vec;matrix=base.matrix

def zero(v):return all(x==0 for x in v) if isinstance(v,list) else v==0
def channel(group,kind):
    source=list(group);keys=[{'time':float(k.time),'value':base.value(k.value)}for k in source]
    if keys and kind!=1:
        if kind!=2 or any(k['value']!=keys[0]['value'] for k in keys) or any(not zero(base.value(getattr(k,a,0))) for k in source for a in ['forward','backward']):
            raise ValueError(f'Unsupported varying channel type {kind}')
        return {'interpolation':1,'keys':keys,'source_interpolation':kind,'constant_reduction':True,
            'source_tangents':[{'forward':base.value(getattr(k,'forward',0)),'backward':base.value(getattr(k,'backward',0))}for k in source]}
    return {'interpolation':int(kind),'keys':keys}
base.channel=channel

def tags(node):
    markers=set()
    for extra in getattr(node,'extra_data_list',[]):
        if not extra:continue
        markers.add(string(extra.name).strip().lower())
        if string(extra.name).lower()=='userpropbuffer':markers.add(string(getattr(extra,'string_data',b'')).strip().lower())
    return markers

def collect_model(path,converter):
    data=read_nif(path);nodes=[];meshes=[];objects=[];mapping={};seen=set();excluded=[];legacy_count=0
    source=path.relative_to(ORIGINAL).as_posix();audit=converter.audited_materials[source]
    static_path=base.RECON/'assets/converted/models'/Path(source).with_suffix('.json')
    if sha(path)!=audit['source_sha256'] or sha(static_path)!=audit['converted_sha256']:raise ValueError('Audited model material source changed')
    def walk(node,parent=None,inherited=None,team=False,shadow=False):
        nonlocal legacy_count
        if node is None or not isinstance(node,NifFormat.NiAVObject) or id(node) in seen:return
        seen.add(id(node));name=string(node.name);index=len(nodes);mapping[id(node)]=index;objects.append(node)
        nodes.append({'name':name,'parent':parent,'translation':vec(node.translation),'rotation':matrix(node.rotation),'scale':float(node.scale)})
        properties=dict(inherited or {})
        for p in node.properties:
            if p:properties[type(p).__name__]=p
        markers=tags(node);team=team or 'repcolor' in markers;shadow=shadow or bool(markers&{'sg','sg2','sgv'})
        hidden=bool(node.flags&1) or shadow or name.lower().startswith(('selectionbox','selection box','collision'))
        if isinstance(node,NifFormat.NiTriBasedGeom) and node.data:
            extra_names=[string(x.name)for x in getattr(node,'extra_data_list',[]) if x]
            legacy_hidden=bool(node.flags&1) or 'Sg' in extra_names or 'Sgv' in extra_names or ' sg' in name.lower() or name.lower().startswith(('selectionbox','selection box','collision'))
            indices=[int(i)for tri in node.data.get_triangles()for i in tri]
            source_mesh_index=None
            if not legacy_hidden and len(node.data.vertices) and indices:source_mesh_index=legacy_count;legacy_count+=1
            # Reuse the independently native-audited per-mesh decisions. The
            # engine resets some inherited contexts, so propagating Sg blindly
            # can incorrectly remove valid geometry (RUSTankDestroyer fixture).
            correction=audit['corrections'][source_mesh_index] if source_mesh_index is not None else None
            hidden=correction is None or correction['skip']
            if hidden:excluded.append({'node':index,'name':name,'reason':'authored hidden/shadow/helper geometry'})
            else:
                material,uv_set=converter.material(path,properties.values());material['teamColorMask']=correction['team_color_mask'];d=node.data
                uvs=[[float(u.u),float(1-u.v)] for u in d.uv_sets[min(uv_set,len(d.uv_sets)-1)]] if d.uv_sets else []
                uv_placeholder=not uvs
                if uv_placeholder:
                    if material.get('texture'):raise ValueError('Textured mesh has no authored UV set')
                    # Match RawMesh::vertices for genuinely untextured geometry;
                    # this does not invent an atlas mapping or texture channel.
                    uvs=[[0.0,0.0] for _ in d.vertices]
                if len(d.vertices) and indices:meshes.append({'name':name,'node':index,'source_mesh_index':source_mesh_index,'positions':[vec(v) for v in d.vertices],
                    'normals':[vec(n) for n in d.normals] if d.has_normals else [],'uvs':uvs,'untextured_uv_placeholder':uv_placeholder,'indices':indices,'material':material,'skin':None,'_object':node})
        children=list(getattr(node,'children',[]))
        if isinstance(node,NifFormat.NiLODNode):children=children[:1]
        for child in children:walk(child,index,properties,team,shadow)
    for root in data.roots:walk(root)
    for mesh in meshes:
        obj=mesh['_object']
        if obj.is_skin():
            skin=obj.skin_instance;sd=skin.data
            mesh['skin']={'skeleton_root':mapping[id(skin.skeleton_root)],'skin_offset':matrix(sd.get_transform()),
                'bones':[{'node':mapping[id(b)],'inverse_bind':matrix(bd.get_transform()),'weights':[{'vertex':int(w.index),'weight':float(w.weight)}for w in bd.vertex_weights]}
                    for b,bd in zip(skin.bones,sd.bone_list)]}
    grouped=collections.defaultdict(list)
    for i,n in enumerate(nodes):grouped[n['name']].append(i)
    names={name:indices[0]for name,indices in grouped.items() if len(indices)==1}
    static=json.loads(static_path.read_text())
    if legacy_count!=len(static['meshes']):raise ValueError('Original static mesh traversal count differs')
    for mesh in meshes:
        original=static['meshes'][mesh['source_mesh_index']]
        if original['name']!=mesh['name'] or original['indices']!=mesh['indices'] or len(original['positions'])!=3*len(mesh['positions']):raise ValueError('Original static mesh identity/topology differs')
    if [m['source_mesh_index']for m in meshes]!=[r['mesh_index']for r in audit['corrections']if not r['skip']]:raise ValueError('Audited retained-mesh mapping differs')
    return data,nodes,meshes,objects,names,excluded

def candidate_bindings():
    """Resolved-view projection points to candidate pairs; direct source TGI
    kf/resource_prefix declarations are retained in the separate inventory.
    Runtime selection/transition behavior is not inferred from these pairs.
    """
    catalog=json.loads((base.RECON/'game/data/catalog.json').read_text())
    originals={p.relative_to(ORIGINAL).as_posix().casefold():p for p in ORIGINAL.rglob('*') if p.suffix.lower() in ('.nif','.kf')}
    grouped=collections.defaultdict(lambda:{'clips':{},'associations':[]})
    def resolve(value,source):
        if not isinstance(value,str) or '%' in value:return None
        rel=value.lstrip('/') if value.startswith('/') else (Path(source).parent/value).as_posix()
        return originals.get(rel.casefold())
    for category in ['units','buildings','things']:
        for identity,thing in catalog[category].items():
            view=thing.get('data',{}).get('View',{});source=thing['source']
            if not isinstance(view,dict):continue
            prefix=view.get('resource_prefix')
            if not isinstance(prefix,str):continue
            aspects=[]
            for key,value in view.items():
                if key.casefold().startswith('aspect') and isinstance(value,(list,dict)):
                    aspects.extend(value if isinstance(value,list) else [value])
            for aspect in aspects:
                model=aspect.get('model');aspect_prefix=aspect.get('aspect_prefix') or ''
                if not isinstance(model,str):continue
                model=resolve(model.replace('%s',prefix+aspect_prefix),source)
                if model is None:continue
                animations=aspect.get('Animation',[]);animations=animations if isinstance(animations,list) else [animations]
                for animation in animations:
                    kf=animation.get('kf');ids=animation.get('ids')
                    if not isinstance(kf,str):continue
                    if '%s' in kf:
                        if not isinstance(ids,str):continue
                        try:kf=kf%(prefix,ids)
                        except TypeError:continue
                    clip=resolve(kf,source)
                    if clip:
                        key=model.relative_to(ORIGINAL).as_posix();grouped[key]['clips'][clip.relative_to(ORIGINAL).as_posix()]=clip
                        grouped[key]['associations'].append({'actor':identity,'category':category,'source_tgi':source,'source_sha256':sha(ORIGINAL/source),'aspect':aspect.get('ids'),
                            'animation_ids':ids,'declared_length':animation.get('length'),'model':key,'kf':clip.relative_to(ORIGINAL).as_posix(),'basis':'recovered original TGI view projection; native runtime selection not asserted'})
    return grouped

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--match',default='Tank|Battleship|Destroyer|Carrier|Artillery|CarArmored');args=parser.parse_args()
    import re
    converter=Converter();material_path=WORKSPACE/'assets-cache/model-materials.json';material_document=json.loads(material_path.read_text())
    if material_document['schema']!=1 or not material_document['complete']:raise ValueError('Native material audit is incomplete')
    converter.audited_materials=material_document['models'];bindings=candidate_bindings();exports=[];skipped=[]
    selected=[model for model in sorted(bindings) if re.search(args.match,model,re.I)]
    target_names=['Vehicle_'+Path(model).stem.casefold() for model in selected]
    if len(set(target_names))!=len(target_names):raise ValueError('Output model stem collision; explicit names required')
    for model,entry in sorted(bindings.items()):
        if not re.search(args.match,model,re.I):continue
        try:
            path=ORIGINAL/model;data,nodes,meshes,objects,names,excluded=collect_model(path,converter)
            if not meshes:raise ValueError('No visible meshes')
            clips=[]
            for source,clip in sorted(entry['clips'].items()):clips.append(base.export_clip(clip,nodes,objects,names))
            if not clips:raise ValueError('No bound clips')
            refs=[{'clip':None,'time':0,'meshes':base.reference_pose(nodes,meshes,objects,None,0)}]
            for i,clip in enumerate(clips):
                for fraction in [0,.25,.5,.75,1]:refs.append({'clip':i,'time':fraction*clip['duration'],'meshes':base.reference_pose(nodes,meshes,objects,clip,fraction*clip['duration'])})
            name='Vehicle_'+Path(model).stem;target=STAGE/f'{name}.rig.json';reference=STAGE/f'{name}.reference.json'
            result={'format':'aa-original-rig-v1','source':model,'source_sha256':sha(path),'static_model_sha256':sha(base.RECON/'assets/converted/models'/Path(model).with_suffix('.json')),'coordinates':'native Z-up local data; output(x,z,-y); browser-oriented UVs','nif_version':'.'.join(str((data.version>>n)&255)for n in (24,16,8,0)),
                'nodes':nodes,'meshes':[{k:v for k,v in m.items() if k!='_object'}for m in meshes],'clips':clips,'excluded_geometry':excluded,
                'limitations':['Isolated authored clips; no runtime selection, turret AI, movement-driven wheel motion or gameplay events.',
                    'Only linear curves and provably constant zero-tangent quadratic channels supported.','Python/PyFFI differential reference, not executed native Gamebryo interpolation.']}
            target.write_text(json.dumps(result,separators=(',',':'))+'\n');reference.write_text(json.dumps({'source_sha256':sha(path),'poses':refs},separators=(',',':'))+'\n')
            exports.append({'name':name,'source':model,'source_sha256':sha(path),'file':target.relative_to(OUT).as_posix(),'sha256':sha(target),'reference':reference.relative_to(OUT).as_posix(),'reference_sha256':sha(reference),
                'clips':len(clips),'nodes':len(nodes),'meshes':len(meshes),'vertices':sum(len(m['positions'])for m in meshes),'associations':entry['associations']})
            print(f'Exported {name}: {len(clips)} clips',flush=True)
        except Exception as e:
            skipped.append({'model':model,'kf_files':sorted(entry['clips']),'reason':str(e),'associations':entry['associations']})
            print(f'Skipped {model}: {e}',flush=True)
    report={'format':'aa-animation-family-manifest-v1','generator_sha256':sha(Path(__file__)),'phase2_exporter_sha256':sha(Path(base.__file__)),
        'recovered_catalog_sha256':sha(base.RECON/'game/data/catalog.json'),'material_audit_sha256':sha(material_path),'exports':exports,'skipped':skipped,
        'association_count':sum(len(v['associations'])for v in bindings.values()),'models_with_view_kf_bindings':len(bindings),
        'scope':'Only selected complete View-bound model families; phase2 five infantry cache and original files remain untouched.'}
    RESEARCH.mkdir(parents=True,exist_ok=True);(RESEARCH/'family-coverage.json').write_text(json.dumps(report,indent=2)+'\n');(OUT/'families-manifest.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'exported':len(exports),'clips':sum(r['clips']for r in exports),'skipped':len(skipped),'sha256':sha(RESEARCH/'family-coverage.json')}))
if __name__=='__main__':main()
