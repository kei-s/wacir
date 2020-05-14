use super::code::*;
use super::compiler::*;
use super::object::*;
use std::convert::TryInto;
use std::rc::Rc;

const STACK_SIZE: usize = 2048;

pub struct VM {
    constants: Vec<Object>,
    instructions: Instructions,
    stack: Vec<Rc<Object>>,
    sp: usize,
}

impl VM {
    pub fn new(bytecode: ByteCode) -> VM {
        VM {
            constants: bytecode.constants,
            instructions: bytecode.instructions,
            stack: Vec::with_capacity(STACK_SIZE),
            sp: 0,
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut ip = 0;
        while ip < self.instructions.0.len() {
            let op = Opcode::from(self.instructions.0[ip]);
            match op {
                Opcode::OpConstant => {
                    let const_index = read_uint16(&self.instructions, ip + 1);
                    ip += 2;
                    // TODO: clone() どうにかなるか
                    let constant = self.constants[const_index as usize].clone();
                    self.push(constant)?;
                }
                Opcode::OpAdd => {
                    let right = self.pop();
                    let left = self.pop();
                    match (&*right, &*left) {
                        (Object::Integer(right_value), Object::Integer(left_value)) => {
                            self.push(Object::Integer(right_value + left_value))?;
                        }
                        _ => {
                            return Err(format!(
                                "unsupported object: right: {}, left: {}",
                                right, left
                            ))
                        }
                    }
                }
            }
            ip += 1;
        }
        Ok(())
    }

    fn push(&mut self, o: Object) -> Result<(), String> {
        if self.sp >= STACK_SIZE {
            return Err("stack overflow".to_string());
        }

        self.stack.push(Rc::clone(&Rc::new(o)));
        self.sp += 1;

        Ok(())
    }

    fn pop(&mut self) -> Rc<Object> {
        let o = self.stack.pop();
        self.sp -= 1;
        o.unwrap()
    }

    pub fn stack_top(self) -> Option<Rc<Object>> {
        if self.sp > 0 {
            self.stack
                .get::<usize>((self.sp - 1).try_into().unwrap())
                .map(|o| Rc::clone(o))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ast::Program;
    use super::super::compiler::Compiler;
    use super::super::lexer::Lexer;
    use super::super::object::Object;
    use super::super::parser::Parser;
    use super::*;

    #[test]
    fn test_integer_arithmetic() {
        let tests = vec![("1", 1), ("2", 2), ("1 + 2", 3)];

        run_vm_tests(tests);
    }

    fn run_vm_tests(tests: Vec<(&str, isize)>) {
        for (input, expected) in tests {
            let program = parse(input.to_string());
            let mut comp = Compiler::new();
            if let Err(err) = comp.compile(program) {
                assert!(false, "compile error: {}", err);
            }

            let mut vm = VM::new(comp.bytecode());
            if let Err(err) = vm.run() {
                assert!(false, "vm error: {}", err);
            }

            if let Some(stack_elem) = vm.stack_top() {
                test_expected_object(&expected, &stack_elem);
            } else {
                assert!(false, "stack_elem is None.");
            }
        }
    }

    fn parse(input: String) -> Program {
        let l = Lexer::new(&input);
        let mut p = Parser::new(l);
        p.parse_program()
    }

    fn test_expected_object(expected: &isize, actual: &Object) {
        test_integer_object(&(*expected as i64), actual);
    }

    fn test_integer_object(expected: &i64, actual: &Object) {
        if let Object::Integer(integer) = actual {
            assert_eq!(expected, integer);
        } else {
            assert!(false, "object is not Integer. {}", actual)
        }
    }
}
