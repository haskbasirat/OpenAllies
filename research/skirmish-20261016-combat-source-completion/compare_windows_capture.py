"""Compare sealed actual Windows reference with full native-source emulation.

CV-relevant flags are compared separately from complete component masks. The
historical Detection-only differences were corrected in the v3 input receipt;
the current complete mask comparison must now also agree with capture03.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sha=lambda raw:hashlib.sha256(raw).hexdigest()
def main():
 capture_path=ROOT/'research/development10/native-source-capture-03/capture.json';emulation_path=HERE/'native-source-completion-v3.json';input_path=HERE/'full-source-inputs-v3.json'
 capture=json.loads(capture_path.read_text());emulation=json.loads(emulation_path.read_text());inputs=json.loads(input_path.read_text())
 actual=next(row for row in capture['captures'] if row['label']=='source_return');precision={0:'Single24',0x200:'Double53',0x300:'Extended64'}[actual['x87_control']&0x300]
 expected=next(row for row in emulation['cases'] if row['image']=='v14' and row['precision']==precision)
 assert [r['ids'] for r in actual['actors']]==[r['id'] for r in expected['values']]
 bound={r['id']:r['mask']['Ok'] for r in inputs['rows'] if r['precision']==precision};differences=[];flags=[];relevant=[]
 for observed,wanted in zip(actual['actors'],expected['values']):
  for raw,field in [('1e4','value_1e4'),('30c','children_30c'),('36c','weighted_36c')]:
   if observed['fields'][raw]!=wanted[field]:differences.append({'id':observed['ids'],'field':raw,'actual':observed['fields'][raw],'emulated':wanted[field]})
  if observed['flags_174']!=bound[observed['ids']]:
   row={'id':observed['ids'],'actual':observed['flags_174'],'bound':bound[observed['ids']],'xor':observed['flags_174']^bound[observed['ids']]};flags.append(row)
   if row['xor']&0x245e9:relevant.append(row)
 means=[actual['manager_means'][f'{off:x}'] for off in range(0x74,0x94,4)]
 result={'schema':2,'scope':__doc__,'capture_sha256':sha(capture_path.read_bytes()),'emulation_sha256':sha(emulation_path.read_bytes()),'input_sha256':sha(input_path.read_bytes()),'actor_count':len(actual['actors']),'same_manager_order':True,'precision_from_actual_control_word':precision,'cache_word_comparisons':len(actual['actors'])*3,'cache_word_mismatches':differences,'means_match':means==expected['means'],'actual_means':means,'emulated_means':expected['means'],'complete_component_mask_differences':flags,'cv_relevant_flag_mask':hex(0x245e9),'cv_relevant_flag_differences':relevant,'observed_x87_control':actual['x87_control'],'observed_mxcsr':actual['mxcsr'],'fp_state_written_by_debugger':not capture['fp_state_never_written_by_debugger'],'cv_dependency_slice_passed':not differences and not relevant and means==expected['means'],'full_source_or_component_mask_completion_claimed':False,'producer_sha256':sha(Path(__file__).read_bytes())}
 (HERE/'windows-v14-comparison-v3.json').write_text(json.dumps(result,indent=2)+'\n');print('passed CV slice',result['cv_dependency_slice_passed'],'cache comparisons',result['cache_word_comparisons'],'complete-mask differences',len(flags))
if __name__=='__main__':main()
