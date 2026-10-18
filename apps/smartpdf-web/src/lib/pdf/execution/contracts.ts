/** Phase 0 wire contracts. No production editor or renderer imports this module. */
declare const starNodeBrand: unique symbol;
declare const renderNodeBrand: unique symbol;
export type StarNodeId = string & { readonly [starNodeBrand]: true };
export type RenderNodeId = string & { readonly [renderNodeBrand]: true };
/** Decimal u64 string: never turn an object number into a JS Number. */
export interface PdfObjectRef { readonly number: string; readonly generation: number }
export type Revision = string; // Canonical unsigned decimal u64 at a future DTO boundary.
export type EvidenceLevel = "VERIFIED" | "RECOVERED_VERIFIED" | "INFERRED" | "UNTRUSTED";
export type StorageKind = "Direct" | "Compressed" | "Generated" | "Replaced" | "Recovered";
export type ContentHash = string; // SHA-256: 64 lowercase hex digits.
export type SourceLocation =
  | { readonly kind: "FileOffset"; readonly offset: string }
  | { readonly kind: "ObjectStream"; readonly container: PdfObjectRef; readonly index: number }
  | { readonly kind: "Generated" | "Unknown" };
export interface EvidenceRecord {
  readonly level: EvidenceLevel;
  readonly proof: string;
  readonly source_revision: Revision | null;
  readonly provenance: SourceLocation | null;
}
export interface EvidencePolicy {
  readonly operation: string;
  readonly accepted_levels: readonly EvidenceLevel[];
  readonly required_revision: Revision;
}
export interface ObjectState {
  readonly object_ref: PdfObjectRef;
  readonly storage_kind: StorageKind;
  readonly source_revision: Revision | null;
  readonly source_location: SourceLocation;
  readonly effective_value:
    | { readonly kind: "StoreObject"; readonly object_ref: PdfObjectRef }
    | { readonly kind: "ShadowValue"; readonly hash: ContentHash };
  readonly evidence: EvidenceRecord;
  readonly content_hash: ContentHash | null;
}
/** [left,bottom,right,top], unrotated PDF default user space, before UserUnit. */
export type PdfBox = readonly [number, number, number, number];
export type Point = readonly [number, number];
export interface PageBounds { readonly page_index: number; readonly rect: PdfBox }
export interface SourceRange {
  readonly owner: PdfObjectRef;
  readonly revision: Revision;
  readonly space: "FileBytes" | "DecodedStream";
  readonly start: string;
  readonly end: string;
}
export interface ResourceKey { readonly owner: PdfObjectRef; readonly category: string; readonly name: string }
export interface ValueChange<T> { readonly before: T; readonly after: T }
export interface SemanticChange {
  readonly text: ValueChange<string> | null;
  readonly font: ValueChange<ContentHash> | null;
  readonly geometry: ValueChange<PageBounds> | null;
  readonly other: readonly string[];
}
/** Rust maps/sets project as sorted entries, not object-key stringification or JSON BigInts. */
export type Entries<K, V> = readonly (readonly [K, V])[];
export interface DocumentDelta {
  readonly added_objects: readonly PdfObjectRef[];
  readonly modified_objects: readonly PdfObjectRef[];
  readonly removed_references: readonly { readonly from: PdfObjectRef; readonly to: PdfObjectRef }[];
  readonly source_range_edits: Entries<SourceRange, ValueChange<ContentHash>>;
  readonly resource_changes: Entries<ResourceKey, ValueChange<PdfObjectRef | null>>;
  readonly semantic_changes: Entries<StarNodeId, SemanticChange>;
  readonly affected_pages: readonly number[];
  readonly before_hashes: Entries<PdfObjectRef, ContentHash>;
  readonly after_hashes: Entries<PdfObjectRef, ContentHash>;
}
export interface MutationEnvelope {
  readonly target_nodes: readonly StarNodeId[];
  readonly allowed_pdf_objects: readonly PdfObjectRef[];
  readonly allowed_resources: readonly ResourceKey[];
  readonly allowed_source_ranges: readonly SourceRange[];
  readonly allowed_bounds: readonly PageBounds[];
  readonly expected_text: Entries<StarNodeId, ValueChange<string>>;
  readonly expected_font_changes: Entries<StarNodeId, ValueChange<ContentHash>>;
  readonly expected_geometry_changes: Entries<StarNodeId, ValueChange<PageBounds>>;
  readonly forbidden_dependencies: readonly PdfObjectRef[];
}
export type RefusalCode = "InvalidIdentity" | "InvalidCoordinates" | "InvalidHashInput"
  | "InsufficientEvidence" | "StaleRevision" | "AmbiguousObjectState" | "InvalidDelta"
  | "IncompleteClosure" | "ForbiddenDependency" | "UnexplainedChange" | "UnsupportedOperation";
