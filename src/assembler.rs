use ast;
use ir;
use error;
use src_unit::SrcUnits;

#[derive(Debug, new)]
pub struct AssemblerOutput {
    pub ast: Option<Vec<ast::Statement>>,
    pub ir: Option<ir::IR>,
    pub bytes: Option<Vec<u8>>,
}

pub struct Assembler {
    src_units: SrcUnits,
    units: Vec<ast::Statement>,
}

impl Assembler {
    pub fn new() -> Assembler {
        Assembler {
            src_units: SrcUnits::new(),
            units: Vec::new(),
        }
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
            ir::IR::generate(output.ast.as_ref().unwrap()),
        )?);

        Assembler::translate_error(&self.src_units, output.ir.as_mut().unwrap().resolve())?;

        output.bytes = Some(output.ir.as_ref().unwrap().to_bytes());

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
}
