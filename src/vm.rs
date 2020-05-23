use super::code::*;
use super::compiler::*;
use super::object::*;
use std::convert::TryInto;

const STACK_SIZE: usize = 2048;
const TRUE: Object = Object::Boolean(true);
const FALSE: Object = Object::Boolean(false);

pub struct VM {
    constants: Vec<Object>,
    instructions: Instructions,
    stack: Vec<Object>,
    sp: usize,
    pub last_popped_stack_elem: Option<Object>,
}

impl VM {
    pub fn new(bytecode: ByteCode) -> VM {
        VM {
            constants: bytecode.constants,
            instructions: bytecode.instructions,
            stack: Vec::with_capacity(STACK_SIZE),
            sp: 0,
            last_popped_stack_elem: None,
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
                Opcode::OpAdd | Opcode::OpSub | Opcode::OpMul | Opcode::OpDiv => {
                    self.execute_binary_operation(op)?;
                }
                Opcode::OpPop => {
                    self.pop();
                }
                Opcode::OpTrue => {
                    self.push(TRUE)?;
                }
                Opcode::OpFalse => {
                    self.push(FALSE)?;
                }
                Opcode::OpEqual | Opcode::OpNotEqual | Opcode::OpGreaterThan => {
                    self.execute_comparison(op)?;
                }
                Opcode::OpBang => {
                    self.execute_bang_operator()?;
                }
                Opcode::OpMinus => {
                    self.execute_minus_operator()?;
                }
                Opcode::OpJump => {
                    let pos = u16::from_be_bytes(
                        self.instructions.0[ip + 1..ip + 1 + 2].try_into().unwrap(),
                    ) as usize;
                    ip = pos - 1;
                }
                Opcode::OpJumpNotTruthy => {
                    let pos = u16::from_be_bytes(
                        self.instructions.0[ip + 1..ip + 1 + 2].try_into().unwrap(),
                    ) as usize;
                    ip += 2;

                    let condition = self.pop();
                    if !Self::is_truthy(condition){
                        ip = pos - 1;
                    }
                }
                //
                // _ => todo!(),
            }
            ip += 1;
        }
        Ok(())
    }

    fn push(&mut self, o: Object) -> Result<(), String> {
        if self.sp >= STACK_SIZE {
            return Err("stack overflow".to_string());
        }

        self.stack.push(o);
        self.sp += 1;

        Ok(())
    }

    fn pop(&mut self) -> Object {
        let o = self.stack.pop();
        self.sp -= 1;
        let obj = o.unwrap();
        // TODO: obj.clone()
        self.last_popped_stack_elem = Some(obj.clone());
        obj
    }

    fn execute_binary_operation(&mut self, op: Opcode) -> Result<(), String> {
        let right = self.pop();
        let left = self.pop();
        match (&left, &right) {
            (Object::Integer(left_value), Object::Integer(right_value)) => {
                self.execute_binary_integer_operation(op, *left_value, *right_value)?;
            }
            _ => {
                return Err(format!(
                    "unsupported object: right: {}, left: {}",
                    &right, &left
                ))
            }
        }
        Ok(())
    }

    fn execute_binary_integer_operation(
        &mut self,
        op: Opcode,
        left_value: i64,
        right_value: i64,
    ) -> Result<(), String> {
        let result = match op {
            Opcode::OpAdd => left_value + right_value,
            Opcode::OpSub => left_value - right_value,
            Opcode::OpMul => left_value * right_value,
            Opcode::OpDiv => left_value / right_value,
            _ => return Err(format!("unknown integer oprerator: {:?}", op)),
        };
        self.push(Object::Integer(result))
    }

    fn execute_comparison(&mut self, op: Opcode) -> Result<(), String> {
        let right = self.pop();
        let left = self.pop();

        if let (Object::Integer(left_value), Object::Integer(right_value)) = (&right, &left) {
            return self.execute_integer_comparison(op, *left_value, *right_value);
        }

        match op {
            Opcode::OpEqual => self.push(Self::native_bool_to_boolean_object(right == left)),
            Opcode::OpNotEqual => self.push(Self::native_bool_to_boolean_object(right != left)),
            _ => Err(format!("unknown operator: {:?} {} {}", op, right, left)),
        }
    }

    fn execute_integer_comparison(
        &mut self,
        op: Opcode,
        left_value: i64,
        right_value: i64,
    ) -> Result<(), String> {
        match op {
            Opcode::OpEqual => self.push(Self::native_bool_to_boolean_object(
                right_value == left_value,
            )),
            Opcode::OpNotEqual => self.push(Self::native_bool_to_boolean_object(
                right_value != left_value,
            )),
            Opcode::OpGreaterThan => self.push(Self::native_bool_to_boolean_object(
                right_value > left_value,
            )),
            _ => Err(format!("unknown operator: {:?}", op)),
        }
    }

    fn execute_bang_operator(&mut self) -> Result<(), String> {
        let operand = self.pop();

        match operand {
            TRUE => self.push(FALSE),
            FALSE => self.push(TRUE),
            _ => self.push(FALSE),
        }
    }

    fn execute_minus_operator(&mut self) -> Result<(), String> {
        let operand = self.pop();
        if let Object::Integer(integer) = operand {
            self.push(Object::Integer(-integer))
        } else {
            Err(format!("unsupported type for negation: {}", operand))
        }
    }

    fn native_bool_to_boolean_object(input: bool) -> Object {
        if input {
            TRUE
        } else {
            FALSE
        }
    }

    fn is_truthy(obj: Object) -> bool {
        if let Object::Boolean(boolean) = obj {
            boolean
        } else {
            true
        }
    }

    pub fn stack_top(&self) -> Option<&Object> {
        if self.sp > 0 {
            self.stack.get::<usize>((self.sp - 1).try_into().unwrap())
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
        let tests = vec![
            ("1", 1),
            ("2", 2),
            ("1 + 2", 3),
            ("1 - 2", -1),
            ("1 * 2", 2),
            ("4 / 2", 2),
            ("50 / 2 * 2 + 10 - 5", 55),
            ("5 + 5 + 5 + 5 - 10", 10),
            ("2 * 2 * 2 * 2 * 2", 32),
            ("5 * 2 + 10", 20),
            ("5 + 2 * 10", 25),
            ("5 * (2 + 10)", 60),
            ("-5", -5),
            ("-10", -10),
            ("-50 + 100 + -50", 0),
            ("(5 + 10 * 2 + 15 / 3) * 2 + -10", 50),
        ];

        run_vm_tests(tests);
    }

    #[test]
    fn test_boolean_expressions() {
        let tests = vec![
            ("true", true),
            ("false", false),
            ("1 < 2", true),
            ("1 > 2", false),
            ("1 < 1", false),
            ("1 > 1", false),
            ("1 == 1", true),
            ("1 != 1", false),
            ("1 == 2", false),
            ("1 != 2", true),
            ("true == true", true),
            ("false == false", true),
            ("true == false", false),
            ("true != false", true),
            ("false != true", true),
            ("(1 < 2) == true", true),
            ("(1 < 2) == false", false),
            ("(1 > 2) == true", false),
            ("(1 > 2) == false", true),
            ("!true", false),
            ("!false", true),
            ("!5", false),
            ("!!true", true),
            ("!!false", false),
            ("!!5", true),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_conditionals() {
        let tests = vec![
            ("if (true) { 10 }", 10),
            ("if (true) { 10 } else { 20 }", 10),
            ("if (false) { 10 } else { 20 }", 20),
            ("if (1) { 10 }", 10),
            ("if (1 < 2) { 10 }", 10),
            ("if (1 < 2) { 10 } else { 20 }", 10),
            ("if (1 > 2) { 10 } else { 20 }", 20),
        ];
        run_vm_tests(tests);
    }

    fn run_vm_tests<T: Expectable>(tests: Vec<(&str, T)>) {
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

            if let Some(stack_elem) = vm.last_popped_stack_elem {
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

    fn test_expected_object<T: Expectable>(expected: &T, actual: &Object) {
        expected.assert_eq(actual);
    }

    trait Expectable {
        fn assert_eq(&self, actual: &Object);
    }

    impl Expectable for i64 {
        fn assert_eq(&self, actual: &Object) {
            if let Object::Integer(integer) = actual {
                assert_eq!(self, integer);
            } else {
                assert!(false, "object is not Integer. {}", actual)
            }
        }
    }

    impl Expectable for bool {
        fn assert_eq(&self, actual: &Object) {
            if let Object::Boolean(boolean) = actual {
                assert_eq!(self, boolean);
            } else {
                assert!(false, "object is not Boolean. {}", actual)
            }
        }
    }
}
