use hassel_lib6502::{Op, OpAddressMode, OpClass, OpCode, OpParam};
use std::sync::Arc;

use ast;
use error;
use error::ErrorKind::AssemblerError;

#[derive(Debug)]
pub enum IRParam {
    Resolved(OpParam),
    Unresolved(Arc<String>),
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
                Label(tag, ref label) => {
                    builder.new_block(None, Some(Arc::clone(label)));
                }
                ImpliedInstruction(tag, ref name) => {
                    if let Some(op_class) = OpClass::from_name(&*name) {
                        if let Some(op_code) = OpCode::find_by_class_and_mode(op_class, OpAddressMode::Implied) {
                            builder.current_block().add_op(
                                IROp::new(op_code, IRParam::Resolved(OpParam::None)),
                            );
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
                OperandInstruction(tag, ref name, ref operand) => unimplemented!(),
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
