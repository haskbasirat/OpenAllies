//!85A550(directory),85ABB0(join), then5A8A40's pair(full path,raw name).
type Result<T>=std::result::Result<T,String>;
pub(crate) fn pair(model:&[u16],texture:&[u16])->Result<(Vec<u16>,Vec<u16>)>{
 if model.contains(&0)||texture.contains(&0){return Err("native model texture path embeddedNUL is outside admitted counted source domain".into())}
 let mut directory=if let Some(i)=model.iter().rposition(|c|matches!(*c,47|92)){model[..i].to_vec()}
  else if let Some(i)=model.iter().rposition(|c|*c==58){model[..=i].to_vec()}else{vec![]};
 if texture.first().is_some_and(|c|matches!(*c,47|92)){
  if directory.last().is_some_and(|c|matches!(*c,47|92)){directory.pop();}
 }else if directory.last().is_some_and(|c|!matches!(*c,47|92)){directory.push(47);}
 directory.extend_from_slice(texture);Ok((directory,texture.to_vec()))
}
///5A9CA5..5A9CCB and5A83F0. Physical host allocation remains growable;
///this is only the actual current native logical+88 capacity projection.
pub(crate) fn capacity(current:usize,count:usize)->Result<usize>{
 if count<=current{return Ok(current)}
 let doubled=current.checked_mul(2).ok_or("model texture logical capacity overflow")?;
 Ok(if count>doubled{count}else{doubled}.max(4))
}
