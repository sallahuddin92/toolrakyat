use std::collections::BTreeSet;

use crate::content::{ContentInstruction, ContentOperand, ContentOperator, ContentParser};
use crate::error::{PdfError, PdfResult};
use crate::mutation::text_edit::TextEditTarget;
use crate::text::matrix::Matrix2D;

/// A single positioning-operand adjustment that moves one text span by a delta
/// expressed in default user space (page coordinates, bottom-left origin).
#[derive(Debug, Clone, PartialEq)]
pub struct TextMoveEdit {
    /// Instruction index of the positioning operator (Tm or Td) to adjust.
    pub positioner_index: usize,
    /// Operand index of the horizontal translation component.
    pub tx_operand_index: usize,
    /// Operand index of the vertical translation component.
    pub ty_operand_index: usize,
    /// Exact operand deltas (already compensated for CTM / line-matrix transforms).
    pub delta_tx: f64,
    pub delta_ty: f64,
}

/// Applies computed operand deltas to parsed instructions.
pub fn apply_move_edits(
    instructions: &mut [ContentInstruction],
    edits: &[TextMoveEdit],
) -> PdfResult<()> {
    for edit in edits {
        let instr = instructions.get_mut(edit.positioner_index).ok_or_else(|| {
            PdfError::InvalidOperation(format!(
                "Move positioner index {} out of bounds",
                edit.positioner_index
            ))
        })?;
        match instr.operator {
            ContentOperator::Tm | ContentOperator::Td => {}
            ref op => {
                return Err(PdfError::InvalidOperation(format!(
                    "Positioner instruction is '{op}', expected Tm or Td"
                )));
            }
        }
        let tx_idx = edit.tx_operand_index;
        let ty_idx = edit.ty_operand_index;
        if instr.operands.len() <= tx_idx.max(ty_idx) {
            return Err(PdfError::InvalidOperation(
                "Move positioner is missing translation operands".into(),
            ));
        }
        let new_tx = instr.operands[tx_idx].as_f64().unwrap_or(0.0) + edit.delta_tx;
        let new_ty = instr.operands[ty_idx].as_f64().unwrap_or(0.0) + edit.delta_ty;
        if !new_tx.is_finite() || !new_ty.is_finite() {
            return Err(PdfError::UnsupportedLayout(
                "Computed move position would produce non-finite coordinates".into(),
            ));
        }
        instr.operands[tx_idx] = ContentOperand::Real(new_tx);
        instr.operands[ty_idx] = ContentOperand::Real(new_ty);
    }
    Ok(())
}

/// Parses raw content-stream bytes and plans an atomic multi-span move.
/// All spans must be individually safe AND must not share a single positioner.
pub fn plan_move_edits_from_bytes(
    stream_bytes: &[u8],
    targets: &[&TextEditTarget],
    dx: f64,
    dy: f64,
) -> PdfResult<Vec<TextMoveEdit>> {
    let mut parser = ContentParser::from_bytes(stream_bytes);
    let instructions = parser.parse_instructions()?;
    plan_move_edits(&instructions, targets, dx, dy)
}

/// Plans an atomic multi-span move over already-parsed instructions.
pub fn plan_move_edits(
    instructions: &[ContentInstruction],
    targets: &[&TextEditTarget],
    dx: f64,
    dy: f64,
) -> PdfResult<Vec<TextMoveEdit>> {
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return Err(PdfError::InvalidOperation("TEXT_MOVE_NO_OP_DELTA".into()));
    }
    let mut edits: Vec<TextMoveEdit> = Vec::with_capacity(targets.len());
    let mut used_positioners: BTreeSet<usize> = BTreeSet::new();
    for target in targets {
        let edit = classify_text_move(instructions, target, dx, dy)?;
        if !used_positioners.insert(edit.positioner_index) {
            return Err(PdfError::InvalidOperation(
                "TEXT_MOVE_SHARED_POSITIONER".into(),
            ));
        }
        edits.push(edit);
    }
    Ok(edits)
}

