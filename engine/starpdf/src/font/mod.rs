pub mod appearance;
pub mod cache;
pub mod catalog;
pub mod cmap;
pub mod coverage;
pub mod embed;
pub mod encoding;
pub mod font;
pub mod planner;
pub mod resource;
pub mod sfnt;
pub mod shaping;
pub mod standard_metrics;
pub mod style;
pub mod subset;

pub use cache::{get_font_cache, FontRuntimeCache};
pub use catalog::{
    find_candidate_fallbacks, get_font_registry, FontAssetRegistry, FontCatalogEntry,
    BUILTIN_FONT_CATALOG,
};
pub use cmap::UnicodeCMap;
pub use coverage::{CoarseCoverageBitmap, COARSE_COVERAGE_BITMAP_BYTES, COARSE_UNICODE_PAGE_COUNT};
pub use embed::{EmbeddedType0Resource, Type0FontEmbedder};
pub use encoding::SimpleEncoding;
pub use font::{Font, FontFamily, FontProgramKind, FontStyle};
pub use planner::{
    plan_adaptive_text, AdaptiveTextPlan, LayoutSafetyClassification, ReplacementStrategy,
    TextPlanner, TextReplacementPlan,
};
pub use resource::PageResources;
pub use sfnt::SfntFont;
pub use shaping::{ShapedGlyph, ShapedRun, TextDirection, TextShaper};
pub use standard_metrics::StandardFontMetrics;
pub use style::{
    font_family_name, parse_font_family, style_from_da_font_name, validate_text_font_size,
    ComputedTextStyle, TextStyleCapability, TextStylePatch, TextStylePlan, TextStylePlanner,
    TextWeight, MAX_TEXT_FONT_SIZE, MIN_TEXT_FONT_SIZE,
};
