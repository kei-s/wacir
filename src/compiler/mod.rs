mod symbol_table;

use super::ast::*;
use super::code::*;
use super::object;
use super::object::Object;
pub use symbol_table::new_symbol_table_stack;
use symbol_table::*;

pub fn new_constants() -> Vec<Object> {
    vec![]
}

struct EmittedInstruction {
    opcode: Opcode,
    position: usize,
}

struct CompilationScope {
    instructions: Instructions,
    last_instruction: Option<EmittedInstruction>,
    previous_instruction: Option<EmittedInstruction>,
}

pub struct Compiler<'a> {
    constants: &'a mut Vec<Object>,
    symbol_table_stack: &'a mut SymbolTableStack,
    scopes: Vec<CompilationScope>,
    scope_index: usize,
}

impl<'a> Compiler<'a> {
    pub fn new_with_state(
        s: &'a mut SymbolTableStack,
        constants: &'a mut Vec<Object>,
    ) -> Compiler<'a> {
        let main_scope = CompilationScope {
            instructions: Instructions(vec![]),
            last_instruction: None,
            previous_instruction: None,
        };
        Compiler {
            constants: constants,
            symbol_table_stack: s,
            scopes: vec![main_scope],
            scope_index: 0,
        }
    }

    pub fn compile(&mut self, program: Program) -> Result<(), String> {
        program.compile(self)
    }

    pub fn bytecode(mut self) -> ByteCode<'a> {
        ByteCode {
            instructions: self.scopes.pop().unwrap().instructions,
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

    fn current_instructions(&mut self) -> &mut Instructions {
        &mut self.scopes[self.scope_index].instructions
    }

    fn add_instruction(&mut self, mut ins: Instructions) -> usize {
        let pos_new_instruction = self.current_instructions().0.len();
        self.current_instructions().0.append(&mut ins.0);
        pos_new_instruction
    }

    fn set_last_instruction(&mut self, op: Opcode, pos: usize) {
        let last = Some(EmittedInstruction {
            opcode: op,
            position: pos,
        });
        self.scopes[self.scope_index].previous_instruction =
            std::mem::replace(&mut self.scopes[self.scope_index].last_instruction, last);
    }

    pub fn add_constant(&mut self, obj: Object) -> usize {
        self.constants.push(obj);
        self.constants.len() - 1
    }

    pub fn last_instruction_is(&self, op: Opcode) -> bool {
        if let Some(emitted) = &self.scopes[self.scope_index].last_instruction {
            emitted.opcode == op
        } else {
            false
        }
    }

    pub fn remove_last_pop(&mut self) {
        let position = self.scopes[self.scope_index]
            .last_instruction
            .as_ref()
            .unwrap()
            .position;
        self.current_instructions().0.truncate(position);
        self.scopes[self.scope_index].last_instruction = std::mem::replace(
            &mut self.scopes[self.scope_index].previous_instruction,
            None,
        );
    }

    pub fn replace_instruction(&mut self, pos: usize, new_instuction: Instructions) {
        for (i, b) in new_instuction.0.iter().enumerate() {
            self.current_instructions().0[pos + i] = *b;
        }
    }

    pub fn change_operand(&mut self, op_pos: usize, operand: usize) {
        let op = Opcode::from(self.current_instructions().0[op_pos]);
        let new_instuction = make_with_operands(op, &[operand]);
        self.replace_instruction(op_pos, new_instuction);
    }

    pub fn enter_scope(&mut self) {
        let scope = CompilationScope {
            instructions: Instructions(vec![]),
            last_instruction: None,
            previous_instruction: None,
        };
        self.scopes.push(scope);
        self.scope_index += 1;
        self.symbol_table_stack.push();
    }

    pub fn leave_scope(&mut self) -> Instructions {
        let instructions = self.scopes.pop().unwrap().instructions;
        self.scope_index -= 1;
        self.symbol_table_stack.pop();
        instructions
    }

    pub fn replace_last_pop_with_return(&mut self) {
        let last_pos = self.scopes[self.scope_index]
            .last_instruction
            .as_ref()
            .unwrap()
            .position;
        self.replace_instruction(last_pos, make(Opcode::OpReturnValue));
        self.scopes[self.scope_index]
            .last_instruction
            .as_mut()
            .unwrap()
            .opcode = Opcode::OpReturnValue;
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
        Statement::ReturnStatement(stmt) => {
            stmt.return_value.compile(compiler)?;
            compiler.emit(Opcode::OpReturnValue);
            Ok(())
        }
    }
});

