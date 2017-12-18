use src_tag::SrcTag;

#[derive(Debug, new)]
pub struct SrcUnit {
    pub id: usize,
    pub name: String,
    pub source: String,
}

#[derive(Debug, Default)]
pub struct SrcUnits {
    units: Vec<SrcUnit>,
}

impl SrcUnits {
    pub fn new() -> SrcUnits {
        Default::default()
    }

    pub fn name(&self, unit_id: usize) -> &String {
        &self.units[unit_id].name
    }

    pub fn source(&self, unit_id: usize) -> &String {
        &self.units[unit_id].source
    }

    pub fn unit(&self, unit_id: usize) -> &SrcUnit {
        &self.units[unit_id]
    }

    pub fn line_comment(&self, tag: SrcTag) -> String {
        let (row, col) = tag.row_col(&self.units[tag.unit].source);
        let line = tag.line(&self.units[tag.unit].source);
        format!("{}:{}:{}: {}", self.units[tag.unit].name, row, col, line)
    }

    pub fn push_unit(&mut self, name: String, source: String) -> usize {
        let unit_id = self.units.len();
        self.units.push(SrcUnit {
            id: unit_id,
            name: name,
            source: source,
        });
        unit_id
    }
}
