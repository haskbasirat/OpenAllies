"""Read-only NIF/PNG row-order audit; writes JSON evidence only, no images."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
RECON=ROOT/'reconstruction'
sys.path.insert(0,str(RECON/'tools'))
from convert_models import NifFormat,Converter,ORIGINAL,string
from convert_models_animations import read_nif
from PIL import Image
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=['Buildings/GBR/GBRCorps/GBRCorps.nif','Buildings/GBR/GBRCorps/GBRCorpsPack.nif','Buildings/GBR/GBRDepotOil/GBRDepotOil.nif']
converter=Converter();reports=[];textures={}
for path in paths:
    nif=ORIGINAL/path;data=read_nif(nif);raw=[];seen=set()
    def walk(node,properties=None):
        if node is None or not isinstance(node,NifFormat.NiAVObject) or id(node) in seen:return
        seen.add(id(node));props=dict(properties or {})
        for p in node.properties:
            if p:props[type(p).__name__]=p
        name=string(node.name);extras=[string(x.name) for x in getattr(node,'extra_data_list',[]) if x]
        if isinstance(node,NifFormat.NiTriBasedGeom) and node.data:
            if bool(node.flags&1) or 'Sg' in extras or 'Sgv' in extras or ' sg' in name.lower() or name.lower().startswith(('selectionbox','selection box','collision')):return
            _,uv_set=converter.material(nif,props.values());d=node.data
            if len(d.vertices) and list(d.get_triangles()):raw.append((name,[[float(uv.u),float(uv.v)] for uv in d.uv_sets[min(uv_set,len(d.uv_sets)-1)]] if d.uv_sets else []))
        children=list(getattr(node,'children',[]))
        if isinstance(node,NifFormat.NiLODNode):children=children[:1]
        for child in children:walk(child,props)
    for root in data.roots:walk(root)
    converted_path=RECON/'assets/converted/models'/Path(path).with_suffix('.json');converted=json.loads(converted_path.read_text())
    assert len(raw)==len(converted['meshes'])
    rows=[]
    for index,((name,uv),mesh) in enumerate(zip(raw,converted['meshes'])):
        assert name==mesh['name'];assert len(uv)*2==len(mesh['uvs'])
        if not uv:continue
        maximum=max(max(abs(source[0]-mesh['uvs'][2*i]),abs((1-source[1])-mesh['uvs'][2*i+1])) for i,source in enumerate(uv))
        assert maximum<=0.00000051
        texture=mesh['material'].get('texture');stats={}
        if texture:
            png=RECON/texture.lstrip('/');image=Image.open(png).convert('RGBA')
            if texture not in textures:
                source=(ORIGINAL/Path(texture.removeprefix('/assets/converted/textures/'))).with_suffix('.dds')
                original=Image.open(source).convert('RGBA')
                textures[texture]={'png_sha256':sha(png),'source':source.relative_to(ORIGINAL).as_posix(),'source_sha256':sha(source),'size':image.size,
                    'rgba_rows_identical_to_original_dds_decode':image.tobytes()==original.tobytes(),'alpha_extrema':image.getchannel('A').getextrema()}
                assert textures[texture]['rgba_rows_identical_to_original_dds_decode']
            def pixel(u,v):return image.getpixel((min(int((u%1)*image.width),image.width-1),min(int((v%1)*image.height),image.height-1)))
            black_native=black_converted=changed=triangles=0;examples=[]
            for j in range(0,len(mesh['indices']),3):
                ids=mesh['indices'][j:j+3];u=sum(uv[i][0] for i in ids)/3;v=sum(uv[i][1] for i in ids)/3
                native=pixel(u,v);flipped=pixel(u,1-v)
                black_native+=max(native[:3])<32;black_converted+=max(flipped[:3])<32;changed+=native!=flipped;triangles+=1
                if len(examples)<2 and native!=flipped:examples.append({'uv_native':[u,v],'uv_converted':[u,1-v],'original_row_rgba':native,'incorrect_flipped_row_rgba':flipped})
            stats={'triangle_centroids':triangles,'different_sample_rgba':changed,'dark_original_rows':black_native,'dark_flipped_rows':black_converted,'examples':examples}
        rows.append({'mesh_index':index,'name':name,'uv_count':len(uv),'first_native_uv':uv[0],'first_converted_uv':mesh['uvs'][:2],
            'maximum_error_against_u_one_minus_v':maximum,'texture':texture,'sample_statistics':stats})
    reports.append({'nif':path,'source_sha256':sha(nif),'converted_sha256':sha(converted_path),'mesh_count':len(raw),'textured_mesh_records':rows})
result={'passed':True,'scope':'No image or mesh mutation; source coordinates and unchanged decoded image rows compared.',
    'models':reports,'textures':textures,'conclusion':'Converted mesh V=1-originalNIFV. PNG rows equal originalDDS decode. OldThreeJS path flips texture rows onupload; WGPU uploads PNG rows directly, so WGPUmeshUV must recover originalV=1-convertedV.',
    'scope_boundary':'Apply inverseV only toconvertedmodel/animationUVs, notblankettoallterrain/generatedUVs orimagefiles. Materials/helpervisibility mayhave separateissues.',
    'sources':['reconstruction/tools/convert_models.py: mesh uvvalues 1-uv.v','reconstruction/tools/archive_textures.py: image.convertRGBA.save withouttranspose',
    'reconstruction/game/src/renderer.mjs: THREE.TextureLoader defaultflipY retained','reconstruction/game/node_modules/three/src/textures/Texture.js: flipY=true',
    'reconstruction/game/node_modules/three/src/renderers/webgl/WebGLTextures.js: UNPACK_FLIP_Y_WEBGL',
    'https://learn.microsoft.com/en-us/windows/win32/direct3d9/texture-coordinates','https://www.w3.org/TR/webgpu/#coordinate-systems'],
    'limitations':['Centroid nearest-texel statistics are diagnostics, not a full renderedprojection comparison.','UVorientation explains atlasmisplacement; notclaimit fixesall material/helper issues.']}
target=Path(__file__).with_name('uv-orientation-evidence.json');target.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'passed':True,'models':len(reports),'textures':len(textures),'sha256':sha(target),'summary':[(r['nif'],r['mesh_count'],len(r['textured_mesh_records']))for r in reports]},indent=2))
