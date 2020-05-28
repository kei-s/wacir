mod symbol_table;

use super::ast::*;
use super::code::*;
use super::object::Object;
pub use symbol_table::new_symbol_table;
use symbol_table::*;

pub fn new_constants() -> Vec<Object> {
    Vec::new()
}

struct EmittedInstruction {
    opcode: Opcode,
    position: usize,
}

pub struct Compiler<'a> {
    instructions: Instructions,
    constants: &'a mut Vec<Object>,
    last_instruction: Option<EmittedInstruction>,
    previous_instruction: Option<EmittedInstruction>,
    symbol_table: &'a mut SymbolTable,
}

impl<'a> Compiler<'a> {
    // pub fn new() -> Compiler {
    //     Compiler {
    //         instructions: Instructions(vec![]),
    //         constants: vec![],
    //         last_instruction: None,
    //         previous_instruction: None,
    //         symbol_table: new_symbol_table(),
    //     }
    // }

    pub fn new_with_state(s: &'a mut SymbolTable, constants: &'a mut Vec<Object>) -> Compiler<'a> {
        Compiler {
            instructions: Instructions(vec![]),
            constants: constants,
            last_instruction: None,
            previous_instruction: None,
            symbol_table: s,
        }
    }

    pub fn compile(&mut self, program: Program) -> Result<(), String> {
        program.compile(self)
    }

    pub fn bytecode(self) -> ByteCode<'a> {
        ByteCode {
            instructions: self.instructions,
            constants: self.constants,
        }
    }

    pub fn emit(&mut self, op: Opcode) -> usize {
        let ins = make(op);
        self.emit_ins(op, ins)
    }

    pub fn emit_with_operands(&mut self, op: Opcode, operands: &[usize]) -> usize {
        let ins = make_with_operands(op, operands);
        self.emit_ins(op, ins)
    }

    fn emit_ins(&mut self, op: Opcode, ins: Instructions) -> usize {
        let pos = self.add_instruction(ins);
        self.set_last_instruction(op, pos);
        pos
    }

    fn add_instruction(&mut self, mut ins: Instructions) -> usize {
        let pos_new_instruction = self.instructions.0.len();
        self.instructions.0.append(&mut ins.0);
        pos_new_instruction
    }

    fn set_last_instruction(&mut self, op: Opcode, pos: usize) {
        let last = Some(EmittedInstruction {
            opcode: op,
            position: pos,
        });
        self.previous_instruction = std::mem::replace(&mut self.last_instruction, last);
    }

    pub fn add_constant(&mut self, obj: Object) -> usize {
        self.constants.push(obj);
        self.constants.len() - 1
    }

    pub fn last_instruction_is_pop(&self) -> bool {
        if let Some(emitted) = &self.last_instruction {
            emitted.opcode == Opcode::OpPop
        } else {
            false
        }
    }

    pub fn remove_last_pop(&mut self) {
        self.instructions
            .0
            .truncate(self.last_instruction.as_ref().unwrap().position);
        self.last_instruction = std::mem::replace(&mut self.previous_instruction, None);
    }

    pub fn replace_instruction(&mut self, pos: usize, new_instuction: Instructions) {
        for (i, b) in new_instuction.0.iter().enumerate() {
            self.instructions.0[pos + i] = *b;
        }
    }

    pub fn change_operand(&mut self, op_pos: usize, operand: usize) {
        let op = Opcode::from(self.instructions.0[op_pos]);
        let new_instuction = make_with_operands(op, &[operand]);
        self.replace_instruction(op_pos, new_instuction);
    }
}

trait Compile {
    fn compile(&self, compiler: &mut Compiler) -> Result<(), String>;
}

macro_rules! impl_compile {
    ($ty:ty => ($self:ident, $compiler:ident) $block:block) => {
        impl Compile for $ty {
            fn compile(&$self, $compiler: &mut Compiler) -> Result<(), String> {
                $block
            }
        }
    };
}

impl_compile!(Program => (self, compiler) {
    for s in &self.statements {
        s.compile(compiler)?
    }
    Ok(())
});

impl_compile!(Statement => (self, compiler) {
    match self {
        Statement::ExpressionStatement(stmt) => {
            stmt.expression.compile(compiler)?;
            compiler.emit(Opcode::OpPop);
            Ok(())
        },
        Statement::LetStatement(stmt) => {
            stmt.compile(compiler)
        }
        _ => todo!(),
    }
});

impl_compile!(LetStatement => (self, compiler) {
    self.value.compile(compiler)?;
    let index = compiler.symbol_table.define(&self.name.value).index;
    compiler.emit_with_operands(Opcode::OpSetGlobal, &[index]);
    Ok(())
});

