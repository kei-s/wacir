extern crate byteorder;
use byteorder::{WriteBytesExt, BigEndian};

type Instructions = Vec<u8>;
type Opcode = u8;

pub enum OpcodeType {
  OpConstant
}

impl OpcodeType {
  pub fn from(op: Opcode) -> OpcodeType {
    match op {
      0 => OpcodeType::OpConstant,
      _ => unimplemented!()
    }
  }

  pub fn opcode(self) -> Opcode {
    match self {
      OpcodeType::OpConstant => 0,
    }
  }
}

pub struct Definition {
  pub name: String,
  pub operand_width: Vec<usize>
}

pub fn lookup(op: Opcode) -> Definition {
  match OpcodeType::from(op) {
    OpcodeType::OpConstant => Definition{ name: "OpConstant".to_string(), operand_width: vec![2] }
  }
}

pub fn make(op: Opcode, operand: u16) -> Instructions {
  let def = lookup(op);

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
        ( OpcodeType::OpConstant.opcode(), 65534, vec![0u8, 255u8, 254u8])
      ];

      for (op, operand, expected) in tests {
        let instruction = make(op, operand);

        assert_eq!(instruction, expected);
      }
  }
}
