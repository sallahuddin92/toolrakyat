# SmartPDF Design Brief & Interaction System

**Document ID:** DOC-DSN-001  
**Status:** CANONICAL DRAFT (Phase 0 / Pre-Implementation)  
**System:** SmartPDF Design Tokens, Typography & UX Patterns  
**Repository:** `sallahuddin92/toolrakyat`  

---

## 1. Product Personality & Design Philosophy

SmartPDF is a **precision instrument for document engineering**. It bridges the gap between deep systems-level safety and fluid, human-centric design.

### 1.1 Core Tenets
1. **Familiarity Without Imitation:** SmartPDF adopts standard, time-tested workflows from professional suites (Acrobat Pro, Figma, VS Code), ensuring zero learning curve for advanced users. **However, SmartPDF strictly avoids Adobe trademarks, proprietary icons, exact colors, and trade dress.** It features its own distinct identity.
2. **High Information Density with Calm Restraint:** Controls are compact, legible, and unobtrusive. The document viewport remains the absolute visual focal point.
3. **Deterministic Feedback:** The interface never leaves the user guessing. Every mutation, font substitution, or security restriction is clearly explained with actionable, non-cryptic status indications.
4. **Local-First Reassurance:** Visible, discreet indicators continuously remind the user that processing is 100% private, on-device, and unmetered.

---

## 2. Visual Identity & Design Tokens

### 2.1 Color Palette
SmartPDF utilizes a neutral slate foundation accented with **Cyan-Cobalt** primary indicators and semantic status colors.

```
Neutral Foundation (Dark & Light modes):
  Base Background:      #0F172A (Slate 900) / #F8FAFC (Slate 50)
  Surface Primary:      #1E293B (Slate 800) / #FFFFFF (Pure White)
  Surface Secondary:    #334155 (Slate 700) / #F1F5F9 (Slate 100)
  Border Subdued:       #475569 (Slate 600) / #E2E8F0 (Slate 200)
  Border Highlight:     #64748B (Slate 500) / #CBD5E1 (Slate 300)

Brand & Accent:
  Primary Accent:       #0EA5E9 (Sky 500)
  Primary Hover:        #0284C7 (Sky 600)
  Primary Glow:         rgba(14, 165, 233, 0.25)

Semantic Status:
  Verified / Success:   #10B981 (Emerald 500)
  Warning / Recovered:  #F59E0B (Amber 500)
  Error / Refusal:      #EF4444 (Rose 500)
  Inferred / Info:      #6366F1 (Indigo 500)
```

### 2.2 Typography Scale
- **UI Font Family:** Modern geometric grotesque system stack:
  `Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`
- **Monospace Font Family:**
  `"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, monospace`
- **Type Scale:**
  - Micro (Captions, Badge tags): `11px / 14px (Font Weight: 500)`
  - Small (Tooltips, Property labels, Menus): `12px / 16px (Font Weight: 400 / 500)`
  - Base (Default UI text, Buttons, Inputs): `13px / 18px (Font Weight: 400 / 600)`
  - Medium (Section headings, Panel titles): `14px / 20px (Font Weight: 600)`
  - Large (Modal headers, Tab titles): `16px / 24px (Font Weight: 700)`
  - Display (Home hero banners): `24px / 32px (Font Weight: 800)`

### 2.3 Spatial Scale & Density
- Base grid unit: **4px**
- Control heights:
  - Global Top Bar: `48px`
  - Secondary Action Bar: `36px`
  - Standard Button / Input: `30px`
  - Compact / Contextual Button: `24px`
  - Bottom Status Bar: `26px`
- Border Radii:
  - Badges / Micro tags: `4px`
  - Buttons / Inputs / Cards: `6px`
  - Floating Toolbars / Popups / Modals: `8px`
  - Selection Marquee Handles: `2px`

---

## 3. Desktop Interaction Architecture