impl_compile!(Expression => (self, compiler) {
    match self {
        Expression::InfixExpression(exp) => exp.compile(compiler),
        Expression::IntegerLiteral(exp) => exp.compile(compiler),
        Expression::Boolean(exp) => exp.compile(compiler),
        Expression::PrefixExpression(exp) => exp.compile(compiler),
        Expression::IfExpression(exp) => exp.compile(compiler),
        Expression::Identifier(exp) => exp.compile(compiler),
        Expression::StringLiteral(exp) => exp.compile(compiler),
        Expression::ArrayLiteral(exp) => exp.compile(compiler),
        Expression::HashLiteral(exp) => exp.compile(compiler),
        _ => todo!("other expressions: {:?}", self),
    }
});

impl_compile!(InfixExpression => (self, compiler) {
    if &*self.operator == "<" {
        self.right.compile(compiler)?;
        self.left.compile(compiler)?;
        compiler.emit(Opcode::OpGreaterThan);
        return Ok(())
    }

    self.left.compile(compiler)?;
    self.right.compile(compiler)?;

    match &*self.operator {
        "+" => {
            compiler.emit(Opcode::OpAdd);
        }
        "-" => {
            compiler.emit(Opcode::OpSub);
        }
        "*" => {
            compiler.emit(Opcode::OpMul);
        }
        "/" => {
            compiler.emit(Opcode::OpDiv);
        }
        ">" => {
            compiler.emit(Opcode::OpGreaterThan);
        }
        "==" => {
            compiler.emit(Opcode::OpEqual);
        }
        "!=" => {
            compiler.emit(Opcode::OpNotEqual);
        }
        other => return Err(format!("unknown operator {}", other))
    }
    Ok(())
});

impl_compile!(PrefixExpression => (self, compiler) {
    self.right.compile(compiler)?;
    match &*self.operator {
        "!" => compiler.emit(Opcode::OpBang),
        "-" => compiler.emit(Opcode::OpMinus),
        other => return Err(format!("unknown operator {}", other))
    };
    Ok(())
});

impl_compile!(IntegerLiteral => (self, compiler) {
    let integer = Object::Integer(self.value);
    let constant = compiler.add_constant(integer);
    compiler.emit_with_operands(Opcode::OpConstant, &[constant]);
    Ok(())
});

impl_compile!(Boolean => (self, compiler) {
    if self.value {
        compiler.emit(Opcode::OpTrue);
    } else {
        compiler.emit(Opcode::OpFalse);
    }
    Ok(())
});

impl_compile!(IfExpression => (self, compiler) {
    self.condition.compile(compiler)?;

    let jump_not_truthy_pos = compiler.emit_with_operands(Opcode::OpJumpNotTruthy, &[9999]);

    self.consequence.compile(compiler)?;

    if compiler.last_instruction_is_pop() {
        compiler.remove_last_pop()
    }

    let jump_pos = compiler.emit_with_operands(Opcode::OpJump, &[9999]);

    let after_consequense_pos = compiler.instructions.0.len();
    compiler.change_operand(jump_not_truthy_pos, after_consequense_pos);

    if let Some(alternative) = &self.alternative {
        alternative.compile(compiler)?;

        if compiler.last_instruction_is_pop() {
            compiler.remove_last_pop();
        }
    } else {
        compiler.emit(Opcode::OpNull);
    }

    let after_alternative_pos = compiler.instructions.0.len();
    compiler.change_operand(jump_pos, after_alternative_pos);

    Ok(())
});

impl_compile!(BlockStatement => (self, compiler) {
    for s in &self.statements {
        s.compile(compiler)?;
    }
    Ok(())
});

impl_compile!(Identifier => (self, compiler) {
    let index = compiler.symbol_table.resolve(&self.value).expect(
        &format!("undefined variable: {}", self.value)
    ).index;
    compiler.emit_with_operands(Opcode::OpGetGlobal, &[index]);
    Ok(())
});

impl_compile!(StringLiteral => (self, compiler) {
    let string = Object::String(self.value.clone());
    let constant = compiler.add_constant(string);
    compiler.emit_with_operands(Opcode::OpConstant, &[constant]);
    Ok(())
});

impl_compile!(ArrayLiteral => (self, compiler) {
    for el in &self.elements {
        el.compile(compiler)?;
    }
    compiler.emit_with_operands(Opcode::OpArray, &[self.elements.len()]);
    Ok(())
});

