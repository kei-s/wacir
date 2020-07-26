use super::{compiler, lexer, parser, vm};
use std::io;
use std::io::prelude::Write;

pub fn start() {
    const PROMPT: &str = ">> ";

    let mut constants = compiler::new_constants();
    let mut globals = vm::new_globals_store();
    let mut symbol_table = compiler::new_symbol_table();

    loop {
        print!("{}", PROMPT);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let l = lexer::Lexer::new(&input);
        let mut p = parser::Parser::new(l);
        let program = p.parse_program();

        let errors = p.errors();
        if !errors.is_empty() {
            print_parser_errors(errors);
            continue;
        }

        // let mut comp = compiler::Compiler::new_with_state(&mut symbol_table, &mut constants);
        // if let Err(err) = comp.compile(program) {
        //     println!("Woops! Compilation failed:\n {}", err);
        //     continue;
        // }

        // let mut machine = vm::VM::new_with_globals_store(comp.bytecode(), &mut globals);
        // if let Err(err) = machine.run() {
        //     println!("Woops! Executing bytecode failed:\n {}", err);
        //     continue;
        // }

        // match machine.last_popped_stack_elem {
        //     Some(stack_top) => println!("{}", stack_top),
        //     None => println!("None."),
        // }
    }
}

fn print_parser_errors(errors: std::vec::Vec<String>) {
    for msg in errors {
        println!("{}", msg)
    }
}
