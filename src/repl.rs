use super::{compiler, lexer, parser, vm};
use std::io;
use std::io::prelude::Write;

pub fn start() {
  const PROMPT: &str = ">> ";

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

    let mut comp = compiler::Compiler::new();
    if let Err(err) = comp.compile(program) {
      println!("Woops! Compilation failed:\n {}", err);
      continue;
    }

    let mut machine = vm::VM::new(comp.bytecode());
    if let Err(err) = machine.run() {
      println!("Woops! Executing bytecode failed:\n {}", err);
      continue;
    }

    match machine.stack_top() {
      Some(stack_top) => println!("{}", stack_top),
      None => println!("None."),
    }
  }
}

fn print_parser_errors(errors: std::vec::Vec<String>) {
  for msg in errors {
    println!("{}", msg)
  }
}
