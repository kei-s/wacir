use std::convert::TryInto;

#[derive(PartialEq)]
pub struct Instructions(pub Vec<u8>);

pub trait ConcatInstructions {
    fn concat(self) -> Instructions;
}

impl ConcatInstructions for Vec<Instructions> {
    fn concat(self) -> Instructions {
        Instructions(
            self.into_iter()
                .map(|ins| ins.0)
                .collect::<Vec<Vec<u8>>>()
                .concat(),
        )
    }
}

impl std::fmt::Debug for Instructions {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut out = String::new();

        let mut i = 0;
        while i < self.0.len() {
            let def = lookup(&Opcode::from(self.0[i]));
            let (operands, read) = read_operands(&def, &self.0[i + 1..]);
            out.push_str(&format!(
                "{:04} {}\n",
                i,
                self.fmt_instruction(def, operands)
            ));
            i += 1 + read;
        }

        write!(f, "{}", out)
    }
}

impl Instructions {
    fn fmt_instruction(&self, def: Definition, operands: Vec<usize>) -> String {
        let operand_count = def.operand_width.len();

        if operands.len() != operand_count {
            return format!(
                "ERROR: operand len {} does not match defined {}\n",
                operands.len(),
                operand_count
            );
        }

        match operand_count {
            0 => def.name,
            1 => format!("{} {}", def.name, operands[0]),
            _ => format!("ERROR: unhundled operand_count for {}", def.name),
        }
    }
}

// pub enum Opcode {
//     OpConstant,
//     OpAdd,
// }
//
// impl Opcode {
//     pub fn byte(self) -> u8 {
//         self as u8
//     }
//
//     pub fn from(byte: u8) -> Opcode {
//         if byte == Opcode::OpConstant.byte() {
//             return Opcode::OpConstant;
//         }
//         if byte == Opcode::OpAdd.byte() {
//             return Opcode::OpAdd;
//         }
//         unreachable!("No such opcode {}", byte)
//     }
// }
macro_rules! opcode_enum {
    ($name:ident, [ $($var:ident),+ ]) => {
        #[repr(u8)]
        pub enum $name {
            $($var,)+
        }

        impl $name {
            pub fn byte(self) -> u8 {
                self as u8
            }

            pub fn from(byte: u8) -> $name {
                $(
                    if byte == $name::$var.byte() {
                        return $name::$var;
                    }
                )+
                panic!("No such opcode {}", byte)
            }
        }
    };
}

opcode_enum!(Opcode, [OpConstant, OpAdd, OpPop]);

pub struct Definition {
    pub name: String,
    pub operand_width: Vec<usize>,
}

pub fn lookup(op: &Opcode) -> Definition {
    match op {
        Opcode::OpConstant => Definition {
            name: "OpConstant".to_string(),
            operand_width: vec![2],
        },
        Opcode::OpAdd => Definition {
            name: "OpAdd".to_string(),
            operand_width: vec![],
        },
        Opcode::OpPop => Definition {
            name: "OpPop".to_string(),
            operand_width: vec![],
        },
    }
}

pub fn make(op: Opcode, operands: &Vec<usize>) -> Instructions {
    let def = lookup(&op);

    let mut instruction_len = 1;
    for w in &def.operand_width {
        instruction_len += w;
    }

    let mut instruction: Vec<u8> = Vec::with_capacity(instruction_len);
    instruction.push(op.byte());

    for i in 0..operands.len() {
        let o = operands[i] as u16;
        let width = def.operand_width[i];
        match width {
            2 => {
                let bytes = o.to_be_bytes();
                instruction.extend_from_slice(&bytes);
            }
            _ => unreachable!(),
        }
    }

    Instructions(instruction)
}

pub fn read_operands(def: &Definition, ins: &[u8]) -> (Vec<usize>, usize) {
    let mut operands = Vec::with_capacity(def.operand_width.len());
    let mut offset = 0;

    for width in &def.operand_width {
        match width {
            2 => {
                let i = ins[offset..offset + 2].try_into().unwrap();
                operands.push(u16::from_be_bytes(i) as usize);
            }
            _ => {}
        }
        offset += width
    }

    (operands, offset)
}

pub fn read_uint16(ins: &Instructions, start: usize) -> u16 {
    u16::from_be_bytes(ins.0[start..start + 2].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make() {
        let tests = vec![
            (
                Opcode::OpConstant,
                vec![65534],
                Instructions(vec![0u8, 255u8, 254u8]),
            ),
            (Opcode::OpAdd, vec![], Instructions(vec![1u8])),
        ];

        for (op, operands, expected) in tests {
            let instruction = make(op, &operands);

            assert_eq!(instruction, expected);
        }
    }

    #[test]
    fn test_instruction_string() {
        let instructions = vec![
            make(Opcode::OpAdd, &vec![]),
            make(Opcode::OpConstant, &vec![2]),
            make(Opcode::OpConstant, &vec![65535]),
        ];

        let expected = r"0000 OpAdd
0001 OpConstant 2
0004 OpConstant 65535
";
        let concatted = instructions.concat();

        assert_eq!(expected, format!("{:?}", concatted));
    }

    #[test]
    fn test_read_operands() {
        let tests = vec![(Opcode::OpConstant, vec![65535], 2)];

        for (op, operands, bytes_read) in tests {
            let def = lookup(&op);
            let instruction = make(op, &operands);

            if let Some((_, ins)) = instruction.0.split_first() {
                let (operands_read, n) = read_operands(&def, ins);
                assert_eq!(bytes_read, n);
                assert_eq!(operands, operands_read);
            } else {
                unreachable!()
            }
        }
    }
}
