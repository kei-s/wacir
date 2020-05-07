extern crate byteorder;
use byteorder::{WriteBytesExt, BigEndian};

type Instructions = Vec<u8>;
type Opcode = u8;

pub enum OpcodeType {
  OpConstant
}

pub struct Definition {
  pub name: String,
  pub operand_width: Vec<usize>
}

pub fn lookup(op: Opcode) -> Result<Definition, String> {
  match op {
    // TODO: 0u8
    0u8 => Ok(Definition{ name: "OpConstant".to_string(), operand_width: vec![2] }),
    _ => Err(format!("Opcode {} undefined.", op))
  }
}

pub fn make(op: Opcode, operand: u16) -> Instructions {
  let def = match lookup(op) {
    Ok(def) => def,
    Err(_) => return vec![]
  };

  let mut instruction_len = 1;
  for w in &def.operand_width {
    instruction_len += w;
  }

  let mut instruction = Vec::with_capacity(instruction_len);
  instruction.push(op);

  let mut vec = Vec::new();
  vec.write_u16::<BigEndian>(operand).unwrap();
  instruction.append(&mut vec);

  instruction
}

#[cfg(test)]
mod tests {
  use super::OpcodeType;
  use super::make;

  #[test]
  fn test_make() {
      let tests = vec![
        ( 0u8, 65534, vec![0u8, 255u8, 254u8])
      ];

      for (op, operand, expected) in tests {
        let instruction = make(op, operand);

        assert_eq!(instruction, expected);
      }
  }
}