impl_compile!(HashLiteral => (self, compiler) {
    for (key, value) in &self.pairs {
        key.compile(compiler)?;
        value.compile(compiler)?;
    }
    compiler.emit_with_operands(Opcode::OpHash, &[self.pairs.len() * 2]);
    Ok(())
});

pub struct ByteCode<'a> {
    pub instructions: Instructions,
    pub constants: &'a mut Vec<Object>,
}

#[cfg(test)]
mod tests {
    use super::super::ast::Program;
    use super::super::code::*;
    use super::super::lexer::Lexer;
    use super::super::parser::Parser;
    use super::super::test_utils::*;
    use super::*;

    #[test]
    fn test_integer_arithmetic() {
        let tests = vec![
            (
                "1 + 2",
                vec![1, 2],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make_with_operands(Opcode::OpConstant, &vec![1]),
                    make(Opcode::OpAdd),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "1; 2",
                vec![1, 2],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make(Opcode::OpPop),
                    make_with_operands(Opcode::OpConstant, &vec![1]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "1 - 2",
                vec![1, 2],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make_with_operands(Opcode::OpConstant, &vec![1]),
                    make(Opcode::OpSub),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "1 * 2",
                vec![1, 2],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make_with_operands(Opcode::OpConstant, &vec![1]),
                    make(Opcode::OpMul),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "2 / 1",
                vec![2, 1],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make_with_operands(Opcode::OpConstant, &vec![1]),
                    make(Opcode::OpDiv),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "-1",
                vec![1],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make(Opcode::OpMinus),
                    make(Opcode::OpPop),
                ],
            ),
        ];

        run_compile_tests(tests);
    }

