use super::{
    ContractResult, CoordinateTransform, DocumentDelta, PageBounds, RenderNodeId, StarNodeId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererDocumentState {
    pub document_id: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewportRequest {
    pub page_index: u32,
    pub transform: CoordinateTransform,
    /// [left, top, right, bottom] in viewport CSS pixels.
    pub viewport_bounds: [f64; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderHit {
    pub render_node: RenderNodeId,
    pub star_node: StarNodeId,
    pub bounds: PageBounds,
}

/// Resource ownership and async scheduling belong to concrete adapters.
/// Rust renderers may use an owned task as Frame; TS adapters return Promises.
pub trait Renderer {
    type Document;
    type Frame;
    fn open(
        &mut self,
        document: Self::Document,
        state: RendererDocumentState,
    ) -> ContractResult<()>;
    fn current_state(&self) -> Option<&RendererDocumentState>;
    fn update(&mut self, state: RendererDocumentState, delta: &DocumentDelta)
        -> ContractResult<()>;
    fn render_viewport(&mut self, viewport: &ViewportRequest) -> ContractResult<Self::Frame>;
    /// Point is unrotated PDF user space; caller chooses among returned candidates.
    fn hit_test(&self, page_index: u32, point: [f64; 2]) -> ContractResult<Vec<RenderHit>>;
    fn invalidate(&mut self, bounds: &[PageBounds]) -> ContractResult<()>;
}
