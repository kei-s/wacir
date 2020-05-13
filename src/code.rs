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
            let def = lookup(&Opcode(self.0[i]));
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
    fn fmt_instruction(&self, def: Definition, operands: Vec<u16>) -> String {
        let operand_count = def.operand_width.len();

        if operands.len() != operand_count {
            return format!(
                "ERROR: operand len {} does not match defined {}\n",
                operands.len(),
                operand_count
            );
        }

        match operand_count {
            1 => format!("{} {}", def.name, operands[0]),
            _ => format!("ERROR: unhundled operand_count for {}", def.name),
        }
    }
}

#[derive(Debug)]
pub struct Opcode(pub u8);

impl Opcode {
    pub fn t(&self) -> OpcodeType {
        match self.0 {
            0 => OpcodeType::OpConstant,
            _ => unreachable!("No such opcode {:?}", self),
        }
    }
}

pub enum OpcodeType {
    OpConstant,
}

impl OpcodeType {
    pub fn opcode(&self) -> Opcode {
        Opcode(match self {
            OpcodeType::OpConstant => 0,
        })
    }
}

pub struct Definition {
    pub name: String,
    pub operand_width: Vec<usize>,
}

pub fn lookup(op: &Opcode) -> Definition {
    match op.t() {
        OpcodeType::OpConstant => Definition {
            name: "OpConstant".to_string(),
            operand_width: vec![2],
        },
    }
}

pub fn make(op: &Opcode, operand: u16) -> Instructions {
    let def = lookup(op);

    let mut instruction_len = 1;
    for w in &def.operand_width {
        instruction_len += w;
    }

    let mut instruction: Vec<u8> = Vec::with_capacity(instruction_len);
    instruction.push(op.0);

    let bytes = operand.to_be_bytes();
    instruction.extend_from_slice(&bytes);

    Instructions(instruction)
}

pub fn read_operands(def: &Definition, ins: &[u8]) -> (Vec<u16>, usize) {
    let mut operands = Vec::with_capacity(def.operand_width.len());
    let mut offset = 0;

    for width in &def.operand_width {
        match width {
            2 => {
                let i = ins[offset..offset + 2].try_into().unwrap();
                operands.push(u16::from_be_bytes(i));
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
        let tests = vec![(
            OpcodeType::OpConstant.opcode(),
            65534,
            Instructions(vec![0u8, 255u8, 254u8]),
        )];

        for (op, operand, expected) in tests {
            let instruction = make(&op, operand);

            assert_eq!(instruction, expected);
        }
    }

    #[test]
    fn test_instruction_string() {
        let instructions = vec![
            make(&OpcodeType::OpConstant.opcode(), 1),
            make(&OpcodeType::OpConstant.opcode(), 2),
            make(&OpcodeType::OpConstant.opcode(), 65535),
        ];

        let expected = r"0000 OpConstant 1
0003 OpConstant 2
0006 OpConstant 65535
";
        let concatted = instructions.concat();

        assert_eq!(expected, format!("{:?}", concatted));
    }

    #[test]
    fn test_read_operands() {
        let tests = vec![(OpcodeType::OpConstant, 65535, 2)];

        for (op, operand, bytes_read) in tests {
            let instruction = make(&op.opcode(), operand);
            let def = lookup(&op.opcode());

            if let Some((_, ins)) = instruction.0.split_first() {
                let (operands_read, n) = read_operands(&def, ins);
                assert_eq!(bytes_read, n);
                assert_eq!(operand, operands_read[0]);
            } else {
                unreachable!()
            }
        }
    }
}