export interface ContractRefusal { readonly code: RefusalCode; readonly reason: string }
export type ContractResult<T> = { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly refusal: ContractRefusal };
export type MutationCertificate =
  | { readonly classification: "EXPLAINED"; readonly unexplained_changes: readonly []; readonly refusal: null }
  | { readonly classification: "UNEXPLAINED"; readonly unexplained_changes: readonly string[]; readonly refusal: null }
  | { readonly classification: "REFUSED"; readonly unexplained_changes: readonly []; readonly refusal: ContractRefusal };
export interface StructuralClosure {
  readonly objects: readonly PdfObjectRef[];
  readonly resources: readonly ResourceKey[];
  readonly source_ranges: readonly SourceRange[];
  readonly completeness: "Complete" | "Incomplete";
}
export interface VisualClosure {
  readonly bounds: Entries<RenderNodeId, PageBounds>;
  readonly completeness: "Complete" | "Incomplete";
}
export interface StarExecutionState {
  readonly effective_object_index: Entries<PdfObjectRef, ObjectState>;
  readonly provenance: Entries<StarNodeId, readonly { readonly object_ref: PdfObjectRef; readonly source: SourceLocation }[]>;
  readonly current_revision: Revision;
  readonly pending_delta: DocumentDelta | null;
  readonly evidence_records: Entries<StarNodeId, EvidenceRecord>;
}
/** Effective decisions require both structural evidence and document permissions. */
export interface DocumentCapabilityDecision { readonly allowed: boolean; readonly reason: string }
export interface DocumentCapabilities {
  readonly can_view: DocumentCapabilityDecision;
  readonly can_edit: DocumentCapabilityDecision;
  readonly can_extract: DocumentCapabilityDecision;
  readonly can_annotate: DocumentCapabilityDecision;
  readonly can_modify_pages: DocumentCapabilityDecision;
}
export interface RendererDocumentState { readonly document_id: string; readonly revision: Revision }
export interface CoordinateConfig {
  readonly media_box: PdfBox;
  readonly crop_box?: PdfBox;
  readonly rotation: 0 | 90 | 180 | 270; // Effective clockwise PDF /Rotate, composed once.
  readonly scale: number; // CSS pixels per point (1/72 inch).
  readonly user_unit: number;
  readonly device_pixel_ratio: number;
}
export interface CoordinateTransform {
  pdfToViewport(point: Point): Point;
  viewportToPdf(point: Point): Point;
  viewportToDevice(point: Point): Point;
  deviceToViewport(point: Point): Point;
  pdfBoxToViewport(rect: PdfBox): PdfBox;
}
export interface ViewportRequest {
  readonly page_index: number;
  readonly transform: CoordinateTransform;
  readonly viewport_bounds: PdfBox; // [left,top,right,bottom] in CSS pixels.
}
export interface RenderHit {
  readonly render_node: RenderNodeId;
  readonly star_node: StarNodeId;
  readonly bounds: PageBounds;
}
export interface Renderer<Document, Frame> {
  open(document: Document, state: RendererDocumentState): Promise<ContractResult<void>>;
  currentState(): RendererDocumentState | null;
  update(state: RendererDocumentState, delta: DocumentDelta): Promise<ContractResult<void>>;
  renderViewport(viewport: ViewportRequest): Promise<ContractResult<Frame>>;
  hitTest(pageIndex: number, pdfPoint: Point): ContractResult<readonly RenderHit[]>;
  invalidate(bounds: readonly PageBounds[]): ContractResult<void>;
}