impl_compile!(LetStatement => (self, compiler) {
    self.value.compile(compiler)?;
    let symbol = compiler.symbol_table_stack.define(&self.name.value);
    let op = if symbol.is_global() {
        Opcode::OpSetGlobal
    } else {
        Opcode::OpSetLocal
    };
    let index = symbol.index.clone();
    compiler.emit_with_operands(op, &[index]);
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
        Expression::IndexExpression(exp) => exp.compile(compiler),
        Expression::FunctionLiteral(exp) => exp.compile(compiler),
        Expression::CallExpression(exp) => exp.compile(compiler),
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

    if compiler.last_instruction_is(Opcode::OpPop) {
        compiler.remove_last_pop()
    }

    let jump_pos = compiler.emit_with_operands(Opcode::OpJump, &[9999]);

    let after_consequense_pos = compiler.current_instructions().0.len();
    compiler.change_operand(jump_not_truthy_pos, after_consequense_pos);

    if let Some(alternative) = &self.alternative {
        alternative.compile(compiler)?;

        if compiler.last_instruction_is(Opcode::OpPop) {
            compiler.remove_last_pop();
        }
    } else {
        compiler.emit(Opcode::OpNull);
    }

    let after_alternative_pos = compiler.current_instructions().0.len();
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
    let symbol = compiler.symbol_table_stack.resolve(&self.value).expect(
        &format!("undefined variable: {}", self.value)
    );
    let op = if symbol.is_global() {
        Opcode::OpGetGlobal
    } else {
        Opcode::OpGetLocal
    };
    let index = symbol.index.clone();
    compiler.emit_with_operands(op, &[index]);
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

impl_compile!(IndexExpression => (self, compiler) {
    self.left.compile(compiler)?;
    self.index.compile(compiler)?;
    compiler.emit(Opcode::OpIndex);
    Ok(())
});

impl_compile!(FunctionLiteral => (self, compiler) {
    compiler.enter_scope();
    for p in &self.parameters {
        compiler.symbol_table_stack.define(&p.value);
    }
    self.body.compile(compiler)?;
    if compiler.last_instruction_is(Opcode::OpPop) {
        compiler.replace_last_pop_with_return();
    }
    if !compiler.last_instruction_is(Opcode::OpReturnValue) {
        compiler.emit(Opcode::OpReturn);
    }
    let num_locals = compiler.symbol_table_stack.last().num_definitions;
    let instructions = compiler.leave_scope();
    let compiled_fn = Object::CompiledFunction(object::CompiledFunction{instructions, num_locals});
    let operand = compiler.add_constant(compiled_fn);
    compiler.emit_with_operands(Opcode::OpConstant, &[operand]);
    Ok(())
});

impl_compile!(CallExpression => (self, compiler) {
    self.function.compile(compiler)?;
    for a in &self.arguments {
        a.compile(compiler)?;
    }
    compiler.emit_with_operands(Opcode::OpCall, &[self.arguments.len()]);
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

    #[test]
    fn test_index_expresssions() {
        let tests = vec![
            (
                "[1, 2, 3][1 + 1]",
                vec![1, 2, 3, 1, 1],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make_with_operands(Opcode::OpArray, &[3]),
                    make_with_operands(Opcode::OpConstant, &[3]),
                    make_with_operands(Opcode::OpConstant, &[4]),
                    make(Opcode::OpAdd),
                    make(Opcode::OpIndex),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "{1: 2}[2 - 1]",
                vec![1, 2, 2, 1],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpHash, &[2]),
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make_with_operands(Opcode::OpConstant, &[3]),
                    make(Opcode::OpSub),
                    make(Opcode::OpIndex),
                    make(Opcode::OpPop),
                ],
            ),
        ];
        run_compile_tests(tests)
    }

    #[test]
    fn test_functions() {
        let tests = vec![
            (
                "fn() { return 5 + 10 }",
                vec![
                    Expect::Integer(5),
                    Expect::Integer(10),
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpConstant, &[0]),
                        make_with_operands(Opcode::OpConstant, &[1]),
                        make(Opcode::OpAdd),
                        make(Opcode::OpReturnValue),
                    ]),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "fn() { 5 + 10 }",
                vec![
                    Expect::Integer(5),
                    Expect::Integer(10),
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpConstant, &[0]),
                        make_with_operands(Opcode::OpConstant, &[1]),
                        make(Opcode::OpAdd),
                        make(Opcode::OpReturnValue),
                    ]),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                "fn() { 1; 2 }",
                vec![
                    Expect::Integer(1),
                    Expect::Integer(2),
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpConstant, &[0]),
                        make(Opcode::OpPop),
                        make_with_operands(Opcode::OpConstant, &[1]),
                        make(Opcode::OpReturnValue),
                    ]),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make(Opcode::OpPop),
                ],
            ),
        ];
        run_compile_tests(tests)
    }

    #[test]
    fn test_functions_without_return_value() {
        let tests = vec![(
            "fn() { }",
            vec![Expect::Instructions(vec![make(Opcode::OpReturn)])],
            vec![
                make_with_operands(Opcode::OpConstant, &[0]),
                make(Opcode::OpPop),
            ],
        )];
        run_compile_tests(tests);
    }

    #[test]
    fn test_compiler_scopes() {
        let mut symbol_table_stack = new_symbol_table_stack();
        let mut constants = new_constants();
        let mut compiler = Compiler::new_with_state(&mut symbol_table_stack, &mut constants);
        assert_eq!(compiler.scope_index, 0);
        compiler.emit(Opcode::OpMul);

        compiler.enter_scope();
        assert_eq!(compiler.scope_index, 1);
        compiler.emit(Opcode::OpSub);
        assert_eq!(
            compiler.scopes[compiler.scope_index].instructions.0.len(),
            1
        );
        let last = &compiler.scopes[compiler.scope_index].last_instruction;
        assert_eq!(last.as_ref().unwrap().opcode, Opcode::OpSub);
        assert_eq!(compiler.symbol_table_stack.stack.len(), 2);

        compiler.leave_scope();
        assert_eq!(compiler.scope_index, 0);
        assert_eq!(compiler.symbol_table_stack.stack.len(), 1);

        compiler.emit(Opcode::OpAdd);
        assert_eq!(
            compiler.scopes[compiler.scope_index].instructions.0.len(),
            2
        );
        let last = &compiler.scopes[compiler.scope_index].last_instruction;
        assert_eq!(last.as_ref().unwrap().opcode, Opcode::OpAdd);
        let previous = &compiler.scopes[compiler.scope_index].previous_instruction;
        assert_eq!(previous.as_ref().unwrap().opcode, Opcode::OpMul);
    }

    #[test]
    fn test_function_calls() {
        let tests = vec![
            (
                "fn() { 24 }()",
                vec![
                    Expect::Integer(24),
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpConstant, &[0]),
                        make(Opcode::OpReturnValue),
                    ]),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpCall, &[0]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                r#"
                let noArg = fn() { 24 };
                noArg();
                "#,
                vec![
                    Expect::Integer(24),
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpConstant, &[0]),
                        make(Opcode::OpReturnValue),
                    ]),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpSetGlobal, &[0]),
                    make_with_operands(Opcode::OpGetGlobal, &[0]),
                    make_with_operands(Opcode::OpCall, &[0]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                r#"
                let oneArg = fn(a) { a };
                oneArg(24);
                "#,
                vec![
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpGetLocal, &[0]),
                        make(Opcode::OpReturnValue),
                    ]),
                    Expect::Integer(24),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpSetGlobal, &[0]),
                    make_with_operands(Opcode::OpGetGlobal, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpCall, &[1]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                r#"
                let manyArg = fn(a, b, c) { a; b; c };
                manyArg(24, 25, 26);
                "#,
                vec![
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpGetLocal, &[0]),
                        make(Opcode::OpPop),
                        make_with_operands(Opcode::OpGetLocal, &[1]),
                        make(Opcode::OpPop),
                        make_with_operands(Opcode::OpGetLocal, &[2]),
                        make(Opcode::OpReturnValue),
                    ]),
                    Expect::Integer(24),
                    Expect::Integer(25),
                    Expect::Integer(26),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpSetGlobal, &[0]),
                    make_with_operands(Opcode::OpGetGlobal, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make_with_operands(Opcode::OpConstant, &[3]),
                    make_with_operands(Opcode::OpCall, &[3]),
                    make(Opcode::OpPop),
                ],
            ),
        ];
        run_compile_tests(tests);
    }

    #[test]
    fn test_let_statement_scopes() {
        let tests = vec![
            (
                r#"
                let num = 55;
                fn() { num }
                "#,
                vec![
                    Expect::Integer(55),
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpGetGlobal, &[0]),
                        make(Opcode::OpReturnValue),
                    ]),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[0]),
                    make_with_operands(Opcode::OpSetGlobal, &[0]),
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                r#"
                fn() {
                    let num = 55;
                    num
                }
                "#,
                vec![
                    Expect::Integer(55),
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpConstant, &[0]),
                        make_with_operands(Opcode::OpSetLocal, &[0]),
                        make_with_operands(Opcode::OpGetLocal, &[0]),
                        make(Opcode::OpReturnValue),
                    ]),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[1]),
                    make(Opcode::OpPop),
                ],
            ),
            (
                r#"
                fn() {
                    let a = 55;
                    let b = 77;
                    a + b
                }
                "#,
                vec![
                    Expect::Integer(55),
                    Expect::Integer(77),
                    Expect::Instructions(vec![
                        make_with_operands(Opcode::OpConstant, &[0]),
                        make_with_operands(Opcode::OpSetLocal, &[0]),
                        make_with_operands(Opcode::OpConstant, &[1]),
                        make_with_operands(Opcode::OpSetLocal, &[1]),
                        make_with_operands(Opcode::OpGetLocal, &[0]),
                        make_with_operands(Opcode::OpGetLocal, &[1]),
                        make(Opcode::OpAdd),
                        make(Opcode::OpReturnValue),
                    ]),
                ],
                vec![
                    make_with_operands(Opcode::OpConstant, &[2]),
                    make(Opcode::OpPop),
                ],
            ),
        ];

        run_compile_tests(tests);
    }

    fn run_compile_tests<T: Expectable>(tests: Vec<(&str, Vec<T>, Vec<Instructions>)>) {
        for (input, expected_constants, expected_instructions) in tests {
            let program = parse(input.to_string());

            let mut symbol_table_stack = new_symbol_table_stack();
            let mut constants = new_constants();
            let mut compiler = Compiler::new_with_state(&mut symbol_table_stack, &mut constants);
            if let Err(err) = compiler.compile(program) {
                assert!(false, "compile error. {}", err)
            }

            let bytecode = compiler.bytecode();

            test_instructions(&expected_instructions, &bytecode.instructions);
            test_constants(expected_constants, bytecode.constants.to_vec());
        }
    }

    fn parse(input: String) -> Program {
        let l = Lexer::new(&input);
        let mut p = Parser::new(l);
        p.parse_program()
    }

    fn test_constants<T: Expectable>(expected: Vec<T>, actual: Vec<Object>) {
        assert_eq!(expected.len(), actual.len());

        for (constant, object) in expected.iter().zip(actual.iter()) {
            test_expected_object(constant, object)
        }
    }
}
