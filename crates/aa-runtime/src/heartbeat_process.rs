//! ONE process-globalB40934 counter used by75B820. It is not initialized by
//! each Node, Session, mission or frame. The fresh application retains it
//! through load/activation; the borrowed heartbeat owns the ordered writes.
#[derive(Debug)]
pub struct Counter {word:i32}
impl Counter {
    pub(crate) fn from_fresh_process()->Self{Self{word:0}}
    pub fn word(&self)->i32{self.word}
    pub fn increment_wrapping(&mut self){self.word=self.word.wrapping_add(1);}
    pub fn reset_after_status_decree(&mut self){self.word=0;}
}
