use crate::content::operand::ContentOperand;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentOperator {
    Q,          // q: Save graphics state
    QEnd,       // Q: Restore graphics state
    Cm,         // cm: Current transformation matrix
    Bt,         // BT: Begin text object
    Et,         // ET: End text object
    Tf,         // Tf: Set text font and size
    Tm,         // Tm: Set text matrix
    Td,         // Td: Move text position
    TD,         // TD: Move text position and set leading
    TStar,      // T*: Move to start of next line
    Tj,         // Tj: Show text string
    TJ,         // TJ: Show text with glyph positioning
    Do,         // Do: Invoke named XObject
    Re,         // re: Append rectangle to path
    M,          // m: Move to
    L,          // l: Line to
    C,          // c: Append cubic Bezier curve to path
    H,          // h: Close subpath
    S,          // S: Stroke path
    SClose,     // s: Close and stroke path
    F,          // f: Fill path non-zero
    FUpper,     // F: Fill path non-zero (equivalent to f)
    FStar,      // f*: Fill path even-odd
    B,          // B: Fill and stroke path non-zero
    BStar,      // B*: Fill and stroke path even-odd
    BClose,     // b: Close, fill, and stroke path non-zero
    BCloseStar, // b*: Close, fill, and stroke path even-odd
    Unknown(String),
}

impl ContentOperator {
    pub fn from_keyword(keyword: &str) -> Self {
        match keyword {
            "q" => Self::Q,
            "Q" => Self::QEnd,
            "cm" => Self::Cm,
            "BT" => Self::Bt,
            "ET" => Self::Et,
            "Tf" => Self::Tf,
            "Tm" => Self::Tm,
            "Td" => Self::Td,
            "TD" => Self::TD,
            "T*" => Self::TStar,
            "Tj" => Self::Tj,
            "TJ" => Self::TJ,
            "Do" => Self::Do,
            "re" => Self::Re,
            "m" => Self::M,
            "l" => Self::L,
            "c" => Self::C,
            "h" => Self::H,
            "S" => Self::S,
            "s" => Self::SClose,
            "f" => Self::F,
            "F" => Self::FUpper,
            "f*" => Self::FStar,
            "B" => Self::B,
            "B*" => Self::BStar,
            "b" => Self::BClose,
            "b*" => Self::BCloseStar,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Q => "q",
            Self::QEnd => "Q",
            Self::Cm => "cm",
            Self::Bt => "BT",
            Self::Et => "ET",
            Self::Tf => "Tf",
            Self::Tm => "Tm",
            Self::Td => "Td",
            Self::TD => "TD",
            Self::TStar => "T*",
            Self::Tj => "Tj",
            Self::TJ => "TJ",
            Self::Do => "Do",
            Self::Re => "re",
            Self::M => "m",
            Self::L => "l",
            Self::C => "c",
            Self::H => "h",
            Self::S => "S",
            Self::SClose => "s",
            Self::F => "f",
            Self::FUpper => "F",
            Self::FStar => "f*",
            Self::B => "B",
            Self::BStar => "B*",
            Self::BClose => "b",
            Self::BCloseStar => "b*",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for ContentOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContentInstruction {
    pub operands: Vec<ContentOperand>,
    pub operator: ContentOperator,
}

impl ContentInstruction {
    #[inline]
    pub const fn new(operands: Vec<ContentOperand>, operator: ContentOperator) -> Self {
        Self { operands, operator }
    }
}
