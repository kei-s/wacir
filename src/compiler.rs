use super::ast::*;
use super::code::*;
use super::object::Object;

pub struct Compiler {
    instructions: Instructions,
    constants: Vec<Object>,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            instructions: Instructions(vec![]),
            constants: vec![],
        }
    }

    pub fn compile(&mut self, program: Program) -> Result<(), String> {
        program.compile(self)
    }

    pub fn bytecode(self) -> ByteCode {
        ByteCode {
            instructions: self.instructions,
            constants: self.constants,
        }
    }

    pub fn emit(&mut self, op: &Opcode, operand: usize) -> usize {
        // TODO: `as u16` is OK?
        let mut ins = make(op, operand as u16);
        let pos = self.add_instruction(&mut ins);
        pos
    }

    pub fn add_instruction(&mut self, ins: &mut Instructions) -> usize {
        let pos_new_instruction = self.instructions.0.len();
        self.instructions.0.append(&mut ins.0);
        pos_new_instruction
    }

    pub fn add_constant(&mut self, obj: Object) -> usize {
        self.constants.push(obj);
        self.constants.len() - 1
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
        Statement::ExpressionStatement(stmt) => stmt.expression.compile(compiler),
        _ => todo!(),
    }
});

impl_compile!(Expression => (self, compiler) {
    match self {
        Expression::InfixExpression(exp) => exp.compile(compiler),
        Expression::IntegerLiteral(exp) => exp.compile(compiler),
        _ => todo!(),
    }
});

impl_compile!(InfixExpression => (self, compiler) {
    self.left.compile(compiler)?;
    self.right.compile(compiler)
});

impl_compile!(IntegerLiteral => (self, compiler) {
    let integer = Object::Integer(self.value);
    let constant = compiler.add_constant(integer);
    compiler.emit(&OpcodeType::OpConstant.opcode(), constant);
    Ok(())
});

pub struct ByteCode {
    instructions: Instructions,
    constants: Vec<Object>,
}

#[cfg(test)]
mod tests {
    use super::super::ast::Program;
    use super::super::code::{make, OpcodeType};
    use super::super::lexer::Lexer;
    use super::super::parser::Parser;
    use super::*;

    #[test]
    fn test_integer_arithmetic() {
        let tests = vec![(
            "1 + 2".to_string(),
            vec![1, 2],
            vec![
                make(&OpcodeType::OpConstant.opcode(), 0),
                make(&OpcodeType::OpConstant.opcode(), 1),
            ],
        )];

        run_compile_tests(tests);
    }

    fn run_compile_tests(tests: Vec<(String, Vec<i64>, Vec<Instructions>)>) {
        for (input, expected_constants, expected_instructions) in tests {
            let program = parse(input);

            let mut compiler = Compiler::new();
            if let Err(err) = compiler.compile(program) {
                assert!(false, "compile error. {}", err)
            }

            let bytecode = compiler.bytecode();

            test_instructions(expected_instructions, bytecode.instructions);
            test_constants(expected_constants, bytecode.constants);
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

    fn test_constants(expected: Vec<i64>, actual: Vec<Object>) {
        assert_eq!(expected.len(), actual.len());

        for (constant, object) in expected.iter().zip(actual.iter()) {
            test_integer_object(constant, object)
        }
    }

    fn test_integer_object(expected: &i64, actual: &Object) {
        if let Object::Integer(integer) = actual {
            assert_eq!(expected, integer);
        } else {
            assert!(false, "object is not Integer. {}", actual)
        }
    }
}
