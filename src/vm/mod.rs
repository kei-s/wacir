mod frame;

use super::code::*;
use super::compiler::*;
use super::object;
use super::object::hash::hash_key_of;
use super::object::Object;
use frame::*;
use std::collections::HashMap;
use std::convert::TryInto;

const STACK_SIZE: usize = 2048;
pub const GLOBALS_SIZE: usize = 65536;
const MAX_FRAMES: usize = 1024;
const TRUE: Object = Object::Boolean(true);
const FALSE: Object = Object::Boolean(false);
const NULL: Object = Object::Null;

pub fn new_globals_store() -> Vec<Object> {
    Vec::with_capacity(GLOBALS_SIZE)
}

pub struct VM<'a> {
    constants: &'a mut Vec<Object>,
    stack: Vec<Object>,
    sp: usize,
    pub last_popped_stack_elem: Option<Object>,
    globals: &'a mut Vec<Object>,
    frames: Vec<Frame>,
    frame_index: usize,
}

impl<'a> VM<'a> {
    pub fn new_with_globals_store(bytecode: ByteCode<'a>, s: &'a mut Vec<Object>) -> VM<'a> {
        let main_fn = object::CompiledFunction {
            instructions: bytecode.instructions,
        };
        let main_frame = new_frame(main_fn);

        let mut frames = Vec::with_capacity(MAX_FRAMES);
        frames.push(main_frame);

