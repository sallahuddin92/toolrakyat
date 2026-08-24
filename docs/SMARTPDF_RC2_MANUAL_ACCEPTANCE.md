# SmartPDF 1.0 RC2 Manual Acceptance

This document is the human-run acceptance script for SmartPDF 1.0 RC2. Automated qualification does not complete or replace these checks.

## Acceptance status

- **Qualified product SHA:** `003f70f0c0cca83a1373d403d21f1a937c7eb7ea`
- **Overall status:** `HUMAN PENDING`
- **Human tester:** ______________________________
- **Test date:** ______________________________
- **Operating system:** ______________________________
- **Browser and version:** ______________________________
- **External PDF reader used for verification:** ______________________________
- **RC2 tag:** Not created

Do not mark the overall status `ACCEPTED` until every required check below has been performed with real PDFs and its evidence has been recorded. Use non-sensitive documents that may be handled locally; do not commit private PDF contents or personal data.

## Test corpus preparation

Prepare distinct real PDFs containing:

- ordinary Latin text in a standard or commonly embedded font;
- text using a specialized or subset font;
- Arabic and Jawi text, including `ڤ`, `ڠ`, `ڽ`, and `چ`;
- CJK text, preferably separate Japanese, Simplified Chinese, Traditional Chinese, and Korean samples;
- mixed Latin, Arabic, and CJK text in one editable run or nearby runs;
- an intentionally unsafe text target, such as Type3 text, unsupported vertical writing, missing required font mapping, or a layout with dependent downstream text.

Record an anonymized local identifier for every document used. Preserve the original files separately for before/after comparison.

| Corpus ID | Description | Producer/source | Pages | Sensitive data removed? |
| :--- | :--- | :--- | :---: | :---: |
|  |  |  |  |  |
|  |  |  |  |  |
|  |  |  |  |  |

## Required manual checks

### 1. Ordinary Latin text edit

- [ ] Open a real PDF containing editable Latin text.
- [ ] Replace a word with text of similar length.
- [ ] Confirm the intended font, size, baseline, color, and nearby spacing remain visually correct.
- [ ] Confirm no characters become missing-glyph boxes or incorrect substitutions.
- [ ] Export and reopen the result in SmartPDF and an external PDF reader.

Evidence / notes:

________________________________________________________________________________

### 2. Specialized or subset font edit

- [ ] Open a real PDF containing an embedded specialized or subset font.
- [ ] Edit using characters already covered by a compatible document font and confirm document-font reuse where supported.
- [ ] Confirm the exported PDF remains visually correct and searchable after reopen.
- [ ] If the requested characters cannot be represented safely, confirm SmartPDF refuses or uses a qualified bundled fallback without corrupting the document.

Evidence / notes:

________________________________________________________________________________

### 3. Arabic and Jawi

- [ ] Replace text with an Arabic phrase and confirm contextual joining, RTL order, ligatures, and mark placement.
- [ ] Replace text with a Jawi phrase containing `ڤ`, `ڠ`, `ڽ`, and `چ`.
- [ ] Confirm there are no missing or wrong glyphs and the visual reading order is correct.
- [ ] Export, reopen, search for the exact phrase, and copy it into a plain-text editor to verify exact logical Unicode.

Evidence / notes:

________________________________________________________________________________

### 4. CJK

- [ ] Edit a real Japanese PDF and inspect kana and Japanese Han glyph forms.
- [ ] Edit a real Simplified Chinese PDF and inspect region-appropriate glyph forms.
- [ ] Edit a real Traditional Chinese PDF and inspect region-appropriate glyph forms.
- [ ] Edit a real Korean PDF and inspect Hangul and any accompanying Han glyphs.
- [ ] Confirm all exports reopen, render, search, and copy correctly.

Evidence / notes:

________________________________________________________________________________

### 5. Mixed-script edit