/// Determines whether the given span can be moved by exactly `(dx, dy)` in user space
/// and computes the required positioning-operand adjustments.
///
/// Safety envelope (v0.21):
/// - The span's show operator must be `Tj`, or `TJ` with a string operand at `operand_index`.
/// - The span must have a dedicated positioning operator (`Tm`, or the first-in-block
///   `Td`/`TD`/`T*`) with no other show operators sharing it, and only text-state
///   operators between the positioner and the show operator.
/// - No downstream text may depend on the moved text-line origin before the next
///   repositioning / block boundary (`Tm`, `Td`, `TD`, `T*`, `BT`, `ET`).
/// - A relative positioner is accepted only when it is the FIRST positioning operator of
///   its `BT..ET` block; otherwise the position depends on accumulated surrounding state.
pub fn classify_text_move(
    instructions: &[ContentInstruction],
    target: &TextEditTarget,
    dx: f64,
    dy: f64,
) -> PdfResult<TextMoveEdit> {
    if target.instruction_index >= instructions.len() {
        return Err(PdfError::TargetTextNotFound(format!(
            "Instruction index {} out of bounds (stream has {} instructions)",
            target.instruction_index,
            instructions.len()
        )));
    }

    verify_show_target(instructions, target)?;

    // Replay graphics + text positioning state up to (not including) the target instruction.
    let mut graphics_stack: Vec<Matrix2D> = Vec::new();
    let mut ctm = Matrix2D::identity();
    let mut line_matrix = Matrix2D::identity();
    let mut prev_line_matrix = Matrix2D::identity();

    let mut in_text_block = false;
    let mut positioner_index: Option<usize> = None;
    let mut positioner_is_tm = false;
    let mut prior_positioning_in_block = false;
    let mut show_after_positioner = false;
    let mut foreign_op_after_positioner = false;

    for (idx, instr) in instructions.iter().enumerate() {
        if idx == target.instruction_index {
            break;
        }
        match instr.operator {
            ContentOperator::Q => graphics_stack.push(ctm),
            ContentOperator::QEnd => {
                ctm = graphics_stack.pop().unwrap_or_default();
            }
            ContentOperator::Cm if instr.operands.len() == 6 => {
                ctm = operands_to_matrix(instr).multiply(&ctm);
            }
            ContentOperator::Bt => {
                in_text_block = true;
                positioner_index = None;
                show_after_positioner = false;
                foreign_op_after_positioner = false;
                prior_positioning_in_block = false;
                line_matrix = Matrix2D::identity();
            }
            ContentOperator::Et => {
                in_text_block = false;
                positioner_index = None;
            }
            ContentOperator::Tm if instr.operands.len() == 6 => {
                prev_line_matrix = line_matrix;
                line_matrix = operands_to_matrix(instr);
                begin_positioner(
                    idx,
                    true,
                    in_text_block,
                    &mut positioner_index,
                    &mut positioner_is_tm,
                    &mut show_after_positioner,
                    &mut foreign_op_after_positioner,
                );
                prior_positioning_in_block = true;
            }
            ContentOperator::Td | ContentOperator::TD if instr.operands.len() >= 2 => {
                let tx = instr.operands[0].as_f64().unwrap_or(0.0);
                let ty = instr.operands[1].as_f64().unwrap_or(0.0);
                prev_line_matrix = line_matrix;
                line_matrix = Matrix2D::translation(tx, ty).multiply(&line_matrix);
                if !prior_positioning_in_block {
                    begin_positioner(
                        idx,
                        false,
                        in_text_block,
                        &mut positioner_index,
                        &mut positioner_is_tm,
                        &mut show_after_positioner,
                        &mut foreign_op_after_positioner,
                    );
                } else if positioner_index == Some(idx) {
                    // unreachable, kept for clarity
                }
                prior_positioning_in_block = true;
            }
            ContentOperator::TStar => {
                // T* moves by current leading; treat as relative positioning that depends
                // on surrounding text state (leading), so it can never be a safe positioner.
                prev_line_matrix = line_matrix;
                line_matrix = Matrix2D::translation(0.0, -24.0).multiply(&line_matrix);
                prior_positioning_in_block = true;
                if positioner_index.is_some() {
                    foreign_op_after_positioner = true;
                }
            }
            ContentOperator::Tj
            | ContentOperator::TJ
            | ContentOperator::Quote
            | ContentOperator::DoubleQuote => {
                if positioner_index.is_some() {
                    show_after_positioner = true;
                }
            }
            _ => {
                if positioner_index.is_some() && !is_text_state_operator(&instr.operator) {
                    foreign_op_after_positioner = true;
                }
            }
        }
    }

    if !in_text_block {
        return Err(PdfError::InvalidOperation(
            "TEXT_MOVE_OUTSIDE_TEXT_BLOCK".into(),
        ));
    }
    let Some(positioner_index_val) = positioner_index else {
        return Err(PdfError::InvalidOperation(
            "TEXT_MOVE_NO_EXPLICIT_POSITIONER".into(),
        ));
    };
    if show_after_positioner {
        return Err(PdfError::InvalidOperation(
            "TEXT_MOVE_SHARED_POSITIONER".into(),
        ));
    }
    if foreign_op_after_positioner {
        return Err(PdfError::InvalidOperation(
            "TEXT_MOVE_STATE_DEPENDENT".into(),
        ));
    }

    // Downstream dependency check: text shown before the next absolute repositioning/block boundary
    // rides on the moved text-line origin and would shift with it.
    for instr in &instructions[(target.instruction_index + 1)..] {
        match instr.operator {
            ContentOperator::Et | ContentOperator::Bt | ContentOperator::Tm => break,
            ContentOperator::Tj
            | ContentOperator::TJ
            | ContentOperator::Quote
            | ContentOperator::DoubleQuote => {
                return Err(PdfError::InvalidOperation(
                    "TEXT_MOVE_DOWNSTREAM_DEPENDENT_TEXT".into(),
                ));
            }
            _ => {}
        }
    }

    let (tx_operand_index, ty_operand_index, delta_tx, delta_ty) = if positioner_is_tm {
        // Rendered origin translation = CTM_linear * (e, f): adjust e/f via CTM inverse.
        let inv_ctm = inverse_linear(&ctm).ok_or_else(singular_transform_error)?;
        let d = inv_ctm.transform_point(dx, dy);
        (4usize, 5usize, d.0, d.1)
    } else {
        // Td displacement passes through previous line matrix linear part, then CTM:
        // solve (CTM_lin * LM_prev_lin) * delta = (dx, dy).
        let combined = ctm.multiply(&prev_line_matrix);
        let inv_combined = inverse_linear(&combined).ok_or_else(singular_transform_error)?;
        let p = inv_combined.transform_point(dx, dy);
        (0usize, 1usize, p.0, p.1)
    };

    Ok(TextMoveEdit {
        positioner_index: positioner_index_val,
        tx_operand_index,
        ty_operand_index,
        delta_tx,
        delta_ty,
    })
}

