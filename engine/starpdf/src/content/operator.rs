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
    V,          // v: Append cubic Bezier curve (initial point replicated)
    Y,          // y: Append cubic Bezier curve (final point replicated)
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
    N,          // n: End path without fill or stroke
    W,          // W: Set clipping path non-zero
    WStar,      // W*: Set clipping path even-odd
    LineWidth,  // w: Set line width
    LineCap,    // J: Set line cap style
    LineJoin,   // j: Set line join style
    MiterLimit, // M: Set miter limit
    Dash,       // d: Set line dash pattern
    Ri,         // ri: Set color rendering intent
    Flatness,   // i: Set flatness tolerance
    Gs,         // gs: Set graphics state from parameter dictionary
    GStroke,    // G: Set gray level for stroking
    GFill,      // g: Set gray level for non-stroking
    RGStroke,   // RG: Set RGB color for stroking
    RGFill,     // rg: Set RGB color for non-stroking
    KStroke,    // K: Set CMYK color for stroking
    KFill,      // k: Set CMYK color for non-stroking
    CSStroke,   // CS: Set color space for stroking
    CSFill,     // cs: Set color space for non-stroking
    SCStroke,   // SC: Set color for stroking
    SCFill,     // sc: Set color for non-stroking
    SCNStroke,  // SCN: Set color for stroking (extended)
    SCNFill,    // scn: Set color for non-stroking (extended)
    Sh,         // sh: Paint area defined by shading pattern
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
            "v" => Self::V,
            "y" => Self::Y,
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
            "n" => Self::N,
            "W" => Self::W,
            "W*" => Self::WStar,
            "w" => Self::LineWidth,
            "J" => Self::LineCap,
            "j" => Self::LineJoin,
            "M" => Self::MiterLimit,
            "d" => Self::Dash,
            "ri" => Self::Ri,
            "i" => Self::Flatness,
            "gs" => Self::Gs,
            "G" => Self::GStroke,
            "g" => Self::GFill,
            "RG" => Self::RGStroke,
            "rg" => Self::RGFill,
            "K" => Self::KStroke,
            "k" => Self::KFill,
            "CS" => Self::CSStroke,
            "cs" => Self::CSFill,
            "SC" => Self::SCStroke,
            "sc" => Self::SCFill,
            "SCN" => Self::SCNStroke,
            "scn" => Self::SCNFill,
            "sh" => Self::Sh,
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
            Self::V => "v",
            Self::Y => "y",
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
            Self::N => "n",
            Self::W => "W",
            Self::WStar => "W*",
            Self::LineWidth => "w",
            Self::LineCap => "J",
            Self::LineJoin => "j",
            Self::MiterLimit => "M",
            Self::Dash => "d",
            Self::Ri => "ri",
            Self::Flatness => "i",
            Self::Gs => "gs",
            Self::GStroke => "G",
            Self::GFill => "g",
            Self::RGStroke => "RG",
            Self::RGFill => "rg",
            Self::KStroke => "K",
            Self::KFill => "k",
            Self::CSStroke => "CS",
            Self::CSFill => "cs",
            Self::SCStroke => "SC",
            Self::SCFill => "sc",
            Self::SCNStroke => "SCN",
            Self::SCNFill => "scn",
            Self::Sh => "sh",
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
