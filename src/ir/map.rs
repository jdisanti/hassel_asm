use std::collections::BTreeMap;

use ir::{IRBlock, IRChunk, IROp, IR};
use src_tag::SrcTag;
use src_unit::SrcUnits;

#[derive(Serialize)]
pub struct SourceMap<'a> {
    src_units: &'a SrcUnits,
    pc_to_src: BTreeMap<u16, SrcTag>,
}

impl<'a> SourceMap<'a> {
    pub fn new(src_units: &'a SrcUnits, ir: &IR) -> SourceMap<'a> {
        let mut map = SourceMap {
            src_units: src_units,
            pc_to_src: BTreeMap::new(),
        };

        ir.add_entries(&mut map);
        map
    }
}

trait AddEntries {
    fn add_entries(&self, source_map: &mut SourceMap);
}

impl AddEntries for IR {
    fn add_entries(&self, source_map: &mut SourceMap) {
        for block in &self.blocks {
            block.add_entries(source_map);
        }
    }
}

impl AddEntries for IRBlock {
    fn add_entries(&self, source_map: &mut SourceMap) {
        for chunk in &self.chunks {
            chunk.add_entries(source_map);
        }
    }
}

impl AddEntries for IRChunk {
    fn add_entries(&self, source_map: &mut SourceMap) {
        match *self {
            IRChunk::Op(ref op) => {
                op.add_entries(source_map);
            }
            _ => {}
        }
    }
}

impl AddEntries for IROp {
    fn add_entries(&self, source_map: &mut SourceMap) {
        source_map.pc_to_src.insert(self.position, self.tag);
    }
}