fn begin_positioner(
    idx: usize,
    is_tm: bool,
    in_text_block: bool,
    positioner_index: &mut Option<usize>,
    positioner_is_tm: &mut bool,
    show_after_positioner: &mut bool,
    foreign_op_after_positioner: &mut bool,
) {
    if !in_text_block {
        return;
    }
    *positioner_index = Some(idx);
    *positioner_is_tm = is_tm;
    *show_after_positioner = false;
    *foreign_op_after_positioner = false;
}

fn verify_show_target(
    instructions: &[ContentInstruction],
    target: &TextEditTarget,
) -> PdfResult<()> {
    let target_instr = &instructions[target.instruction_index];
    match target_instr.operator {
        ContentOperator::Tj => {
            if target.operand_index != 0 || target_instr.operands.is_empty() {
                return Err(positioner_unsupported());
            }
        }
        ContentOperator::TJ => {
            let Some(arr) = target_instr
                .operands
                .first()
                .and_then(ContentOperand::as_array)
            else {
                return Err(positioner_unsupported());
            };
            let Some(item) = arr.get(target.operand_index) else {
                return Err(positioner_unsupported());
            };
            if !matches!(item, ContentOperand::String(_)) {
                return Err(positioner_unsupported());
            }
            let string_count = arr
                .iter()
                .filter(|it| matches!(it, ContentOperand::String(_)))
                .count();
            if string_count > 1 {
                return Err(PdfError::InvalidOperation(
                    "TEXT_MOVE_SHARED_POSITIONER".into(),
                ));
            }
        }
        _ => return Err(positioner_unsupported()),
    }
    Ok(())
}

