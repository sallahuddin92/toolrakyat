# SmartPDF v0.20 (RC1) Release Notes

ToolRakyat is proud to announce the **Release Candidate 1 (RC1)** of **SmartPDF**, powered by the local-first **StarPDF** Rust WebAssembly engine.

SmartPDF v0.20 brings full document editing capabilities directly to your browser with complete privacy and zero server roundtrips.

---

## Highlights in RC1

### 🔒 Pure Client-Side Privacy
Your documents never leave your device. All PDF parsing, text searching, content modification, and rendering happen 100% locally in your browser using high-performance WebAssembly.

### ✍️ Native Existing-Text Editing
Select native text runs directly on the page canvas or find words instantly with full-document keyword search. Modify text directly in content streams while preserving original fonts, layouts, and formatting.

### 🖼️ Image & Vector Graphics Editing
Replace or remove embedded images with automatic color space and dimension preservation. Add, resize, and restyle vector shapes and lines directly on the canvas.

### 📋 Interactive Forms & Annotations
Fill out and edit interactive AcroForm fields (text fields, checkboxes, radio groups, dropdown menus) and modify markup annotations (`FreeText`, `Square`, `Highlight`) with instant on-canvas feedback.

### 📑 Comprehensive Page Management
Reorder, duplicate, delete, and insert blank pages with ease. Extract specific pages or merge multiple PDF documents together into a single downloadable file.

### 🔄 Multi-Level History & Lifecycle Protection
Seamless Undo / Redo (`Ctrl+Z` / `Cmd+Z`, `Ctrl+Shift+Z` / `Cmd+Shift+Z`) with up to 25 snapshots. Built-in safeguards protect against losing unsaved modifications when opening or switching documents.

### 🌐 Cross-Browser Qualified
Tested and verified across **Chromium** (Google Chrome, Microsoft Edge, Brave), **Mozilla Firefox**, and **Playwright WebKit** with a 100% pass rate across 171 automated end-to-end test suites.

---

## Supported Environments
- Google Chrome / Chromium-based browsers
- Mozilla Firefox
- WebKit-based browsers
- Desktop & Laptop screen resolutions (1280x720, 1440x900, 1920x1080 and above)

---

## Known Boundaries & Safety Policies
- **Encrypted Documents**: Password-protected or security-handler-restricted documents display a clear, informative message and are safely protected.
- **Complex Writing Systems**: Vertical writing and complex non-Latin scripts remain view-only to prevent text layout distortions.
