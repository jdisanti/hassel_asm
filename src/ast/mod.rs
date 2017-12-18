use std::sync::Arc;
use lalrpop_util;

use error;
use src_tag::SrcTag;
use src_unit::SrcUnit;

mod grammar;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Number {
    Byte(u8),
    Word(u16),
    Invalid(usize),
}

#[derive(Debug)]
pub enum Term {
    Number(SrcTag, Number),
    Name(SrcTag, Arc<String>),
}

#[derive(Debug)]
pub enum Operand {
    Immediate(Box<Operand>),
    HighByte(Term),
    LowByte(Term),
    AbsoluteX(Term),
    AbsoluteY(Term),
    IndirectX(Term),
    IndirectY(Term),
    Term(Term),
}

#[derive(Debug)]
pub enum MetaInstruction {
    Org(SrcTag, Number),
    Byte(SrcTag, Vec<Number>),
}

#[derive(Debug)]
pub enum Statement {
    Comment,
    Label(SrcTag, Arc<String>),
    ImpliedInstruction(SrcTag, Arc<String>),
    OperandInstruction(SrcTag, Arc<String>, Operand),
    MetaInstruction(MetaInstruction),
}

impl Statement {
    pub fn parse<'a>(src_unit: &'a SrcUnit) -> error::Result<Vec<Statement>> {
        if src_unit.source == "" {
            return Ok(Vec::new());
        } else {
            let mut errors: Vec<lalrpop_util::ErrorRecovery<usize, (usize, &'a str), ()>> = Vec::new();
            let ast = grammar::parse_Program(src_unit.id, &mut errors, &src_unit.source);
            if errors.is_empty() {
                match ast {
                    Ok(expression) => Ok(expression),
                    Err(err) => Err(translate_errors(src_unit, [err].iter()).into()),
                }
            } else {
                Err(
                    translate_errors(src_unit, errors.iter().map(|err| &err.error)).into(),
                )
            }
        }
    }
}

fn translate_errors<'a, I>(unit: &SrcUnit, errors: I) -> error::ErrorKind
where
    I: Iterator<Item = &'a lalrpop_util::ParseError<usize, (usize, &'a str), ()>>,
{
    let mut messages = Vec::new();
    for error in errors {
        match *error {
            lalrpop_util::ParseError::InvalidToken { location } => {
                let (row, col) = SrcTag::new(0, location).row_col(&unit.source);
                messages.push(format!("{}:{}:{}: invalid token", unit.name, row, col));
            }
            lalrpop_util::ParseError::UnrecognizedToken {
                ref token,
                ref expected,
            } => {
                match *token {
                    Some((start, token, _end)) => {
                        let (row, col) = SrcTag::new(0, start).row_col(&unit.source);
                        messages.push(format!(
                            "{}:{}:{}: unexpected token \"{}\". Expected one of: {:?}",
                            unit.name,
                            row,
                            col,
                            token.1,
                            expected
                        ));
                    }
                    None => {
                        messages.push(format!(
                            "{}: unexpected EOF; expected: {:?}",
                            unit.name,
                            expected
                        ));
                    }
                }
            }
            lalrpop_util::ParseError::ExtraToken { ref token } => {
                let (row, col) = SrcTag::new(0, token.0).row_col(&unit.source);
                messages.push(format!(
                    "{}:{}:{}: extra token \"{}\"",
                    unit.name,
                    row,
                    col,
                    (token.1).1
                ));
            }
            lalrpop_util::ParseError::User { ref error } => {
                messages.push(format!("{:?}", error));
            }
        }
    }
    error::ErrorKind::ParseError(messages)
}