    #[test]
    fn test_boolean_expressions() {
        let tests = vec![
            (
                "true",
                vec![],
                vec![make(Opcode::OpTrue), make(Opcode::OpPop)],
            ),
            (
                "false",
                vec![],
                vec![make(Opcode::OpFalse), make(Opcode::OpPop)],
            ),
            (
                "1 > 2",
                vec![1, 2],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make_with_operands(Opcode::OpConstant, &vec![1]),
                    make(Opcode::OpGreaterThan),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "1 < 2",
                vec![2, 1],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make_with_operands(Opcode::OpConstant, &vec![1]),
                    make(Opcode::OpGreaterThan),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "1 == 2",
                vec![1, 2],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make_with_operands(Opcode::OpConstant, &vec![1]),
                    make(Opcode::OpEqual),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "1 != 2",
                vec![1, 2],
                vec![
                    make_with_operands(Opcode::OpConstant, &vec![0]),
                    make_with_operands(Opcode::OpConstant, &vec![1]),
                    make(Opcode::OpNotEqual),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "true == false",
                vec![],
                vec![
                    make(Opcode::OpTrue),
                    make(Opcode::OpFalse),
                    make(Opcode::OpEqual),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "true != false",
                vec![],
                vec![
                    make(Opcode::OpTrue),
                    make(Opcode::OpFalse),
                    make(Opcode::OpNotEqual),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "!true",
                vec![],
                vec![
                    make(Opcode::OpTrue),
                    make(Opcode::OpBang),
                    make(Opcode::OpPop),
                ],
            ),
        ];

        run_compile_tests(tests);
    }

    #[test]
    fn test_conditionals() {
        let tests = vec![
            (
                "if (true) { 10 }; 3333;",
                vec![10, 3333],
                vec![
                    // 0000
                    make(Opcode::OpTrue),
                    // 0001
                    make_with_operands(Opcode::OpJumpNotTruthy, &[10]),
                    // 0004
                    make_with_operands(Opcode::OpConstant, &[0]),
                    // 0007
                    make_with_operands(Opcode::OpJump, &[11]),
                    // 0010
                    make(Opcode::OpNull),
                    // 0011
                    make(Opcode::OpPop),
                    // 0008
                    make_with_operands(Opcode::OpConstant, &[1]),
                    // 0011
                    make(Opcode::OpPop),
                ],
            ),
            (
                "if (true) { 10 } else { 20 }; 3333;",
                vec![10, 20, 3333],
                vec![
                    // 0000
                    make(Opcode::OpTrue),
                    // 0001
                    make_with_operands(Opcode::OpJumpNotTruthy, &[10]),
                    // 0004
                    make_with_operands(Opcode::OpConstant, &[0]),
                    // 0007
                    make_with_operands(Opcode::OpJump, &[13]),
                    // 0010
                    make_with_operands(Opcode::OpConstant, &[1]),
                    // 0013
                    make(Opcode::OpPop),
                    // 0014
                    make_with_operands(Opcode::OpConstant, &[2]),
                    // 0017
                    make(Opcode::OpPop),
                ],
            ),
        ];

        run_compile_tests(tests);
    }

    #[test]
    fn test_global_let_statements() {
        let tests = vec![
            (
                r"
                let one = 1;
                let two = 2;
                ",
                vec![1, 2],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpSetGlobal, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpSetGlobal, &[1]),
                ],
            ),
            (
                r"
                let one = 1;
                one;
                ",
                vec![1],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpSetGlobal, &[0]),
                    make_with_operands(Opcode::OpGetGlobal, &[0]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                r"
                let one = 1;
                let two = one;
                two;
                ",
                vec![1],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpSetGlobal, &[0]),
                    make_with_operands(Opcode::OpGetGlobal, &[0]),
                    make_with_operands(Opcode::OpSetGlobal, &[1]),
                    make_with_operands(Opcode::OpGetGlobal, &[1]),
                    make(Opcode::OpPop),
                ],
            ),
        ];
        run_compile_tests(tests);
    }

    #[test]
    fn test_string_expressions() {
        let tests = vec![
            (
                r#""monkey""#,
                vec!["monkey"],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                r#""mon" + "key""#,
                vec!["mon", "key"],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make(Opcode::OpAdd),
                    make(Opcode::OpPop),
                ],
            ),
        ];

        run_compile_tests(tests)
    }

    #[test]
    fn test_array_literals() {
        let tests = vec![
            (
                "[]",
                vec![],
                vec![
                    make_with_operands(Opcode::OpArray, &[0]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "[1, 2, 3]",
                vec![1, 2, 3],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make_with_operands(Opcode::OpArray, &[3]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "[1 + 2, 3 - 4, 5 * 6]",
                vec![1, 2, 3, 4, 5, 6],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make(Opcode::OpAdd),
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make_with_operands(Opcode::OpConstant, &[3]),
                    make(Opcode::OpSub),
                    make_with_operands(Opcode::OpConstant, &[4]),
                    make_with_operands(Opcode::OpConstant, &[5]),
                    make(Opcode::OpMul),
                    make_with_operands(Opcode::OpArray, &[3]),
                    make(Opcode::OpPop),
                ],
            ),
        ];
        run_compile_tests(tests)
    }

    #[test]
    fn test_hash_literals() {
        let tests = vec![
            (
                "{}",
                vec![],
                vec![
                    make_with_operands(Opcode::OpHash, &[0]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "{1: 2, 3: 4, 5: 6}",
                vec![1, 2, 3, 4, 5, 6],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make_with_operands(Opcode::OpConstant, &[3]),
                    make_with_operands(Opcode::OpConstant, &[4]),
                    make_with_operands(Opcode::OpConstant, &[5]),
                    make_with_operands(Opcode::OpHash, &[6]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "{1: 2 + 3, 4: 5 * 6}",
                vec![1, 2, 3, 4, 5, 6],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make(Opcode::OpAdd),
                    make_with_operands(Opcode::OpConstant, &[3]),
                    make_with_operands(Opcode::OpConstant, &[4]),
                    make_with_operands(Opcode::OpConstant, &[5]),
                    make(Opcode::OpMul),
                    make_with_operands(Opcode::OpHash, &[4]),
                    make(Opcode::OpPop),
                ],
            ),
        ];
        run_compile_tests(tests)
    }

    fn run_compile_tests<T: Expectable>(tests: Vec<(&str, Vec<T>, Vec<Instructions>)>) {
        for (input, expected_constants, expected_instructions) in tests {
            let program = parse(input.to_string());

            let mut symbol_table = new_symbol_table();
            let mut constants = Vec::new();
            let mut compiler = Compiler::new_with_state(&mut symbol_table, &mut constants);
            if let Err(err) = compiler.compile(program) {
                assert!(false, "compile error. {}", err)
            }

            let bytecode = compiler.bytecode();

            test_instructions(expected_instructions, bytecode.instructions);
            test_constants(expected_constants, bytecode.constants.to_vec());
        }
    }

    fn parse(input: String) -> Program {
        let l = Lexer::new(&input);
        let mut p = Parser::new(l);
        p.parse_program()
    }

    fn test_instructions(expected: Vec<Instructions>, actual: Instructions) {
        let concated = expected.concat();
        assert_eq!(concated, actual);
    }

    fn test_constants<T: Expectable>(expected: Vec<T>, actual: Vec<Object>) {
        assert_eq!(expected.len(), actual.len());

        for (constant, object) in expected.iter().zip(actual.iter()) {
            test_expected_object(constant, object)
        }
    }
}