### 3.1 Three-Panel Workspace Anatomy

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ ✦ SmartPDF   File  Edit  View  Insert  Tools  Window  Help      [Doc1.pdf ×] [+]  [Export ▾]│  <-- Global Menu & Tab Bar (48px)
├────────────────────────────────────────────────────────────────────────────────────────┤
│ [Pointer ▾] [Text] [Image] [Shape] [Form] [Annotate] [Pages] │ 🔍 Find │ ↩ ↪ │ 🛡️ Private│  <-- Quick Tools Bar (36px)
├──────────────┬─────────────────────────────────────────────────────────┬───────────────┤
│ NAVIGATION   │                                                         │ PROPERTIES    │
│ RAIL (240px) │                  CENTRAL VIEWPORT                       │ PANEL (280px) │
│              │                                                         │ (Collapsible) │
│ [Thumbnails] │                      ┌─────────────────┐                │               │
│ [Bookmarks]  │                      │ Document Page 1 │                │ [Selection]   │
│ [Layers]     │                      │                 │                │ Type: Text    │
│ [Comments]   │                      │   [Hello World] │                │ Font: NotoSans│
│ [Attachments]│                      │                 │                │ Size: 14pt    │
│              │                      │ ┌─────────────┐ │                │ Fill: #000000 │
│              │                      │ │ Context Bar │ │                │ Align: Left   │
│              │                      │ └─────────────┘ │                │               │
│              │                      └─────────────────┘                │ [History]     │
├──────────────┴─────────────────────────────────────────────────────────┴───────────────┤
│ Page 1 of 12  │  [−] 125% [+] (Fit Page)  │  Sel: Text Span  │  Evidence: VERIFIED  │  <-- Status Bar (26px)
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Floating Contextual Toolbars
When an element (Text, Image, Vector, Annotation, Form Field) is selected, a lightweight, floating toolbar appears **4px above or below** the selection bounds:
- **Text Selection:** Font Family Picker, Font Size Stepper, Bold/Italic/Underline/Strike, Text Color Picker, Text Alignment, Edit/Replace Input.
- **Image Selection:** Replace Image, Delete, Download/Extract, Crop, Aspect Ratio Lock, Reset Transform.
- **Vector Selection:** Stroke Color, Fill Color, Line Width Stepper, Delete, Bring Forward/Send Backward.
- **Annotation Selection:** Comment Popover, Color Swatches, Border Width, Opacity Slider, Delete.
- **Form Widget Selection:** Field Name, Field Type, Value Input, Clear Value, Read-Only Toggle.

### 3.3 Command Palette (`Cmd+K` / `Ctrl+K`)
Instant searchable command surface exposing 100% of application capabilities:
- Type `"watermark"` -> triggers Add Watermark action.
- Type `"extract"` -> triggers Extract Pages modal.
- Type `"rotate"` -> triggers Rotate Current Page 90° Clockwise.
- Type `"encrypt"` -> opens Security & Password settings.

---

## 4. Selection & Direct Manipulation Model

### 4.1 Selection Bounds & Transform Handles
- Active selection is outlined in `Primary Accent` (`#0EA5E9`) with a 1.5px solid border.
- 8 bounding handles (4 corners, 4 midpoints) rendered as crisp 7x7px white squares with 1.5px slate borders.
- Hover states on handles provide directional resize cursors (`nwse-resize`, `nesw-resize`, `ns-resize`, `ew-resize`).
- When dragging, subtle dashed snap lines appear when aligning to page margins, horizontal centers, or adjacent object bounding boxes.

### 4.2 Multi-Select Box (Marquee)
- Dragging on empty canvas space initiates a translucent marquee selection rectangle (`rgba(14, 165, 233, 0.15)` fill with `rgba(14, 165, 233, 0.6)` dashed border).
- Releasing selects all objects whose bounding boxes intersect the marquee.

---

## 5. Keyboard & Accessibility Architecture

### 5.1 Canonical Keyboard Shortcuts

| Category | Action | macOS Shortcut | Windows / Linux Shortcut |
| :--- | :--- | :--- | :--- |
| **File** | Open File | `Cmd+O` | `Ctrl+O` |
| **File** | Save / Incremental Save | `Cmd+S` | `Ctrl+S` |
| **File** | Save As / Export | `Cmd+Shift+S` | `Ctrl+Shift+S` |
| **File** | Close Document Tab | `Cmd+W` | `Ctrl+W` |
| **Edit** | Undo | `Cmd+Z` | `Ctrl+Z` |
| **Edit** | Redo | `Cmd+Shift+Z` | `Ctrl+Y` / `Ctrl+Shift+Z` |
| **Edit** | Cut Selection | `Cmd+X` | `Ctrl+X` |
| **Edit** | Copy Selection | `Cmd+C` | `Ctrl+C` |
| **Edit** | Paste | `Cmd+V` | `Ctrl+V` |
| **Edit** | Delete Selected Object | `Backspace` / `Delete` | `Delete` |
| **Navigation** | Command Palette | `Cmd+K` | `Ctrl+K` |
| **Navigation** | Find / Search in Document | `Cmd+F` | `Ctrl+F` |
| **Navigation** | Next / Previous Match | `Enter` / `Shift+Enter` | `Enter` / `Shift+Enter` |
| **View** | Zoom In / Out | `Cmd++` / `Cmd+-` | `Ctrl++` / `Ctrl+-` |
| **View** | Zoom to Fit Page | `Cmd+0` | `Ctrl+0` |
| **View** | Zoom to Fit Width | `Cmd+1` | `Ctrl+1` |
| **Pages** | Next / Previous Page | `PageDown` / `PageUp` | `PageDown` / `PageUp` |
| **Pages** | Jump to First / Last Page | `Cmd+Home` / `Cmd+End` | `Ctrl+Home` / `Ctrl+End` |

### 5.2 Accessibility Compliance
- **Full Focus Traversal:** All interactive controls, inputs, and modal dialogs are reachable via `Tab` and `Shift+Tab`.
- **Screen Reader Support:** Document reading order is linked to `/StructTreeRoot` tag order; non-text visual elements expose `/Alt` text fields.
- **High-Contrast Support:** Complete visual legibility under OS High Contrast mode (Windows High Contrast Black/White, macOS Increase Contrast).