fn positioner_unsupported() -> PdfError {
    PdfError::InvalidOperation("TEXT_MOVE_UNSUPPORTED_POSITIONER".into())
}

fn singular_transform_error() -> PdfError {
    PdfError::InvalidOperation("TEXT_MOVE_SINGULAR_TRANSFORM".into())
}

fn inverse_linear(m: &Matrix2D) -> Option<Matrix2D> {
    let det = m.a * m.d - m.b * m.c;
    if det.abs() < 1e-12 || !det.is_finite() {
        return None;
    }
    Some(Matrix2D::new(
        m.d / det,
        -m.b / det,
        -m.c / det,
        m.a / det,
        0.0,
        0.0,
    ))
}

fn operands_to_matrix(instr: &ContentInstruction) -> Matrix2D {
    Matrix2D::new(
        instr
            .operands
            .first()
            .and_then(|o| o.as_f64())
            .unwrap_or(1.0),
        instr
            .operands
            .get(1)
            .and_then(|o| o.as_f64())
            .unwrap_or(0.0),
        instr
            .operands
            .get(2)
            .and_then(|o| o.as_f64())
            .unwrap_or(0.0),
        instr
            .operands
            .get(3)
            .and_then(|o| o.as_f64())
            .unwrap_or(1.0),
        instr
            .operands
            .get(4)
            .and_then(|o| o.as_f64())
            .unwrap_or(0.0),
        instr
            .operands
            .get(5)
            .and_then(|o| o.as_f64())
            .unwrap_or(0.0),
    )
}

