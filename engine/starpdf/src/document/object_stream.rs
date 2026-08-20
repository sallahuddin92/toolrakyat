use std::collections::BTreeMap;

use crate::error::{PdfError, PdfResult};
use crate::filter::flate::FlateDecoder;
use crate::filter::limits::DecompressLimits;
use crate::syntax::lexer::Lexer;
use crate::syntax::object::{PdfObject, StreamObject};
use crate::syntax::parser::Parser;
use crate::syntax::token::Token;

#[derive(Debug, Clone)]
pub struct DecodedObjectStream {
    pub n_objects: usize,
    pub first_offset: usize,
    pub index_map: BTreeMap<u32, (u64, usize)>, // index_in_stream -> (obj_num, offset_rel)
    pub obj_map: BTreeMap<u64, usize>,          // obj_num -> offset_rel
    pub data: Vec<u8>,
}

pub struct ObjectStreamReader;

impl ObjectStreamReader {
    pub fn decode_stream(
        stream: &StreamObject,
        limits: &DecompressLimits,
    ) -> PdfResult<DecodedObjectStream> {
        let dict = &stream.dict;

        let n = dict
            .get("N")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| PdfError::InvalidSyntax("Object stream missing /N".into()))?
            as usize;

        if n > limits.max_object_stream_objects {
            return Err(PdfError::InvalidSyntax(format!(
                "Object stream /N {n} exceeds security limit {}",
                limits.max_object_stream_objects
            )));
        }

        let first = dict
            .get("First")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| PdfError::InvalidSyntax("Object stream missing /First".into()))?
            as usize;

        // Decompress data
        let mut decoded_data = stream.data.clone();
        if let Some(filter) = dict.get("Filter").and_then(|v| v.as_name()) {
            if filter == "FlateDecode" {
                decoded_data = FlateDecoder::decode(&stream.data, limits)?;
            }
        }

        if first > decoded_data.len() {
            return Err(PdfError::InvalidSyntax(format!(
                "/First offset {first} exceeds decoded object stream length {}",
                decoded_data.len()
            )));
        }

        // Parse header: N pairs of (obj_num, offset_rel)
        let header_slice = &decoded_data[..first];
        let mut lexer = Lexer::from_bytes(header_slice);

        let mut index_map = BTreeMap::new();
        let mut obj_map = BTreeMap::new();

        for i in 0..n {
            let obj_num = match lexer.next_token()? {
                Some(Token::Integer(num)) if num >= 0 => num as u64,
                Some(other) => {
                    return Err(PdfError::InvalidSyntax(format!(
                        "Expected object number at index {i} in ObjStm, found {other:?}"
                    )))
                }
                None => return Err(PdfError::UnexpectedEof),
            };

            let offset_rel = match lexer.next_token()? {
                Some(Token::Integer(off)) if off >= 0 => off as usize,
                Some(other) => {
                    return Err(PdfError::InvalidSyntax(format!(
                        "Expected offset at index {i} in ObjStm for obj {obj_num}, found {other:?}"
                    )))
                }
                None => return Err(PdfError::UnexpectedEof),
            };

            index_map.insert(i as u32, (obj_num, offset_rel));
            obj_map.insert(obj_num, offset_rel);
        }

        Ok(DecodedObjectStream {
            n_objects: n,
            first_offset: first,
            index_map,
            obj_map,
            data: decoded_data,
        })
    }

    pub fn extract_object(
        decoded: &DecodedObjectStream,
        index_in_stream: u32,
    ) -> PdfResult<PdfObject> {
        let &(_obj_num, offset_rel) = decoded.index_map.get(&index_in_stream).ok_or_else(|| {
            PdfError::InvalidSyntax(format!(
                "Object index {index_in_stream} not found in ObjStm (total {})",
                decoded.n_objects
            ))
        })?;

        let absolute_offset = decoded
            .first_offset
            .checked_add(offset_rel)
            .ok_or_else(|| PdfError::InvalidSyntax("Offset overflow in object stream".into()))?;

        if absolute_offset >= decoded.data.len() {
            return Err(PdfError::InvalidSyntax(format!(
                "Object offset {absolute_offset} exceeds decoded stream length {}",
                decoded.data.len()
            )));
        }

        let body_slice = &decoded.data[absolute_offset..];
        let mut parser = Parser::from_bytes(body_slice);
        parser.parse_object()
    }
}
