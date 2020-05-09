use super::ast::Program;
use super::code::{concat_instructions, Instructions};
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
        Ok(())
    }

    pub fn bytecode(self) -> ByteCode {
        ByteCode {
            instructions: self.instructions,
            constants: self.constants,
        }
    }
}

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
        let concated = concat_instructions(expected);
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