fn is_text_state_operator(op: &ContentOperator) -> bool {
    matches!(
        op,
        ContentOperator::Tf
            | ContentOperator::Tc
            | ContentOperator::Tw
            | ContentOperator::Tz
            | ContentOperator::TL
            | ContentOperator::Ts
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ContentParser;

    fn parse(stream: &[u8]) -> Vec<ContentInstruction> {
        let mut parser = ContentParser::from_bytes(stream);
        parser.parse_instructions().unwrap()
    }

    #[test]
    fn classifies_simple_tm_positioned_span_as_safe() {
        let stream =
            b"q\n1 0 0 1 50 700 cm\nBT\n/F1 12 Tf\n12 0 0 12 72 720 Tm\n(Hello) Tj\nET\nQ\n";
        let instructions = parse(stream);
        let target = TextEditTarget::new(0, 0, 5, 0);
        let edit = classify_text_move(&instructions, &target, 10.0, -5.0).unwrap();
        assert_eq!(edit.positioner_index, 4);
        assert_eq!((edit.tx_operand_index, edit.ty_operand_index), (4, 5));
        assert_eq!(
            (edit.delta_tx, edit.delta_ty),
            (10.0, -5.0),
            "identity CTM passes deltas through unchanged"
        );
    }

    #[test]
    fn compensates_cm_scale_for_tm_move() {
        let stream = b"2 0 0 2 100 100 cm\nBT\n12 0 0 12 0 0 Tm\n(Hi) Tj\nET\n";
        let instructions = parse(stream);
        let target = TextEditTarget::new(0, 0, 3, 0);
        let edit = classify_text_move(&instructions, &target, 20.0, 10.0).unwrap();
        assert_eq!((edit.delta_tx, edit.delta_ty), (10.0, 5.0));
    }

    #[test]
    fn classifies_first_td_in_block_as_safe() {
        let stream = b"BT\n/F1 12 Tf\n72 720 Td\n(Hello) Tj\nET\n";
        let instructions = parse(stream);
        let target = TextEditTarget::new(0, 0, 3, 0);
        let edit = classify_text_move(&instructions, &target, 5.0, 5.0).unwrap();
        assert_eq!(edit.positioner_index, 2);
        assert_eq!((edit.tx_operand_index, edit.ty_operand_index), (0, 1));
    }

    #[test]
    fn refuses_td_that_depends_on_prior_positioning() {
        // Second Td accumulates on top of the first one's line matrix.
        let stream = b"BT\n50 700 Td\n(Line A) Tj\n0 20 Td\n(Line B) Tj\nET\n";
        let instructions = parse(stream);
        let target_a = TextEditTarget::new(0, 0, 2, 0);
        let target_b = TextEditTarget::new(0, 0, 4, 0);

        // Span B depends on accumulated state -> refusal.
        assert!(classify_text_move(&instructions, &target_b, 5.0, 5.0).is_err());

        // Span A has downstream dependent text (Line B) -> refusal.
        assert!(classify_text_move(&instructions, &target_a, 5.0, 5.0).is_err());
    }

    #[test]
    fn refuses_shared_positioner_between_two_spans() {
        let stream = b"BT\n/F1 12 Tf\n12 0 0 12 72 720 Tm\n(A) Tj\n[(B)] TJ\nET\n";
        let instructions = parse(stream);
        let target_b = TextEditTarget::new(0, 0, 4, 0);
        let err = classify_text_move(&instructions, &target_b, 5.0, 5.0)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(err.contains("TEXT_MOVE_SHARED_POSITIONER"), "{err}");
    }

    #[test]
    fn refuses_downstream_dependent_text() {
        let stream = b"BT\n12 0 0 12 72 720 Tm\n(A) Tj\n(B) Tj\nET\n";
        let instructions = parse(stream);
        let target = TextEditTarget::new(0, 0, 2, 0);
        let err = classify_text_move(&instructions, &target, 5.0, 5.0)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(err.contains("TEXT_MOVE_DOWNSTREAM_DEPENDENT_TEXT"), "{err}");
    }

    #[test]
    fn refuses_no_explicit_positioner() {
        // Position derived entirely from TJ kerning accumulation after BT with no Tm/Td.
        let stream = b"BT\n/F1 12 Tf\n(A) Tj\nET\n";
        let instructions = parse(stream);
        let target = TextEditTarget::new(0, 0, 2, 0);
        assert!(classify_text_move(&instructions, &target, 5.0, 5.0).is_err());
    }

    #[test]
    fn plan_rejects_two_spans_sharing_one_positioner_atomically() {
        let stream = b"BT\n/F1 12 Tf\n12 0 0 12 72 720 Tm\n(A) Tj\n[(B)] TJ\nET\n";
        let t1 = TextEditTarget::new(0, 0, 3, 0);
        let t2 = TextEditTarget::new(0, 0, 4, 0);
        // Span B shares the positioner -> whole plan fails atomically.
        let result = plan_move_edits_from_bytes(stream, &[&t1, &t2], 5.0, 5.0);
        assert!(result.is_err());
    }

    #[test]
    fn apply_move_edits_updates_operands() {
        let stream = b"BT\n12 0 0 12 72 720 Tm\n(Hello) Tj\nET\n";
        let target = TextEditTarget::new(0, 0, 2, 0);
        let instructions = parse(stream);
        let edit = classify_text_move(&instructions, &target, 10.25, -3.5).unwrap();
        let mut mutable_instructions = instructions.clone();
        apply_move_edits(&mut mutable_instructions, &[edit]).unwrap();
        let tm = &mutable_instructions[1];
        assert_eq!(tm.operands[4].as_f64(), Some(82.25));
        assert_eq!(tm.operands[5].as_f64(), Some(716.5));
    }
}
