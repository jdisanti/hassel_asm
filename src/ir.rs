use hassel_lib6502::{OpAddressMode, OpClass, OpCode, OpParam};
use std::collections::HashMap;
use std::sync::Arc;

use ast;
use error;
use error::ErrorKind::AssemblerError;
use src_tag::SrcTag;

#[derive(Debug)]
pub enum IRParam {
    Resolved(OpAddressMode, OpParam),
    Unresolved(OpAddressMode, SrcTag, Arc<String>),
    UnresolvedLowByte(OpAddressMode, SrcTag, Arc<String>),
    UnresolvedHighByte(OpAddressMode, SrcTag, Arc<String>),
}

impl IRParam {
    pub fn mode(&self) -> OpAddressMode {
        use self::IRParam::*;
        match *self {
            Resolved(mode, _) => mode,
            Unresolved(mode, _, _) => mode,
            UnresolvedLowByte(mode, _, _) => mode,
            UnresolvedHighByte(mode, _, _) => mode,
        }
    }

    pub fn with_mode(self, mode: OpAddressMode) -> IRParam {
        use self::IRParam::*;
        match self {
            Resolved(_, param) => Resolved(mode, param),
            Unresolved(_, tag, name) => Unresolved(mode, tag, name),
            UnresolvedLowByte(_, tag, name) => UnresolvedLowByte(mode, tag, name),
            UnresolvedHighByte(_, tag, name) => UnresolvedHighByte(mode, tag, name),
        }
    }

    pub fn len(&self) -> Option<u8> {
        match *self {
            IRParam::Resolved(_, param) => Some(param.len()),
            _ => None,
        }
    }

    fn resolve_op_params(&mut self, op_position: u16, lookup_table: &HashMap<Arc<String>, u16>) -> error::Result<()> {
        let lookup = &|tag: SrcTag, name: &Arc<String>| if let Some(position) = lookup_table.get(name) {
            Ok(*position)
        } else {
            Err(AssemblerError(tag, format!("unknown label: \"{}\"", name)))
        };

        let replacement = match *self {
            IRParam::Resolved(mode, param) => IRParam::Resolved(mode, param),
            IRParam::Unresolved(mode, tag, ref name) => {
                let position = lookup(tag, name)?;
                // Absolute addresses must be converted to offsets for branch instructions
                if mode == OpAddressMode::PCOffset {
                    let pc = op_position.wrapping_add(2);
                    let pc_offset = ((position as isize) - (pc as isize)) as i16;
                    if pc_offset > 127 || pc_offset < -128 {
                        // TODO: Refactor so that this code modification is possible
                        let msg = "modifying code to fix branch offsets outside \
                            of range -128 to +127 is not currently supported";
                        return Err(AssemblerError(tag, msg.into()).into());
                    } else {
                        IRParam::Resolved(mode, OpParam::Byte(pc_offset as u8))
                    }
                } else {
                    IRParam::Resolved(mode, OpParam::Word(lookup(tag, name)?))
                }
            }
            IRParam::UnresolvedLowByte(mode, tag, ref name) => {
                IRParam::Resolved(mode, OpParam::Byte(lookup(tag, name)? as u8))
            }
            IRParam::UnresolvedHighByte(mode, tag, ref name) => {
                IRParam::Resolved(mode, OpParam::Byte((lookup(tag, name)? >> 8) as u8))
            }
        };

        *self = replacement;
        Ok(())
    }
}

#[derive(Debug, new)]
pub struct IROp {
    code: &'static OpCode,
    param: IRParam,
}

impl IROp {
    fn resolve_op_params(&mut self, op_position: u16, lookup_table: &HashMap<Arc<String>, u16>) -> error::Result<()> {
        self.param.resolve_op_params(op_position, lookup_table)?;
        assert!(self.param.len() == Some(self.code.len - 1));
        Ok(())
    }

    fn append_bytes(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.code.value);
        if let IRParam::Resolved(_, param) = self.param {
            match param {
                OpParam::None => {},
                OpParam::Byte(val) => bytes.push(val),
                OpParam::Word(_) => {
                    bytes.push(param.low_byte());
                    bytes.push(param.high_byte());
                }
            }
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug)]
pub enum IRChunk {
    Op(IROp),
    Bytes(Vec<u8>),
}

impl IRChunk {
    fn len(&self) -> usize {
        match *self {
            IRChunk::Op(ref op) => op.code.len as usize,
            IRChunk::Bytes(ref bytes) => bytes.len(),
        }
    }

    fn resolve_op_params(
        &mut self,
        chunk_position: u16,
        lookup_table: &HashMap<Arc<String>, u16>,
    ) -> error::Result<()> {
        match *self {
            IRChunk::Op(ref mut op) => op.resolve_op_params(chunk_position, lookup_table),
            _ => Ok(()),
        }
    }