        VM {
            constants: bytecode.constants,
            stack: Vec::with_capacity(STACK_SIZE),
            sp: 0,
            last_popped_stack_elem: None,
            globals: s,
            frames: frames,
            frame_index: 1,
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        while self.current_frame().ip < self.current_frame().instructions().0.len() {
            let ip = self.current_frame().ip;
            let ins = self.current_frame().instructions();
            let op = Opcode::from(ins.0[ip]);
            match op {
                Opcode::OpConstant => {
                    let const_index = read_uint16(ins, ip + 1);
                    self.current_frame().ip += 2;
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
                    let pos = read_uint16(ins, ip + 1) as usize;
                    self.current_frame().ip = pos - 1;
                }
                Opcode::OpJumpNotTruthy => {
                    let pos = read_uint16(ins, ip + 1) as usize;
                    self.current_frame().ip += 2;

                    let condition = self.pop();
                    if !Self::is_truthy(condition) {
                        self.current_frame().ip = pos - 1;
                    }
                }
                Opcode::OpNull => {
                    self.push(NULL)?;
                }
                Opcode::OpSetGlobal => {
                    let global_index = read_uint16(ins, ip + 1) as usize;
                    self.current_frame().ip += 2;
                    let popped = self.pop();
                    if self.globals.len() == global_index {
                        self.globals.push(popped);
                    } else {
                        self.globals[global_index] = popped;
                    }
                }
                Opcode::OpGetGlobal => {
                    let global_index = read_uint16(ins, ip + 1) as usize;
                    self.current_frame().ip += 2;
                    // TODO: clone() どうにかなるか
                    let obj = (&self.globals[global_index]).clone();
                    self.push(obj)?;
                }
                Opcode::OpArray => {
                    let num_elements = read_uint16(ins, ip + 1) as usize;
                    self.current_frame().ip += 2;
                    let array = self.build_array(self.sp - num_elements, self.sp);
                    self.sp -= num_elements;
                    self.push(array)?;
                }
                Opcode::OpHash => {
                    let num_elements = read_uint16(ins, ip + 1) as usize;
                    self.current_frame().ip += 2;
                    let hash = self.build_hash(self.sp - num_elements, self.sp)?;
                    self.sp -= num_elements;
                    self.push(hash)?;
                }
                Opcode::OpIndex => {
                    let index = self.pop();
                    let left = self.pop();
                    self.execute_index_expression(left, index)?;
                }
                Opcode::OpCall => {
                    let obj = &self.stack[self.sp - 1];
                    if let Object::CompiledFunction(func) = obj {
                        // TODO: clone() でいいのか？ self.stack から pop したものじゃダメ？
                        let frame = new_frame(func.clone());
                        self.push_frame(frame);
                        // self.current_frame().ip += 1; させないために continue する
                        continue;
                    } else {
                        return Err("calling non-function".to_string());
                    }
                }
                Opcode::OpReturnValue => {
                    let return_value = self.pop();
                    self.pop_frame();
                    self.pop();
                    self.push(return_value)?;
                }
                Opcode::OpReturn => {
                    self.pop_frame();
                    self.pop();
                    self.push(NULL)?;
                }
                //
                _ => todo!("unknown Opcode: {:?}", op),
            }
            self.current_frame().ip += 1;
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
            (Object::String(left_value), Object::String(right_value)) => {
                self.execute_binary_string_operation(op, left_value, right_value)?;
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

    fn execute_binary_string_operation(
        &mut self,
        op: Opcode,
        left_value: &str,
        right_value: &str,
    ) -> Result<(), String> {
        if op != Opcode::OpAdd {
            return Err(format!("unknown string operator: {:?}", op));
        }
        self.push(Object::String(left_value.to_string() + right_value))
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
            NULL => self.push(TRUE),
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

    fn build_array(&mut self, start_index: usize, end_index: usize) -> Object {
        let elements = self.stack.drain(start_index..end_index).collect();
        Object::Array(object::Array { elements })
    }

    fn build_hash(&mut self, start_index: usize, end_index: usize) -> Result<Object, String> {
        let mut drain = self.stack.drain(start_index..end_index);

        let mut pairs = HashMap::new();
        while let (Some(key), Some(value)) = (drain.next(), drain.next()) {
            let hash_key = hash_key_of(&key)?;
            let pair = object::HashPair { key, value };
            pairs.insert(hash_key, pair);
        }
        Ok(Object::Hash(object::Hash { pairs }))
    }

    fn execute_index_expression(&mut self, left: Object, index: Object) -> Result<(), String> {
        match (left, index) {
            (Object::Array(array), Object::Integer(integer)) => {
                self.execute_array_index(array, integer)
            }
            (Object::Hash(hash), i) => self.execute_hash_index(hash, i),
            (l, _) => Err(format!("index operator not supported: {}", l)),
        }
    }

    fn execute_array_index(&mut self, array: object::Array, index: i64) -> Result<(), String> {
        if index < 0 || index as usize + 1 > array.elements.len() {
            return self.push(NULL);
        }
        self.push(array.elements[index as usize].clone())
    }

    fn execute_hash_index(&mut self, hash: object::Hash, index: Object) -> Result<(), String> {
        let key = hash_key_of(&index)?;
        self.push(match hash.pairs.get(&key) {
            Some(pair) => pair.value.clone(),
            None => NULL,
        })
    }

    fn native_bool_to_boolean_object(input: bool) -> Object {
        if input {
            TRUE
        } else {
            FALSE
        }
    }

    fn is_truthy(obj: Object) -> bool {
        match obj {
            Object::Boolean(boolean) => boolean,
            Object::Null => false,
            _ => true,
        }
    }

    fn current_frame(&mut self) -> &mut Frame {
        &mut self.frames[self.frame_index - 1]
    }

    fn push_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
        self.frame_index += 1;
    }

    fn pop_frame(&mut self) -> Frame {
        self.frame_index -= 1;
        self.frames.pop().unwrap()
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
    use super::super::object::hash::hash_key_of;
    use super::super::parser::Parser;
    use super::super::test_utils::*;
    use super::*;
    use std::collections::HashMap;

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
            ("!(if (false) { 5; })", true),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_conditionals() {
        {
            let tests = vec![
                ("if (true) { 10 }", 10),
                ("if (true) { 10 } else { 20 }", 10),
                ("if (false) { 10 } else { 20 }", 20),
                ("if (1) { 10 }", 10),
                ("if (1 < 2) { 10 }", 10),
                ("if (1 < 2) { 10 } else { 20 }", 10),
                ("if (1 > 2) { 10 } else { 20 }", 20),
                ("if ((if (false) { 10 })) { 10 } else { 20 }", 20),
            ];
            run_vm_tests(tests);
        }
        {
            let tests = vec![("if (1 > 2) { 10 }", NULL), ("if (false) { 10 }", NULL)];
            run_vm_tests(tests);
        }
    }

    #[test]
    fn test_global_let_statements() {
        let tests = vec![
            ("let one = 1; one", 1),
            ("let one = 1; let two = 2; one + two", 3),
            ("let one = 1; let two = one + one; one + two", 3),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_string_expressions() {
        let tests = vec![
            (r#""monkey""#, "monkey"),
            (r#""mon" + "key""#, "monkey"),
            (r#""mon" + "key" + "banana""#, "monkeybanana"),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_array_literals() {
        let tests = vec![
            ("[]", vec![]),
            ("[1, 2, 3]", vec![1, 2, 3]),
            ("[1 + 2, 3 * 4, 5 + 6]", vec![3, 12, 11]),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_hash_literals() {
        let tests = vec![
            ("{}", vec![]),
            (
                "{1: 2, 2: 3}",
                vec![
                    (hash_key_of(&Object::Integer(1)).unwrap(), 2),
                    (hash_key_of(&Object::Integer(2)).unwrap(), 3),
                ],
            ),
            (
                "{1 + 1: 2 * 2, 3 + 3: 4 * 4}",
                vec![
                    (hash_key_of(&Object::Integer(2)).unwrap(), 4),
                    (hash_key_of(&Object::Integer(6)).unwrap(), 16),
                ],
            ),
        ]
        .into_iter()
        .map(|(input, expected)| (input, expected.into_iter().collect::<HashMap<_, _>>()))
        .collect();

        run_vm_tests(tests);
    }

    #[test]
    fn test_index_expressions() {
        {
            let tests = vec![
                ("[1, 2, 3][1]", 2),
                ("[1, 2, 3][0 + 2]", 3),
                ("[[1, 1, 1]][0][0]", 1),
                ("{1: 1, 2: 2}[1]", 1),
                ("{1: 1, 2: 2}[2]", 2),
            ];
            run_vm_tests(tests);
        }
        {
            let tests = vec![
                ("[][0]", NULL),
                ("[1, 2, 3][99]", NULL),
                ("[1][-1]", NULL),
                ("{1: 1}[0]", NULL),
                ("{}[0]", NULL),
            ];
            run_vm_tests(tests);
        }
    }

    #[test]
    fn test_calling_functions_wihtout_arguments() {
        let tests = vec![
            (
                r#"
                let fivePlusTen = fn() { 5 + 10 };
                fivePlusTen();
                "#,
                15,
            ),
            (
                r#"
                let one = fn() { 1 };
                let two = fn() { 2 };
                one() + two()
                "#,
                3,
            ),
            (
                r#"
                let a = fn() { 1 };
                let b = fn() { a() + 1 };
                let c = fn() { b() + 1 };
                c();
                "#,
                3,
            ),
        ];

        run_vm_tests(tests);
    }

    #[test]
    fn test_functions_with_return_statement() {
        let tests = vec![
            (
                r#"
                let earlyExit = fn() { return 99; 100; };
                earlyExit();
                "#,
                99,
            ),
            (
                r#"
                let earlyExit = fn() { return 99; return 100; };
                earlyExit();
                "#,
                99,
            ),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_functions_without_return_value() {
        let tests = vec![
            (
                r#"
                let noReturn = fn() { };
                noReturn();
                "#,
                NULL,
            ),
            (
                r#"
                let noReturn = fn() { };
                let noReturnTwo = fn() { noReturn() };
                noReturn();
                noReturnTwo();
                "#,
                NULL,
            ),
        ];
        run_vm_tests(tests);
    }

    #[test]
    fn test_first_class_functions() {
        let tests = vec![(
            r#"
            let returnsOne = fn() { 1; };
            let returnsOneReturner = fn() { returnsOne };
            returnsOneReturner()();
            "#,
            1,
        )];
        run_vm_tests(tests);
    }

    fn run_vm_tests<T: Expectable>(tests: Vec<(&str, T)>) {
        for (input, expected) in tests {
            let program = parse(input.to_string());
            let mut symbol_table_stack = new_symbol_table_stack();
            let mut constants = new_constants();
            let mut comp = Compiler::new_with_state(&mut symbol_table_stack, &mut constants);
            if let Err(err) = comp.compile(program) {
                assert!(false, "compile error: {}", err);
            }

            let mut globals = new_globals_store();
            let mut vm = VM::new_with_globals_store(comp.bytecode(), &mut globals);
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
}
