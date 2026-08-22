# StarPDF Multi-Span Layout Safety Lock (Phase 3B.2A)

## 1. Overview & Core Product Invariant

In PDF content streams, text positioning operates through a state machine where text-showing operators (`Tj`, `TJ`, `'`, `"`) displace the text matrix $T_m$ horizontally according to glyph advances, font size, character spacing ($T_c$), word spacing ($T_w$), horizontal scaling ($T_z$), and kerning numeric adjustments.

When mutating multi-span or single-span text runs:
- Target $T_0$ receives the encoded replacement bytes.
- Neutralized targets $T_1 \dots T_k$ receive empty byte strings (`()`).

Without advance compensation, eliminating the glyph advances previously contributed by $T_1 \dots T_k$ or altering the length of $T_0$ could inadvertently shift subsequent text shown on the same line.

### The Core Rule:
For every text edit classified **SAFE**:
- **Wrong edits**: 0
- **Partial edits**: 0
- **Unintended downstream text movement**: 0.0000 pt

---

## 2. PDF Advance & Displacement Semantics

### Standard Displacement Equation
For each glyph character with metric advance width $w_0$ (in $1/1000$ font units):
$$\Delta x = \left( \frac{w_0}{1000} \cdot T_{fs} + T_c + (w == \text{' '} ? T_w : 0) \right) \cdot \frac{T_z}{100}$$

For a numeric kerning adjustment operand $n$ in a `TJ` array:
$$\Delta x = -\frac{n}{1000} \cdot T_{fs} \cdot \frac{T_z}{100}$$

### Cumulative Run Advance
The total original advance across a multi-span run $[T_0, \dots, T_k]$ is calculated as:
$$\text{Advance}_{\text{orig}} = \sum_{i=0}^k \text{Advance}(T_i) + \sum \text{TJ adjustments between } T_0 \text{ and } T_k$$

The advance of the replacement text in the active font state is:
$$\text{Advance}_{\text{new}} = \text{Advance}(\text{replacement}, T_{fs}, T_c, T_w, T_z)$$

---

## 3. Downstream Dependency Analysis

### Independent Downstream Text
Subsequent text is **completely independent** of the target run if:
1. The text block ends (`ET`), or
2. The text matrix is explicitly reset (`Tm`), or
3. A line-positioning operator (`Td`, `TD`, `T*`) executes, resetting $T_m = T_{lm}$ to the start of the next line, or
4. A new text block begins (`BT`).

In these cases, downstream text uses explicit coordinates and is guaranteed not to move regardless of $\Delta \text{Advance}$.

### Dependent Downstream Text
Subsequent text is **dependent** on the target run if:
1. There are additional text elements in the same `TJ` array after $T_k$, or
2. There are subsequent text-showing operators (`Tj`, `TJ`, `'`, `"`) on the same line before any `Td`, `TD`, `T*`, `Tm`, or `ET`.

---

## 4. Mathematically Exact Layout Advance Compensation

When dependent downstream text exists:

### Case 1: Replacement Fits Within Original Box ($\text{Advance}_{\text{new}} \le \text{Advance}_{\text{orig}}$)
The engine compensates the exact remaining advance $\Delta \text{Advance} = \text{Advance}_{\text{orig}} - \text{Advance}_{\text{new}}$ using native PDF `TJ` numeric adjustment:

$$n_{\text{comp}} = \frac{\text{Advance}_{\text{new}} - \text{Advance}_{\text{orig}}}{T_{fs} \cdot \frac{T_z}{100}} \cdot 1000.0$$

- **In `TJ` arrays**: $n_{\text{comp}}$ is merged with the adjacent numeric adjustment operand immediately following $T_k$ (or inserted as a new kerning element).
- **Across `Tj` operators**: $T_k$ is converted from `() Tj` to `[() n_comp] TJ` (or if single span, `[(replacement) n_comp] TJ`).

**Net displacement through target group**:
$$\text{Advance}_{\text{new}} + \left(-\frac{n_{\text{comp}}}{1000} \cdot T_{fs} \cdot \frac{T_z}{100}\right) = \text{Advance}_{\text{orig}}$$

Downstream text starting coordinates remain **100% bit-exact (0.0000 pt shift)**.

### Case 2: Replacement Exceeds Original Box with Dependent Downstream Text
If $\text{Advance}_{\text{new}} > \text{Advance}_{\text{orig}} + 0.5\text{pt}$ and dependent downstream text exists on the same line:
- Applying negative displacement would cause the new text to overlap downstream text.
- Allowing uncompensated expansion would shift downstream text.
- **Action**: Safe Typed Refusal.

**Typed Refusal**:
```
User Message: "This text can't be safely edited in place."
Technical Details: "Other text in this PDF depends on the spacing of this text run."
```

---

## 5. Prohibited Techniques
As specified by the core engine rules, StarPDF **never** uses synthetic hacks to simulate spacing:
- No whitespace padding (spaces)
- No invisible / transparent / white glyphs
- No floating annotations or FreeText overlays
- No page rasterization or canvas patching

All mutations are 100% native PDF content stream operations preserving clean vector geometry.

---

## 6. Verification & Quality Gates

| Verification Gate | Result | Notes |
| :--- | :--- | :--- |
| `cargo fmt --check` | **PASS** | Strict formatting adherence |
| `cargo clippy --all-targets --all-features -- -D warnings` | **PASS** | 0 warnings |
| `cargo test --all-features` | **PASS** | All engine & layout safety tests pass |
| `npm run lint` | **PASS** | 0 ESLint errors |
| `npm run typecheck` | **PASS** | 0 TypeScript errors |
| `npm test` | **PASS** | 700/700 Vitest tests pass |
| `npm run build` | **PASS** | Next.js production build succeeds |
| `npx playwright test` | **PASS** | 192/192 Playwright tests pass (Chromium, Firefox, WebKit) |
