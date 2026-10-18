//! Proven class virtualB4 and the logical-index form of native4EA200.
//! Native pointer transport is replaced by the growable map's tile index.
pub(crate) fn is_actor(class:u32,kind:u8,object_present:bool,actor_present:bool)->Result<bool,String> {
    let expected=match class {
        2|6|7|9=>false,
        3|4=>true,
        _=>return Err("zone B4 actual constructor class is unsupported".into()),
    };
    if !object_present || actor_present!=expected || kind!=if expected {2}else{1} {
        return Err("zone B4 actual class/shell constructor storage differs".into());
    }
    Ok(expected)
}
pub(crate) fn cached_tile(width:i32,xy:[u32;2])->Result<i64,String> {
    if width<=0 {return Err("zone cached-tile actual map width is invalid".into())}
    Ok(i64::from(width)*i64::from(xy[1] as i32)+i64::from(xy[0] as i32))
}
