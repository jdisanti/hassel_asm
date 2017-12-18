use hassel_lib6502::{OpAddressMode, OpClass, OpCode, OpParam};
use std::sync::Arc;

use ast;
use error;
use error::ErrorKind::AssemblerError;
use src_tag::SrcTag;

#[derive(Debug)]
pub enum IRParam {
    Resolved(OpAddressMode, OpParam),
    Unresolved(OpAddressMode, Arc<String>),
    UnresolvedLowByte(OpAddressMode, Arc<String>),
    UnresolvedHighByte(OpAddressMode, Arc<String>),
}

impl IRParam {
    pub fn mode(&self) -> OpAddressMode {
        use self::IRParam::*;
        match *self {
            Resolved(mode, _) => mode,
            Unresolved(mode, _) => mode,
            UnresolvedLowByte(mode, _) => mode,
            UnresolvedHighByte(mode, _) => mode,
        }
    }

    pub fn with_mode(self, mode: OpAddressMode) -> IRParam {
        use self::IRParam::*;
        match self {
            Resolved(_, param) => Resolved(mode, param),
            Unresolved(_, name) => Unresolved(mode, name),
            UnresolvedLowByte(_, name) => Unresolved(mode, name),
            UnresolvedHighByte(_, name) => Unresolved(mode, name),
        }
    }
}

#[derive(Debug, new)]
pub struct IROp {
    code: &'static OpCode,
    param: IRParam,
}

#[derive(Debug)]
pub enum IRChunk {
    Op(IROp),
    Bytes(Vec<u8>),
}

#[derive(Debug)]
pub struct IRBlock {
    position: Option<u16>,
    label: Option<Arc<String>>,
    chunks: Vec<IRChunk>,
}

impl IRBlock {
    fn new(position: Option<u16>, label: Option<Arc<String>>) -> IRBlock {
        IRBlock {
            position: position,
            label: label,
            chunks: Vec::new(),
        }
    }

    fn add_op(&mut self, op: IROp) {
        self.chunks.push(IRChunk::Op(op));
    }

    fn add_bytes(&mut self, bytes: Vec<u8>) {
        self.chunks.push(IRChunk::Bytes(bytes));
    }
}

#[derive(Debug, new)]
pub struct IR {
    blocks: Vec<IRBlock>,
}

impl IR {
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
            ast::Term::Name(_tag, ref name) => IR::name_to_param(name, modifier, mode),
        }
    }

    fn name_to_param(
        name: &Arc<String>,
        modifier: ast::OperandModifier,
        mode: OpAddressMode,
    ) -> error::Result<IRParam> {
        match modifier {
            ast::OperandModifier::None => Ok(IRParam::Unresolved(mode, Arc::clone(name))),
            ast::OperandModifier::HighByte => Ok(IRParam::UnresolvedHighByte(mode, Arc::clone(name))),
            ast::OperandModifier::LowByte => Ok(IRParam::UnresolvedLowByte(mode, Arc::clone(name))),
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
