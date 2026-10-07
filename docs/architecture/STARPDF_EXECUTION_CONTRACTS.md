# Decision: StarPDF execution contracts v1 (Phase 0)

Status: additive contracts; no production callers. Base: `972c068960ed6e868f553bbc34d64019cf590f77`.

## Problem and constraints

Selection, mutation and rendering need shared identities, provenance, change descriptions and explicit proof boundaries. Existing `ObjectRef`, `MutationPlan`, `PdfDocument`, `ObjectStore`, mutation paths and the PDF.js production renderer remain authoritative. This phase defines a small shadow layer and focused checks. It introduces no new PDF editing, decryption, graph traversal or renderer implementation.

## Alternatives and decision

Replacing the parser/renderer together would make qualification and rollback difficult. Using a bare object number as selection identity loses repeated occurrences, and using coordinates or revision numbers makes identity change during editing. Duplicating PDF objects in a second store would increase memory and create competing authorities.

Use opaque occurrence IDs, compact references to existing values, ordered deltas and operation-specific evidence policies. Add `execution` beside the engine, and an unused TypeScript contract module beside the existing client. SHA-256 uses the `sha2` crate and Web Crypto, rather than a bespoke digest. `serde_json` is a test-only dependency.

## Chosen contracts

| Contract | Meaning |
|---|---|
| StarNodeId | Stable document namespace + existing ObjectRef + occurrence path. Geometry/revision are excluded. |
| RenderNodeId | StarNodeId + stable primitive key; several primitives can link to one occurrence. |
| EvidenceRecord | VERIFIED, RECOVERED_VERIFIED, INFERRED or UNTRUSTED, proof/reason, optional source revision/location. |
| EvidencePolicy | Per-operation accepted levels and required revision. No enum ordering implies permission. |
| ObjectState | Reference, storage kind, optional actual source revision/location, store object or hashed shadow value, evidence, optional hash. |
| DocumentDelta | Ordered sets/maps of object, reference, range, resource and semantic changes, pages and before/after hashes. |
| MutationEnvelope | Exact targets, object/resource/range/bounds allowances, expected text/font/geometry changes and forbidden dependencies. |
| MutationCertificate | EXPLAINED, UNEXPLAINED or REFUSED. `require_explained` rejects any unexplained change. |
| StarExecutionState | Lazy effective xref snapshot plus empty provenance/evidence maps and optional pending delta. Not production authority. |
| Renderer | Open/current state, delta update, viewport render, hit-test candidates and bounds invalidation. Rust task/frame type and TS Promise adapters can own scheduling. |
| Cs(Δ) / StructuralClosure | Supplied dependency objects, resources and source ranges plus explicit completeness. |
| Cv(Δ) / VisualClosure | Supplied render-node bounds covering old and new visual extent plus explicit completeness. |

Only known distinct source revisions establish ObjectState precedence. Equal conflicting or unknown revisions refuse comparison. Identical states are idempotent; stale known states cannot supersede newer states. Generated and replaced values use hash references, without copying `PdfObject` or changing the existing store.

The shadow constructor reads the existing effective xref without resolving objects or changing its cache. That xref does not retain the source revision of every object. Therefore source revision and content hash are initially absent, and evidence is UNTRUSTED. It does not manufacture VERIFIED or RECOVERED_VERIFIED from document-level recovery status. The caller supplies the current shadow document-state revision separately.

## Identity transport

`sn1/<document UTF-8 lowercase hex>/<object u64 decimal>/<generation u16 decimal>/<occurrence UTF-8 lowercase hex>`.

`rn1/<canonical StarNodeId UTF-8 lowercase hex>/<primitive key UTF-8 lowercase hex>`.

Components are nonempty, object number is positive, decimal fields have no leading zeros, and invalid UTF-8/noncanonical encodings refuse. Hex framing preserves separators and Unicode exactly. TypeScript rejects lone UTF-16 surrogates. A document namespace is chosen once for the document lifecycle and retained across edits. Occurrence paths and primitive keys must be retained/remapped by future editing adapters, rather than rebuilt blindly from mutable instruction offsets.

Rust identity newtypes serialize through serde as strings. Isolated `contract_round_trip_*` WASM functions deserialize and serialize with `serde-wasm-bindgen`. TypeScript branded strings and codecs preserve the full u64 object number; no JS Number conversion is allowed. Shared golden vectors include Unicode, a reused occurrence path and u64 maximum.

Other TypeScript structures describe future DTO shapes, not implemented production WASM APIs. Object numbers, revisions and source offsets use decimal strings. Rust BTreeMap/BTreeSet values must be projected as sorted entries; numbers compare numerically, IDs compare canonical UTF-8 bytes. Future DTO adapters must validate these shapes; plain JSON serialization of Rust numeric references is insufficient.

## Delta and certificate rules

Added and modified object sets are disjoint. Before hashes cover exactly modified objects; after hashes cover exactly added plus modified objects. Range edits must identify a modified owner, source revision and byte space; ranges are half-open `[start,end)`, with zero-length insertions permitted at an allowed boundary. Physical file bytes and decoded-stream offsets cannot be mixed.

Comparison requires complete Cs(Δ) and Cv(Δ), consistent dependency coverage, valid finite bounds and visual-page coverage. Resource before/after references and removed-reference endpoints must be in Cs(Δ). Geometry must be covered by Cv(Δ) for its occurrence. Forbidden structural dependencies refuse certification. Changes outside declared allowances, mismatched expected semantics and unknown semantic kinds are UNEXPLAINED. All empty default closures are incomplete, so default construction cannot silently approve an edit.

