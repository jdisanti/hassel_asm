//
// Copyright 2017 hassel_asm Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use hassel_lib6502::{OpAddressMode, OpClass, OpCode, OpParam};
use std::sync::Arc;
use std::collections::HashMap;

use ast;
use error;
use error::ErrorKind::AssemblerError;
use ir::{IRBlock, IRChunk, IROp, IRParam, IR};
use src_tag::SrcTag;

pub trait AppendBytes {
    fn append_bytes(&self, bytes: &mut Vec<u8>);
}

impl AppendBytes for IRBlock {
    fn append_bytes(&self, bytes: &mut Vec<u8>) {
        for chunk in &self.chunks {
            chunk.append_bytes(bytes);
        }
    }
}

impl AppendBytes for IRChunk {
    fn append_bytes(&self, bytes: &mut Vec<u8>) {
        match *self {
            IRChunk::Op(ref op) => op.append_bytes(bytes),
            IRChunk::Bytes(ref val) => bytes.extend(val),
            IRChunk::Vector(_, _, val) => {
                bytes.push(val as u8);
                bytes.push((val >> 8) as u8);
            }
        }
    }
}

impl AppendBytes for IROp {
    fn append_bytes(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.code.value);
        if let IRParam::Resolved(_, param) = self.param {
            match param {
                OpParam::None => {}
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

trait ResolveLength {
    fn resolve_length(&mut self) -> error::Result<()>;
}

impl ResolveLength for IRBlock {
    fn resolve_length(&mut self) -> error::Result<()> {
        let length = self.chunks
            .iter()
            .fold(0usize, |acc, chunk| acc + chunk.len());
        if length > 0xFFFF {
            Err(error::ErrorKind::SrcUnitError("assembly won't fit in 65535 bytes".into()).into())
        } else {
            self.length = length as u16;
            Ok(())
        }
    }
}

trait ResolveParameters {
    fn resolve_parameters(&mut self, position: u16, lookup_table: &HashMap<Arc<String>, u16>) -> error::Result<()>;
}

impl ResolveParameters for IRBlock {
    fn resolve_parameters(
        &mut self,
        block_position: u16,
        lookup_table: &HashMap<Arc<String>, u16>,
    ) -> error::Result<()> {
        let mut position = block_position;
        for chunk in &mut self.chunks {
            chunk.resolve_parameters(position, lookup_table)?;
            position = position.wrapping_add(chunk.len() as u16);
        }
        Ok(())
    }
}

impl ResolveParameters for IRChunk {
    fn resolve_parameters(
        &mut self,
        chunk_position: u16,
        lookup_table: &HashMap<Arc<String>, u16>,
    ) -> error::Result<()> {
        match *self {
            IRChunk::Op(ref mut op) => op.resolve_parameters(chunk_position, lookup_table),
            IRChunk::Vector(tag, ref label, ref mut value) => {
                if let Some(position) = lookup_table.get(label) {
                    *value = *position;
                    Ok(())
                } else {
                    Err(AssemblerError(tag, format!("unknown label: \"{}\"", label)).into())
                }
            }
            _ => Ok(()),
        }
    }
}

impl ResolveParameters for IROp {
    fn resolve_parameters(&mut self, op_position: u16, lookup_table: &HashMap<Arc<String>, u16>) -> error::Result<()> {
        self.position = op_position;
        self.param.resolve_parameters(op_position, lookup_table)?;
        assert!(self.param.len() == Some(self.code.len - 1));
        Ok(())
    }
}

impl ResolveParameters for IRParam {
    fn resolve_parameters(&mut self, op_position: u16, lookup_table: &HashMap<Arc<String>, u16>) -> error::Result<()> {
        let lookup = &|tag: SrcTag, name: &Arc<String>| {
            if let Some(position) = lookup_table.get(name) {
                Ok(*position)
            } else {
                Err(AssemblerError(tag, format!("unknown label: \"{}\"", name)))
            }
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

pub struct IRGenerator {}

impl IRGenerator {
    pub fn generate(units: &[ast::Statement]) -> error::Result<IR> {
        let mut ir = IRGenerator::generate_ir(units)?;
        IRGenerator::resolve(&mut ir)?;
        Ok(ir)
    }

    fn resolve(ir: &mut IR) -> error::Result<()> {
        for block in &mut ir.blocks {
            block.resolve_length()?;
        }

        let mut lookup_table: HashMap<Arc<String>, u16> = HashMap::new();
        let mut position = 0u16;
        for block in &mut ir.blocks {
            if let Some(pos) = block.position {
                position = pos;
            } else {
                block.position = Some(position);
            }

            if let Some(ref label) = block.label {
                lookup_table.insert(Arc::clone(label), position);
            }
            position = position.wrapping_add(block.length);
        }

        for block in &mut ir.blocks {
            position = block.position.unwrap();
            block.resolve_parameters(position, &lookup_table)?;
        }

        Ok(())
    }

    fn generate_ir(units: &[ast::Statement]) -> error::Result<IR> {
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
                        let mut param = IRGenerator::resolve_operand(operand)?;
                        if op_class.is_branch() && !op_class.is_jump() {
                            param = param.with_mode(OpAddressMode::PCOffset);
                        }
                        if let Some(op_code) = OpCode::find_by_class_and_mode(op_class, param.mode()) {
                            builder
                                .current_block()
                                .add_op(IROp::new(tag, op_code, param, 0));
                        } else {
                            return Err(AssemblerError(tag, format!("op {} requires a parameter", name)).into());
                        }
                    } else {
                        return Err(AssemblerError(tag, format!("unknown opcode: {}", name)).into());
                    }
                }
                MetaInstruction(ref meta_inst) => match *meta_inst {
                    ast::MetaInstruction::Org(tag, number) => {
                        if let ast::Number::Word(location) = number {
                            builder.new_block(Some(location), None);
                        } else {
                            return Err(AssemblerError(tag, "org must be a 16-bit address".into()).into());
                        }
                    }
                    ast::MetaInstruction::Pad(tag, number) => {
                        if let ast::Number::Word(location) = number {
                            builder.new_block(Some(location), None);
                        } else {
                            return Err(AssemblerError(tag, "pad requires a 16-bit address".into()).into());
                        }
                    }
                    ast::MetaInstruction::Byte(tag, ref numbers) => {
                        let mut bytes = Vec::new();
                        for num in numbers {
                            if let ast::Number::Byte(byte) = *num {
                                bytes.push(byte);
                            } else {
                                return Err(AssemblerError(tag, "byte constants must be in range".into()).into());
                            }
                        }
                        builder.current_block().add_bytes(bytes);
                    }
                    ast::MetaInstruction::Word(tag, ref numbers) => {
                        let mut bytes = Vec::new();
                        for num in numbers {
                            if let ast::Number::Byte(byte) = *num {
                                bytes.push(byte);
                                bytes.push(0);
                            } else if let ast::Number::Word(word) = *num {
                                bytes.push(word as u8);
                                bytes.push((word >> 8) as u8);
                            } else {
                                return Err(AssemblerError(tag, "word constants must be in range".into()).into());
                            }
                        }
                        builder.current_block().add_bytes(bytes);
                    }
                    ast::MetaInstruction::Vector(tag, ref label) => {
                        builder.current_block().add_vector(tag, label);
                    }
                    ast::MetaInstruction::Include(_, _) => {}
                },
            }
        }
        Ok(builder.build())
    }

    fn resolve_operand(operand: &ast::Operand) -> error::Result<IRParam> {
        use ast::Operand::*;
        match *operand {
            None => Ok(IRParam::Resolved(OpAddressMode::Implied, OpParam::None)),
            Immediate(modifier, ref term) => IRGenerator::resolve_param(term, modifier, OpAddressMode::Immediate),
            Address(modifier, ref term) => IRGenerator::resolve_param(term, modifier, OpAddressMode::Absolute),
            AbsoluteX(ref term) => IRGenerator::resolve_param(
                term,
                ast::OperandModifier::None,
                OpAddressMode::AbsoluteOffsetX,
            ),
            AbsoluteY(ref term) => IRGenerator::resolve_param(
                term,
                ast::OperandModifier::None,
                OpAddressMode::AbsoluteOffsetY,
            ),
            Indirect(ref term) => IRGenerator::resolve_param(term, ast::OperandModifier::None, OpAddressMode::Indirect),
            IndirectX(ref term) => IRGenerator::resolve_param(
                term,
                ast::OperandModifier::None,
                OpAddressMode::PreIndirectX,
            ),
            IndirectY(ref term) => IRGenerator::resolve_param(
                term,
                ast::OperandModifier::None,
                OpAddressMode::PostIndirectY,
            ),
        }
    }

    fn resolve_param(term: &ast::Term, modifier: ast::OperandModifier, mode: OpAddressMode) -> error::Result<IRParam> {
        match *term {
            ast::Term::Number(tag, ref num) => IRGenerator::num_to_param(tag, num, modifier, mode),
            ast::Term::Name(tag, ref name) => IRGenerator::name_to_param(tag, name, modifier, mode),
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
                    _ => Err(AssemblerError(tag, "can't take high/low byte of a single byte".into()).into()),
                }
            }
            ast::Number::Word(val) => match modifier {
                ast::OperandModifier::None => Ok(IRParam::Resolved(mode, OpParam::Word(val))),
                ast::OperandModifier::HighByte => Ok(IRParam::Resolved(mode, OpParam::Byte((val >> 8) as u8))),
                ast::OperandModifier::LowByte => Ok(IRParam::Resolved(mode, OpParam::Byte(val as u8))),
            },
            ast::Number::Invalid(_val) => {
                Err(AssemblerError(tag, "number not within 8-bit or 16-bit bounds".into()).into())
            }
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
