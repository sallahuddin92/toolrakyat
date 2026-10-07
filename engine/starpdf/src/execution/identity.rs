use std::fmt;
use std::str::FromStr;

use super::{ContractRefusal, ContractResult, RefusalCode};
use crate::syntax::object::ObjectRef;

/// Canonical wire string; never exposes a pointer or a JavaScript numeric object ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(try_from = "String", into = "String"))]
pub struct StarNodeId(String);

impl StarNodeId {
    pub fn new(document: &str, object: ObjectRef, occurrence: &str) -> ContractResult<Self> {
        if document.is_empty() || occurrence.is_empty() || object.number == 0 {
            return Err(invalid());
        }
        Ok(Self(format!(
            "sn1/{}/{}/{}/{}",
            hex(document.as_bytes()),
            object.number,
            object.generation,
            hex(occurrence.as_bytes())
        )))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parts(&self) -> (String, ObjectRef, String) {
        // Construction and deserialization validate all fields.
        let parts: Vec<_> = self.0.split('/').collect();
        (
            decode_text(parts[1]).expect("validated document identity"),
            ObjectRef::new(
                parts[2].parse().expect("validated object number"),
                parts[3].parse().expect("validated generation"),
            ),
            decode_text(parts[4]).expect("validated occurrence path"),
        )
    }
}

impl FromStr for StarNodeId {
    type Err = ContractRefusal;
    fn from_str(value: &str) -> ContractResult<Self> {
        let parts: Vec<_> = value.split('/').collect();
        if parts.len() != 5 || parts[0] != "sn1" {
            return Err(invalid());
        }
        let number = parts[2].parse::<u64>().map_err(|_| invalid())?;
        let generation = parts[3].parse::<u16>().map_err(|_| invalid())?;
        let id = Self::new(
            &decode_text(parts[1])?,
            ObjectRef::new(number, generation),
            &decode_text(parts[4])?,
        )?;
        if id.as_str() != value {
            return Err(invalid());
        }
        Ok(id)
    }
}

/// A stable primitive key within its owning selectable occurrence.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "wasm", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "wasm", serde(try_from = "String", into = "String"))]
pub struct RenderNodeId(String);

impl RenderNodeId {
    pub fn new(node: &StarNodeId, primitive: &str) -> ContractResult<Self> {
        if primitive.is_empty() {
            return Err(invalid());
        }
        Ok(Self(format!(
            "rn1/{}/{}",
            hex(node.as_str().as_bytes()),
            hex(primitive.as_bytes())
        )))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn star_node(&self) -> StarNodeId {
        decode_text(self.0.split('/').nth(1).expect("validated render identity"))
            .expect("validated parent encoding")
            .parse()
            .expect("validated parent identity")
    }
}

impl FromStr for RenderNodeId {
    type Err = ContractRefusal;
    fn from_str(value: &str) -> ContractResult<Self> {
        let parts: Vec<_> = value.split('/').collect();
        if parts.len() != 3 || parts[0] != "rn1" {
            return Err(invalid());
        }
        let node = decode_text(parts[1])?.parse()?;
        let id = Self::new(&node, &decode_text(parts[2])?)?;
        if id.as_str() != value {
            return Err(invalid());
        }
        Ok(id)
    }
}

macro_rules! wire_string {
    ($id:ty) => {
        impl fmt::Display for $id {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
        impl From<$id> for String {
            fn from(id: $id) -> Self {
                id.0
            }
        }
        impl TryFrom<String> for $id {
            type Error = ContractRefusal;
            fn try_from(value: String) -> ContractResult<Self> {
                value.parse()
            }
        }
    };
}
wire_string!(StarNodeId);
wire_string!(RenderNodeId);

pub(super) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(value, "{byte:02x}").expect("String write");
    }
    value
}

fn decode_text(value: &str) -> ContractResult<String> {
    if value.is_empty()
        || value.len() % 2 != 0
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid());
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).map_err(|_| invalid()))
        .collect::<ContractResult<Vec<_>>>()?;
    String::from_utf8(bytes).map_err(|_| invalid())
}

fn invalid() -> ContractRefusal {
    ContractRefusal::new(
        RefusalCode::InvalidIdentity,
        "expected a canonical nonempty v1 identity",
    )
}

/// Isolated WASM contract seam; does not open or mutate a production document.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn contract_round_trip_star_node_id(
    value: wasm_bindgen::JsValue,
) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    let node: StarNodeId = serde_wasm_bindgen::from_value(value)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&node).map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn contract_round_trip_render_node_id(
    value: wasm_bindgen::JsValue,
) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    let node: RenderNodeId = serde_wasm_bindgen::from_value(value)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&node).map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
}