    fn append_bytes(&self, bytes: &mut Vec<u8>) {
        match *self {
            IRChunk::Op(ref op) => op.append_bytes(bytes),
            IRChunk::Bytes(ref val) => bytes.extend(val),
        }
    }
}

#[derive(Debug)]
pub struct IRBlock {
    position: Option<u16>,
    label: Option<Arc<String>>,
    chunks: Vec<IRChunk>,
    length: u16,
}

impl IRBlock {
    fn new(position: Option<u16>, label: Option<Arc<String>>) -> IRBlock {
        IRBlock {
            position: position,
            label: label,
            chunks: Vec::new(),
            length: 0,
        }
    }

    fn add_op(&mut self, op: IROp) {
        self.chunks.push(IRChunk::Op(op));
    }

    fn add_bytes(&mut self, bytes: Vec<u8>) {
        self.chunks.push(IRChunk::Bytes(bytes));
    }

    fn resolve_op_params(
        &mut self,
        block_position: u16,
        lookup_table: &HashMap<Arc<String>, u16>,
    ) -> error::Result<()> {
        let mut position = block_position;
        for chunk in &mut self.chunks {
            chunk.resolve_op_params(position, lookup_table)?;
            position += chunk.len() as u16;
        }
        Ok(())
    }

    fn resolve_length(&mut self) -> error::Result<()> {
        let length = self.chunks.iter().fold(0usize, |acc, chunk| acc + chunk.len());
        if length > 0xFFFF {
            Err(
                error::ErrorKind::SrcUnitError("assembly won't fit in 65535 bytes".into()).into(),
            )
        } else {
            self.length = length as u16;
            Ok(())
        }
    }

    fn append_bytes(&self, bytes: &mut Vec<u8>) {
        for chunk in &self.chunks {
            chunk.append_bytes(bytes);
        }
    }
}

#[derive(Debug, new)]
pub struct IR {
    blocks: Vec<IRBlock>,
}

impl IR {
    pub fn to_bytes(&self) -> Vec<u8> {
        if self.blocks.is_empty() {
            return Vec::new();
        }

        let start_pos = self.blocks[0].position.unwrap() as usize;

        let mut bytes = Vec::new();
        for block in &self.blocks {
            let current_pos = start_pos + bytes.len();
            assert!(current_pos <= 0xFFFF && current_pos + block.length as usize <= 0xFFFF);
            if block.position.unwrap() as usize > current_pos {
                bytes.resize(block.position.unwrap() as usize - start_pos, 0);
            }
            block.append_bytes(&mut bytes);
        }
        bytes
    }

    pub fn resolve(&mut self) -> error::Result<()> {
        for block in &mut self.blocks {
            block.resolve_length()?;
        }

        let mut lookup_table: HashMap<Arc<String>, u16> = HashMap::new();
        let mut position = 0u16;
        for block in &mut self.blocks {
            if let Some(pos) = block.position {
                position = pos;
            } else {
                block.position = Some(position);
            }
            if let Some(ref label) = block.label {
                lookup_table.insert(Arc::clone(label), position);
            }
            position += block.length;
        }

        for block in &mut self.blocks {
            position = block.position.unwrap();
            block.resolve_op_params(position, &lookup_table)?;
        }

        Ok(())
    }

    pub fn generate(units: &Vec<ast::Statement>) -> error::Result<IR> {
        let mut builder = IRBuilder::new();
        for statement in units {
            use ast::Statement::*;
            match *statement {
                Comment => {}
                Label(_tag, ref label) => {
                    builder.new_block(None, Some(Arc::clone(label)));
                }
                Instruction(tag, ref name, ref operand) => {
                    if let Some(op_class) = OpClass::from_name(&*name) {
                        let mut param = IR::resolve_operand(operand)?;
                        if op_class.is_branch() && !op_class.is_jump() {
                            param = param.with_mode(OpAddressMode::PCOffset);
                        }
                        if let Some(op_code) = OpCode::find_by_class_and_mode(op_class, param.mode()) {
                            builder.current_block().add_op(IROp::new(op_code, param));
                        } else {
                            return Err(
                                AssemblerError(tag, format!("op {} requires a parameter", name)).into(),
                            );
                        }
                    } else {
                        return Err(
                            AssemblerError(tag, format!("unknown opcode: {}", name)).into(),
                        );
                    }
                }
                MetaInstruction(ref meta_inst) => {
                    match *meta_inst {
                        ast::MetaInstruction::Org(tag, number) => {
                            if let ast::Number::Word(location) = number {
                                builder.new_block(Some(location), None);
                            } else {
                                return Err(
                                    AssemblerError(tag, "org must be a 16-bit number".into()).into(),
                                );
                            }
                        }
                        ast::MetaInstruction::Byte(tag, ref numbers) => {
                            let mut bytes = Vec::new();
                            for num in numbers {
                                if let ast::Number::Byte(byte) = *num {
                                    bytes.push(byte);
                                } else {
                                    return Err(
                                        AssemblerError(tag, "byte constants must be in range".into()).into(),
                                    );
                                }
                            }
                            builder.current_block().add_bytes(bytes);
                        }
                    }
                }
            }
        }
        Ok(builder.build())
    }