**UNEXPLAINED CHANGE = 0** is an acceptance invariant. The comparison validates supplied facts, not actual PDF bytes, graph reachability, pixels or cryptographic signatures. A caller marking a closure complete does not independently prove it. Future mutation and qualification adapters must derive/verify those facts before the certificate becomes a production gate. No existing mutation is retrofitted in Phase 0.

## Coordinate convention

PDF boxes and PageBounds are `[left,bottom,right,top]` in unrotated default user space. Units are multiplied by `/UserUnit` (default 1), then by CSS pixels per PDF point. Use effective inherited MediaBox, CropBox and clockwise `/Rotate`; intersect crop with media, and reject empty/invalid intersections. Caller-added viewing rotation is composed with `/Rotate` exactly once before constructing the transform.

Viewport points have a top-left origin in CSS pixels local to the page; pan/DOM positioning is external. For effective box `(l,b,r,t)`, PDF point `(x,y)` becomes, before scale:

| Rotation | Viewport point |
|---|---|
| 0 | `(x-l, t-y)` |
| 90 | `(y-b, x-l)` |
| 180 | `(r-x, y-b)` |
| 270 | `(t-y, r-x)` |

Device pixels are CSS viewport pixels multiplied by devicePixelRatio, separately from page scale and UserUnit. Point transforms do not clip or round. Bounds transform all four corners; minimum interaction handle size belongs to UI presentation. Finite coordinates and positive finite scale factors are required. Existing production geometry helpers stay unchanged; migration must account for their current clipping/minimum-size behavior.

## Deterministic hash profile

SHA-256 input is: UTF-8 `StarPDF.semantic.v1` followed by NUL; one domain byte (Font=1, Image=2, TextRun=3, Annotation=4, PdfObject=5); big-endian u64 field count; then fields sorted by nonempty ASCII name. Each field is u64 name-byte-length, exact name bytes, u64 value-byte-length, exact value bytes. Digests are 32 bytes in Rust and 64 lowercase hex characters in future DTOs.

Text values use exact UTF-8 without implicit Unicode normalization. Semantic numeric values use finite big-endian IEEE-754 binary64 with negative zero normalized to positive zero. Binary payloads are exact bytes. Never include pointers, process randomness, viewport/selection/UI state, or revision counters in semantic inputs. Domain separation and length framing avoid cross-kind and concatenation ambiguity.

Future profiles can supply font program/encoding/metrics; image samples/color interpretation; text glyphs/font/placement; annotation subtype/content/appearance. The initial API accepts named bytes, and does not claim to extract or canonicalize all PDF semantics. Domain-specific required-field profiles and decoded-versus-encoded stream policy remain unresolved; callers must not compare hashes made under different semantic profiles.

## Migration and unresolved questions

Phase 0 contracts are unused by production. Later work can derive deltas beside MutationPlan, populate per-object evidence and occurrence provenance, then expose certificates and selection links through coordinator-owned adapters. PDF.js remains default, fallback and oracle. Replacement requires corpus, performance, memory and export/reopen evidence. Structural healing export is future work; no novelty claim is made for these foundations.

Open questions: durable occurrence remapping across stream rewrites; per-object revision provenance in recovered xrefs; domain-specific semantic hash schemas; independent closure completeness proofs; adapter task cancellation/resource disposal; pixel-diff tolerances. These do not silently become guarantees of the v1 contract.

## Qualification

From the dedicated worktree:

```sh
cargo fmt --manifest-path engine/starpdf/Cargo.toml --check
cargo clippy --manifest-path engine/starpdf/Cargo.toml --all-features --lib --test execution_contracts_tests -- -D warnings
CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_INCREMENTAL=false cargo test --manifest-path engine/starpdf/Cargo.toml --all-features
cargo build --manifest-path engine/starpdf/Cargo.toml --release
npm run typecheck
npx eslint src/lib/pdf/execution scripts/test-starpdf-contracts-wasm.mjs --max-warnings 0
node scripts/test-starpdf-contracts-wasm.mjs
npm run build
```

Test debug symbols and incremental output are disabled above to keep test artifacts within the available disk space; all-feature test coverage is unchanged.

The WASM test runner builds Node bindings in a temporary directory and removes them after the tests; it never regenerates checked-in production assets. It needs the wasm32 target and a wasm-bindgen CLI matching Cargo's resolved wasm-bindgen version. The repo ignores Cargo.lock. This run uses CLI/crate 0.2.127 via the dedicated worktree's ignored lock only; no existing dependency constraint is changed. To reproduce with that CLI after fresh dependency resolution, use `cargo update --manifest-path engine/starpdf/Cargo.toml -p js-sys --precise 0.3.104` and `cargo update --manifest-path engine/starpdf/Cargo.toml -p wasm-bindgen --precise 0.2.127`, or use a matching CLI for the resolved version. An isolated CLI can be provided through `STARPDF_WASM_BINDGEN`.

Recorded qualification: fmt and Clippy with warnings denied passed; all-feature Rust suite 373 passed (26 new contract tests); TypeScript and focused ESLint passed; focused Vitest 27 passed including the actual WASM serde round trip; native release, isolated WASM release and Next production builds passed. Original checkout content/status and RC1 reference matched the pre-work snapshot.
