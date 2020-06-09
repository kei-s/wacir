use super::super::code::Instructions;
use super::super::object::CompiledFunction;

pub struct Frame {
    func: CompiledFunction,
    pub ip: usize,
}

impl Frame {
    pub fn instructions(&self) -> &Instructions {
        &self.func.instructions
    }
}

pub fn new_frame(func: CompiledFunction) -> Frame {
    Frame { func, ip: 0 }
}
