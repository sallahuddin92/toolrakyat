use crate::content::operand::ContentOperand;
use crate::content::operator::{ContentInstruction, ContentOperator};
use crate::error::PdfResult;
use crate::syntax::lexer::Lexer;
use crate::syntax::token::Token;

pub struct ContentParser<'a> {
    lexer: Lexer<'a>,
}

impl<'a> ContentParser<'a> {
    pub const fn new(lexer: Lexer<'a>) -> Self {
        Self { lexer }
    }

    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::from_bytes(bytes),
        }
    }

    pub fn parse_instructions(&mut self) -> PdfResult<Vec<ContentInstruction>> {
        let mut instructions = Vec::new();
        let mut current_operands = Vec::new();

        while let Some(token) = self.lexer.next_token()? {
            match token {
                Token::Integer(i) => current_operands.push(ContentOperand::Integer(i)),
                Token::Real(r) => current_operands.push(ContentOperand::Real(r)),
                Token::Name(n) => current_operands.push(ContentOperand::Name(n)),
                Token::LiteralString(s) => current_operands.push(ContentOperand::String(s)),
                Token::HexString(s) => current_operands.push(ContentOperand::String(s)),
                Token::ArrayOpen => {
                    let arr = self.parse_array_operand()?;
                    current_operands.push(ContentOperand::Array(arr));
                }
                Token::Keyword(kw) => {
                    let op = ContentOperator::from_keyword(&kw);
                    let operands = std::mem::take(&mut current_operands);
                    instructions.push(ContentInstruction::new(operands, op));
                }
                Token::KeywordObj
                | Token::KeywordEndObj
                | Token::KeywordStream
                | Token::KeywordEndStream
                | Token::KeywordXref
                | Token::KeywordTrailer
                | Token::KeywordStartXref
                | Token::KeywordR => {
                    // In content stream, treated as raw keywords
                    let kw_str = match token {
                        Token::KeywordR => "R",
                        Token::KeywordObj => "obj",
                        Token::KeywordEndObj => "endobj",
                        Token::KeywordStream => "stream",
                        Token::KeywordEndStream => "endstream",
                        Token::KeywordXref => "xref",
                        Token::KeywordTrailer => "trailer",
                        Token::KeywordStartXref => "startxref",
                        _ => "unknown",
                    };
                    let op = ContentOperator::from_keyword(kw_str);
                    let operands = std::mem::take(&mut current_operands);
                    instructions.push(ContentInstruction::new(operands, op));
                }
                Token::Comment(_)
                | Token::Null
                | Token::Boolean(_)
                | Token::ArrayClose
                | Token::DictOpen
                | Token::DictClose => {
                    // Ignored or skipped in content stream operator operand position
                }
            }
        }

        Ok(instructions)
    }

    fn parse_array_operand(&mut self) -> PdfResult<Vec<ContentOperand>> {
        let mut items = Vec::new();

        while let Some(tok) = self.lexer.next_token()? {
            match tok {
                Token::ArrayClose => break,
                Token::Integer(i) => items.push(ContentOperand::Integer(i)),
                Token::Real(r) => items.push(ContentOperand::Real(r)),
                Token::Name(n) => items.push(ContentOperand::Name(n)),
                Token::LiteralString(s) => items.push(ContentOperand::String(s)),
                Token::HexString(s) => items.push(ContentOperand::String(s)),
                Token::ArrayOpen => {
                    let nested = self.parse_array_operand()?;
                    items.push(ContentOperand::Array(nested));
                }
                _ => {}
            }
        }

        Ok(items)
    }
}
