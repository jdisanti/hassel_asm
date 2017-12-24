use ir::{IRBlock, IRChunk, IROp, IR};
use src_unit::SrcUnits;

#[derive(Serialize)]
pub struct SourceMapEntry {
    unit: usize,
    offset: usize,
    line: usize,
    address: u16,
}

#[derive(Serialize)]
pub struct SourceMap<'a> {
    src_units: &'a SrcUnits,
    entries: Vec<SourceMapEntry>,
}

impl<'a> SourceMap<'a> {
    pub fn new(src_units: &'a SrcUnits, ir: &IR) -> SourceMap<'a> {
        let mut map = SourceMap {
            src_units: src_units,
            entries: Vec::new(),
        };

        ir.add_entries(src_units, &mut map);
        map
    }
}

trait AddEntries {
    fn add_entries(&self, src_units: &SrcUnits, source_map: &mut SourceMap);
}

impl AddEntries for IR {
    fn add_entries(&self, src_units: &SrcUnits, source_map: &mut SourceMap) {
        for block in &self.blocks {
            block.add_entries(src_units, source_map);
        }
    }
}

impl AddEntries for IRBlock {
    fn add_entries(&self, src_units: &SrcUnits, source_map: &mut SourceMap) {
        for chunk in &self.chunks {
            chunk.add_entries(src_units, source_map);
        }
    }
}

impl AddEntries for IRChunk {
    fn add_entries(&self, src_units: &SrcUnits, source_map: &mut SourceMap) {
        match *self {
            IRChunk::Op(ref op) => {
                op.add_entries(src_units, source_map);
            }
            _ => {}
        }
    }
}

impl AddEntries for IROp {
    fn add_entries(&self, src_units: &SrcUnits, source_map: &mut SourceMap) {
        source_map.entries.push(SourceMapEntry {
            unit: self.tag.unit,
            offset: self.tag.offset,
            line: (&src_units.unit(self.tag.unit).source[0..self.tag.offset])
                .chars()
                .fold(1, |acc, chr| if chr == '\n' { acc + 1 } else { acc }),
            address: self.position,
        });
    }
}