    fn resolve_operand(operand: &ast::Operand) -> error::Result<IRParam> {
        use ast::Operand::*;
        match *operand {
            None => Ok(IRParam::Resolved(OpAddressMode::Implied, OpParam::None)),
            Immediate(modifier, ref term) => IR::resolve_param(term, modifier, OpAddressMode::Immediate),
            Address(modifier, ref term) => IR::resolve_param(term, modifier, OpAddressMode::Absolute),
            AbsoluteX(ref term) => {
                IR::resolve_param(
                    term,
                    ast::OperandModifier::None,
                    OpAddressMode::AbsoluteOffsetX,
                )
            }
            AbsoluteY(ref term) => {
                IR::resolve_param(
                    term,
                    ast::OperandModifier::None,
                    OpAddressMode::AbsoluteOffsetY,
                )
            }
            Indirect(ref term) => IR::resolve_param(term, ast::OperandModifier::None, OpAddressMode::Indirect),
            IndirectX(ref term) => {
                IR::resolve_param(
                    term,
                    ast::OperandModifier::None,
                    OpAddressMode::PreIndirectX,
                )
            }
            IndirectY(ref term) => {
                IR::resolve_param(
                    term,
                    ast::OperandModifier::None,
                    OpAddressMode::PostIndirectY,
                )
            }
        }
    }

    fn resolve_param(term: &ast::Term, modifier: ast::OperandModifier, mode: OpAddressMode) -> error::Result<IRParam> {
        match *term {
            ast::Term::Number(tag, ref num) => IR::num_to_param(tag, num, modifier, mode),
            ast::Term::Name(tag, ref name) => IR::name_to_param(tag, name, modifier, mode),
        }
    }

    fn name_to_param(
        tag: SrcTag,
        name: &Arc<String>,
        modifier: ast::OperandModifier,
        mode: OpAddressMode,
    ) -> error::Result<IRParam> {
        match modifier {
            ast::OperandModifier::None => Ok(IRParam::Unresolved(mode, tag, Arc::clone(name))),
            ast::OperandModifier::HighByte => Ok(IRParam::UnresolvedHighByte(mode, tag, Arc::clone(name))),
            ast::OperandModifier::LowByte => Ok(IRParam::UnresolvedLowByte(mode, tag, Arc::clone(name))),
        }
    }

    fn num_to_param(
        tag: SrcTag,
        num: &ast::Number,
        modifier: ast::OperandModifier,
        mode: OpAddressMode,
    ) -> error::Result<IRParam> {
        match *num {
            ast::Number::Byte(val) => {
                // If we're only a byte wide, then we can take advantage of faster address modes
                let corrected_mode = match mode {
                    OpAddressMode::Absolute => OpAddressMode::ZeroPage,
                    OpAddressMode::AbsoluteOffsetX => OpAddressMode::ZeroPageOffsetX,
                    OpAddressMode::AbsoluteOffsetY => OpAddressMode::ZeroPageOffsetY,
                    _ => mode,
                };
                match modifier {
                    ast::OperandModifier::None => Ok(IRParam::Resolved(corrected_mode, OpParam::Byte(val))),
                    _ => Err(
                        AssemblerError(tag, "can't take high/low byte of a single byte".into()).into(),
                    ),
                }
            }
            ast::Number::Word(val) => {
                match modifier {
                    ast::OperandModifier::None => Ok(IRParam::Resolved(mode, OpParam::Word(val))),
                    ast::OperandModifier::HighByte => Ok(IRParam::Resolved(mode, OpParam::Byte((val >> 8) as u8))),
                    ast::OperandModifier::LowByte => Ok(IRParam::Resolved(mode, OpParam::Byte(val as u8))),
                }
            }
            ast::Number::Invalid(_val) => Err(
                AssemblerError(tag, "number not within 8-bit or 16-bit bounds".into()).into(),
            ),
        }
    }
}

struct IRBuilder {
    blocks: Vec<IRBlock>,
}

impl IRBuilder {
    pub fn new() -> IRBuilder {
        IRBuilder { blocks: Vec::new() }
    }

    pub fn new_block(&mut self, position: Option<u16>, label: Option<Arc<String>>) {
        self.blocks.push(IRBlock::new(position, label));
    }

    pub fn current_block(&mut self) -> &mut IRBlock {
        if self.blocks.is_empty() {
            self.new_block(None, None);
        }
        let cur = self.blocks.len() - 1;
        &mut self.blocks[cur]
    }

    pub fn build(self) -> IR {
        IR::new(self.blocks)
    }
}
