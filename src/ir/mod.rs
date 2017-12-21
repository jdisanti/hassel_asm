use hassel_lib6502::{OpAddressMode, OpCode, OpParam};
use std::sync::Arc;

use src_tag::SrcTag;

pub(crate) mod gen;
pub(crate) mod map;

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
            Resolved(mode, _)
            | Unresolved(mode, _, _)
            | UnresolvedLowByte(mode, _, _)
            | UnresolvedHighByte(mode, _, _) => mode,
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
}

#[derive(Debug, new)]
pub struct IROp {
    pub tag: SrcTag,
    pub code: &'static OpCode,
    pub param: IRParam,
    pub position: u16,
}

#[derive(Debug)]
pub enum IRChunk {
    Op(IROp),
    Bytes(Vec<u8>),
    Vector(SrcTag, Arc<String>, u16),
}

impl IRChunk {
    fn len(&self) -> usize {
        match *self {
            IRChunk::Op(ref op) => op.code.len as usize,
            IRChunk::Bytes(ref bytes) => bytes.len(),
            IRChunk::Vector(_, _, _) => 2,
        }
    }
}

#[derive(Debug)]
pub struct IRBlock {
    pub position: Option<u16>,
    pub label: Option<Arc<String>>,
    pub chunks: Vec<IRChunk>,
    pub length: u16,
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

    fn add_vector(&mut self, tag: SrcTag, label: &Arc<String>) {
        self.chunks
            .push(IRChunk::Vector(tag, Arc::clone(&label), 0));
    }
}

#[derive(Debug, new)]
pub struct IR {
    pub blocks: Vec<IRBlock>,
}