- [ ] Replace one target with mixed Latin, Arabic, and CJK text.
- [ ] Confirm font switching is visually coherent and all scripts render in the correct order.
- [ ] Confirm spaces, punctuation, digits, and bidirectional boundaries are correct.
- [ ] Export and reopen; search and copy the complete mixed-script phrase.

Evidence / notes:

________________________________________________________________________________

### 6. Safe refusal

- [ ] Attempt an edit on a target that cannot be rewritten safely.
- [ ] Confirm the operation is refused with a clear user-facing explanation.
- [ ] Confirm no partial mutation occurs and the document remains usable.
- [ ] Export or reopen the unmodified document and confirm it is not corrupt.

Evidence / notes:

________________________________________________________________________________

### 7. Export, reopen, search, and copy

Perform this check for every successful edit above.

- [ ] Export an editable PDF.
- [ ] Reopen it in a new SmartPDF session.
- [ ] Reopen it in an external PDF reader.
- [ ] Search for the exact replacement text in both environments where search is available.
- [ ] Copy the replacement text into a plain-text editor and compare Unicode exactly.
- [ ] Confirm the PDF reports no parse, repair, or corruption warning.

Evidence / notes:

________________________________________________________________________________

### 8. Undo and redo

- [ ] Perform a successful text edit.
- [ ] Undo and confirm the original text and layout return exactly.
- [ ] Redo and confirm the replacement returns exactly.
- [ ] Export after redo, reopen, and confirm the redone state persists.
- [ ] Repeat with at least one multilingual edit.

Evidence / notes:

________________________________________________________________________________

### 9. No unrelated text movement

- [ ] Before editing, capture coordinates or a high-resolution screenshot of nearby unrelated text.
- [ ] Perform each Latin, specialized-font, Arabic/Jawi, CJK, and mixed-script edit.
- [ ] Compare before and after at high zoom.
- [ ] Confirm the edited target remains at the intended anchor unless deliberately moved.
- [ ] Confirm unrelated text has no horizontal or vertical movement, overlap, reflow, or baseline drift.

Evidence / notes:

________________________________________________________________________________

## Defect and blocker log

Any wrong glyph, wrong Unicode, unexpected text movement, unrelated text movement, corrupt export, failed reopen, failed exact search/copy, crash, or silent partial edit is an RC2 release blocker until resolved and requalified.

| ID | Corpus ID | Steps | Expected | Observed | Severity | Resolution |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
|  |  |  |  |  |  |  |

## Anonymized malformed-xref field retest

This automated/private retest records the RC2 blocker fix without including or identifying the source PDF. It does not replace the human checks above and does not change the overall `HUMAN PENDING` status.

| Property | Result |
| :--- | :--- |
| File size | 5,435,222 bytes |
| Xref layout | Classic xref; linearized-style current section plus later classic section |
| `startxref` | `173` |
| Malformed `/Prev` | `5433294` (forward of the current xref) |
| StarPDF recovery | PASS — `RECOVERED_MALFORMED_PREV`, 26 pages resolved |
| Text extraction | PASS across all 26 pages |
| Safe mutation | PASS — FreeText text annotation added; the document's existing specialized fonts remained independently subject to normal editability checks |
| Export | PASS — clean terminal xref emitted without a terminal `/Prev` |
| StarPDF reopen | PASS — output reports `VALID`, not recovered |
| Text verification | PASS — added annotation text recovered exactly after reopen |
| PDF.js rendering | PASS before and after export in Chromium |
| Source PDF committed | NO |

## Human sign-off

- [ ] All required checks completed with real PDFs.
- [ ] Wrong glyphs observed: `0`
- [ ] Wrong logical Unicode results observed: `0`
- [ ] Wrong target text movements observed: `0`
- [ ] Unrelated text movements observed: `0`
- [ ] Corrupt exports observed: `0`
- [ ] All release-blocking findings resolved and requalified.

**Decision:** `HUMAN PENDING`

Human release owner: ______________________________

Signature / approval reference: ______________________________

Date: ______________________________

Only a human release owner may change the decision to `ACCEPTED`. RC2 remains untagged until that approval is recorded.
