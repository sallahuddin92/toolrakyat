# SmartPDF Applicationization — Phase 4: Image & Vector Direct Manipulation

## 1. Overview

Phase 4 makes visible PDF images and supported vector shapes directly selectable and editable on the canvas page itself, adhering strictly to the direct manipulation and real PDF mutation principles:
- **PDF page is the editor**: Interactive direct manipulation handles appear directly on top of the selected object on the canvas.
- **Real PDF mutation**: Moving, resizing, recoloring, or deleting an image or vector shape mutates the actual PDF content stream through StarPDF. Export $\rightarrow$ reopen $\rightarrow$ PDF.js render faithfully preserves all modifications.
- **Shared resource isolation**: Editing or moving a paint occurrence of a shared image XObject or vector shape never unintentionally alters other occurrences or pages.
- **Atomicity & History**: 1 completed drag gesture = 1 history transaction. Temporary previews are rendered during pointer drag; on `pointerup`, bounds are converted from screen pixels to PDF user space and committed through the unified command architecture. Pressing `Escape` or making a no-change gesture cancels preview with 0 document changes and 0 history entries.

---

## 2. Engine & WASM Architecture (Rust)

### 2.1 Image Update & Isolation (`engine/starpdf/src/image/editor.rs`)
- Added `UpdateImageSpec` specifying `page_index`, `image_id`, `x`, `y`, `width`, `height`, and `clone_if_shared`.
- Implemented `ImageEditor::update_image`:
  1. Identifies the specific paint occurrence `cm` $\rightarrow$ `Do` instruction pair in the page content stream.
  2. Refuses mutation safely if the image is inside an unsupported inline image or nested Form XObject.
  3. Isolates shared content streams by creating a cloned copy of the stream if referenced across multiple pages.
  4. Rewrites the transformation matrix `[width, 0, 0, height, x, y] cm` in the content stream without altering the underlying `/XObject` `/Im` resource dictionary, guaranteeing that other pages or occurrences sharing the same `/Im` resource remain untouched.
- Added `PdfChange::UpdateImage` in `engine/starpdf/src/mutation/change.rs` and `PdfDocument::update_image`.
- Exported `starpdf_update_image` in `engine/starpdf/src/wasm/api.rs`.

### 2.2 Vector Primitives & Geometry Mutation (`engine/starpdf/src/vector/editor.rs`)
- Supported editable primitives:
  - **Rectangle**: `re` or `m` $\rightarrow$ `l` $\rightarrow$ `l` $\rightarrow$ `l` $\rightarrow$ `h` with `S`, `f`, `B` operators.
  - **Line**: 2-point open path `m` $\rightarrow$ `l` with `S` operator.
- Supported operations:
  - Move / Resize rectangle geometry (`rect_x`, `rect_y`, `rect_w`, `rect_h`).
  - Endpoint 1 and Endpoint 2 line repositioning (`line_x1`, `line_y1`, `line_x2`, `line_y2`).
  - Stroke color (`stroke_color_rgb`), fill color (`fill_color_rgb`), line width (`line_width`), and stroke/fill enablement (`is_stroked`, `is_filled`).
  - Safe refusal for complex clipping paths (`VectorEditability::ComplexClipping`).

---

## 3. TypeScript Client & Command Architecture

- **Worker & Client**:
  - Added `updateImage` method in `StarPdfClient` and `StarPdfDocument` handle.
  - Enhanced `updateGraphic` in `StarPdfClient` with `clone_if_shared` default resilience.
- **Commands**:
  - `UpdateImageCommand` (`src/lib/pdf/commands/image-commands.ts`): Mutates image position/dimensions with incremental export and undo/redo support.
  - `UpdateVectorCommand`, `AddRectangleCommand`, `AddLineCommand`, `DeleteVectorCommand` (`src/lib/pdf/commands/vector-commands.ts`).
- **Coordinate Conversion** (`src/lib/pdf/selection/geometry.ts`):
  - `convertPixelsToPdfRect`: Translates canvas CSS screen pixels (top-left origin, zoom scale, rotation) to standard PDF user coordinates (bottom-left origin).
  - `convertPixelsToPdfPoint` & `convertPdfPointToPixels`: Handles line endpoint mapping with rotation (0°, 90°, 180°, 270°).

---

## 4. UI Direct Manipulation & Overlay

- **Transform Box** (`src/components/tools/implementations/pdf/PdfInteractiveOverlay.tsx`):
  - Renders when an image or vector shape is selected.
  - **4 Corner Resize Handles**: `nw`, `ne`, `se`, `sw` with directional cursor styles (`cursor-nwse-resize`, `cursor-nesw-resize`).
  - **Interior Move Area**: `cursor-move` allowing dragging the object across the canvas.
  - **Line Endpoint Handles**: `vector-line-endpoint-1` and `vector-line-endpoint-2` with `cursor-crosshair` for line path manipulation.
  - **Live SVG Preview**: Interactive dashed stroke preview during drag gesture.
  - **Global Window Event Lifecycle**: Drag gesture lifecycle managed via global window listeners for rock-solid pointer tracking across browsers.
- **Contextual Action Toolbar** (`src/components/tools/implementations/pdf/PdfContextualToolbar.tsx`):
  - Context-aware toolbars for image (format, dimensions, Replace, Remove) and vector (stroke color picker, fill color picker, line width selector, Delete).

---

## 5. Verification & Test Coverage

1. **Rust Unit & Integration Tests**:
   - `cargo test --all-features`: 100% passing across all parser, document, search, mutation, text layout safety, image, and vector test suites.
   - `cargo clippy --all-targets --all-features -- -D warnings`: 0 warnings, 0 errors.
2. **Frontend Unit Tests**:
   - `npm test`: 30 test files, 703 tests passing.
3. **Type & Lint Checks**:
   - `npm run lint`: Clean, 0 errors.
   - `npm run typecheck`: TypeScript passes with 0 errors.
   - `npm run build`: Production build passes cleanly.
4. **Playwright End-to-End Suite**:
   - `npx playwright test src/tests/e2e/smartpdf-editor.spec.ts`: 198 tests passing across Chromium, Firefox, and WebKit.
   - Phase 4 image move/resize with shared XObject paint isolation verified.
   - Phase 4 vector rectangle resize, color styling, and line endpoint editing verified.
