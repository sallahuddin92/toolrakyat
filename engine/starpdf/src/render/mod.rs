pub mod compiler;
pub mod differential;
pub mod ir;
pub mod rasterizer;

pub use compiler::RenderCompiler;
pub use differential::{DifferentialComparator, DifferentialMetrics};
pub use ir::{
    ColorRgba, FillRule, FillStyle, LineCap, LineJoin, PathVerb, RenderClip, RenderGlyph,
    RenderImage, RenderItem, RenderPage, RenderPath, RenderTextRun, StrokeStyle, Transform2D,
    UnsupportedOperatorReport,
};
pub use rasterizer::{NativeRasterizer, RenderedPage};
