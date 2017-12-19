use ast;
use ir;
use ir::gen::AppendBytes;
use error;
use src_unit::SrcUnits;

#[derive(Debug, new)]
pub struct AssemblerOutput {
    pub ast: Option<Vec<ast::Statement>>,
    pub ir: Option<ir::IR>,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Default)]
pub struct Assembler {
    src_units: SrcUnits,
    units: Vec<ast::Statement>,
}

impl Assembler {
    pub fn new() -> Assembler {
        Assembler::default()
    }

    pub fn parse_unit(&mut self, unit_name: &str, unit: &str) -> error::Result<()> {
        let unit_id = self.src_units.push_unit(unit_name.into(), unit.into());
        let statements = ast::Statement::parse(self.src_units.unit(unit_id))?;
        self.units.extend(statements.into_iter());
        Ok(())
    }

    pub fn assemble(self) -> error::Result<AssemblerOutput> {
        let mut output = AssemblerOutput {
            ast: Some(self.units),
            ir: None,
            bytes: None,
        };

        output.ir = Some(Assembler::translate_error(
            &self.src_units,
            ir::gen::IRGenerator::generate(
                output.ast.as_ref().unwrap(),
            ),
        )?);

        output.bytes = Some(Assembler::convert_to_bytes(output.ir.as_ref().unwrap()));
        Ok(output)
    }

    fn translate_error<T>(src_units: &SrcUnits, result: error::Result<T>) -> error::Result<T> {
        match result {
            Ok(_) => result,
            Err(err) => {
                Err(
                    error::ErrorKind::SrcUnitError(error::format_error(src_units, &err)).into(),
                )
            }
        }
    }

    fn convert_to_bytes(ir: &ir::IR) -> Vec<u8> {
        if ir.blocks.is_empty() {
            return Vec::new();
        }

        let start_pos = ir.blocks[0].position.unwrap() as usize;

        let mut bytes = Vec::new();
        for block in &ir.blocks {
            let current_pos = start_pos + bytes.len();
            assert!(current_pos <= 0xFFFF && current_pos + block.length as usize <= 0xFFFF);
            if block.position.unwrap() as usize > current_pos {
                bytes.resize(block.position.unwrap() as usize - start_pos, 0);
            }
            block.append_bytes(&mut bytes);
        }
        bytes
    }
}
