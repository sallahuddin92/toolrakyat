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
/// Supports bounded dependency closures when all dependent text spans within
/// a text matrix block are included in the target set.
pub fn plan_move_edits(
    instructions: &[ContentInstruction],
    targets: &[&TextEditTarget],
    dx: f64,
    dy: f64,
) -> PdfResult<Vec<TextMoveEdit>> {
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return Err(PdfError::InvalidOperation("TEXT_MOVE_NO_OP_DELTA".into()));
    }
    let target_indices: BTreeSet<usize> = targets.iter().map(|t| t.instruction_index).collect();
    let mut edits: Vec<TextMoveEdit> = Vec::with_capacity(targets.len());
    let mut used_positioners: BTreeSet<usize> = BTreeSet::new();
    for target in targets {
        let edit = classify_text_move_with_context(instructions, target, dx, dy, &target_indices)?;
        if used_positioners.insert(edit.positioner_index) {
            edits.push(edit);
        }
    }
    Ok(edits)
}

/// Determines whether the given span can be moved by exactly `(dx, dy)` in user space
/// and computes the required positioning-operand adjustments.
pub fn classify_text_move(
    instructions: &[ContentInstruction],
    target: &TextEditTarget,
    dx: f64,
    dy: f64,
) -> PdfResult<TextMoveEdit> {
    let mut target_indices = BTreeSet::new();
    target_indices.insert(target.instruction_index);
    classify_text_move_with_context(instructions, target, dx, dy, &target_indices)
}

/// Context-aware text move classifier that checks if dependent spans are part of a target closure.
pub fn classify_text_move_with_context(
    instructions: &[ContentInstruction],
    target: &TextEditTarget,
    dx: f64,
    dy: f64,
    target_indices: &BTreeSet<usize>,
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
                }
                prior_positioning_in_block = true;
            }
            ContentOperator::TStar => {
                // T* moves by current leading; treat as relative positioning that depends
                // on surrounding text state (leading), so it can never be a standalone safe positioner.
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
                if positioner_index.is_some() && !target_indices.contains(&idx) {
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
    for (offset, instr) in instructions[(target.instruction_index + 1)..]
        .iter()
        .enumerate()
    {
        let downstream_idx = target.instruction_index + 1 + offset;
        match instr.operator {
            ContentOperator::Et | ContentOperator::Bt | ContentOperator::Tm => break,
            ContentOperator::Tj
            | ContentOperator::TJ
            | ContentOperator::Quote
            | ContentOperator::DoubleQuote => {
                if !target_indices.contains(&downstream_idx) {
                    return Err(PdfError::InvalidOperation(
                        "TEXT_MOVE_DOWNSTREAM_DEPENDENT_TEXT".into(),
                    ));
                }
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
    fn test_a_independent_tm_text_move_succeeds() {
        let stream = b"BT\n/F1 12 Tf\n12 0 0 12 100 200 Tm\n(Standalone) Tj\nET\n";
        let instructions = parse(stream);
        let target = TextEditTarget::new(0, 0, 3, 0);
        let edit = classify_text_move(&instructions, &target, 15.0, -10.0).unwrap();
        assert_eq!(edit.positioner_index, 2);
        assert_eq!((edit.delta_tx, edit.delta_ty), (15.0, -10.0));

        let mut mutable_instructions = instructions.clone();
        apply_move_edits(&mut mutable_instructions, &[edit]).unwrap();
        assert_eq!(mutable_instructions[2].operands[4].as_f64(), Some(115.0));
        assert_eq!(mutable_instructions[2].operands[5].as_f64(), Some(190.0));
    }

    #[test]
    fn test_b_downstream_dependent_text_safe_refusal() {
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
    fn test_c_bounded_dependent_group_entire_closure_moves_rigidly() {
        let stream = b"BT\n/F1 12 Tf\n12 0 0 12 72 720 Tm\n(A) Tj\n[(B)] TJ\nET\n";
        let t1 = TextEditTarget::new(0, 0, 3, 0);
        let t2 = TextEditTarget::new(0, 0, 4, 0);
        let instructions = parse(stream);

        // Moving both spans in the closure succeeds with exactly one positioner adjustment
        let edits = plan_move_edits(&instructions, &[&t1, &t2], 10.0, 20.0).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].positioner_index, 2);
        assert_eq!((edits[0].delta_tx, edits[0].delta_ty), (10.0, 20.0));

        let mut mutable_instructions = instructions.clone();
        apply_move_edits(&mut mutable_instructions, &edits).unwrap();
        assert_eq!(mutable_instructions[2].operands[4].as_f64(), Some(82.0));
        assert_eq!(mutable_instructions[2].operands[5].as_f64(), Some(740.0));
    }

    #[test]
    fn test_d_downstream_reset_by_explicit_tm_succeeds_independently() {
        let stream = b"BT\n/F1 12 Tf\n12 0 0 12 100 200 Tm\n(Title) Tj\n12 0 0 12 100 150 Tm\n(Body) Tj\nET\n";
        let instructions = parse(stream);
        let target_title = TextEditTarget::new(0, 0, 3, 0);

        // Moving Title alone succeeds because Body is reset by explicit Tm
        let edit = classify_text_move(&instructions, &target_title, 20.0, 10.0).unwrap();
        assert_eq!(edit.positioner_index, 2);

        let mut mutable_instructions = instructions.clone();
        apply_move_edits(&mut mutable_instructions, &[edit]).unwrap();
        // Title moved
        assert_eq!(mutable_instructions[2].operands[4].as_f64(), Some(120.0));
        assert_eq!(mutable_instructions[2].operands[5].as_f64(), Some(210.0));
        // Body unchanged
        assert_eq!(mutable_instructions[4].operands[4].as_f64(), Some(100.0));
        assert_eq!(mutable_instructions[4].operands[5].as_f64(), Some(150.0));
    }

    #[test]
    fn test_e_tj_array_dependency_classification() {
        // Multi-string TJ array: moving individual string inside TJ is refused
        let stream = b"BT\n/F1 12 Tf\n12 0 0 12 100 200 Tm\n[(Hello) -120 (World)] TJ\nET\n";
        let instructions = parse(stream);
        let target_hello = TextEditTarget::new(0, 0, 3, 0);
        let target_world = TextEditTarget::new(0, 0, 3, 2);

        assert!(classify_text_move(&instructions, &target_hello, 5.0, 5.0).is_err());
        assert!(classify_text_move(&instructions, &target_world, 5.0, 5.0).is_err());
    }

    #[test]
    fn test_f_td_td_tstar_dependencies_classification() {
        let stream = b"BT\n50 700 Td\n(Line A) Tj\n0 -20 Td\n(Line B) Tj\nET\n";
        let instructions = parse(stream);
        let target_a = TextEditTarget::new(0, 0, 2, 0);
        let target_b = TextEditTarget::new(0, 0, 4, 0);

        // Line A alone fails due to downstream dependent Line B
        assert!(classify_text_move(&instructions, &target_a, 5.0, 5.0).is_err());

        // Line B alone fails due to relative accumulation on Line A
        assert!(classify_text_move(&instructions, &target_b, 5.0, 5.0).is_err());

        // Both together form a closure and move rigidly
        let edits = plan_move_edits(&instructions, &[&target_a, &target_b], 15.0, 10.0).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].positioner_index, 1);
    }

    #[test]
    fn test_g_rotated_ctm_compensates_correctly() {
        // 90-degree counter-clockwise rotation CTM: [0 1 -1 0 0 0]
        let stream = b"0 1 -1 0 0 0 cm\nBT\n12 0 0 12 0 0 Tm\n(Rotated) Tj\nET\n";
        let instructions = parse(stream);
        let target = TextEditTarget::new(0, 0, 3, 0);
        let edit = classify_text_move(&instructions, &target, 10.0, 20.0).unwrap();
        // inverse([0 1; -1 0]) * (10, 20) = (20, -10)
        assert!((edit.delta_tx - 20.0).abs() < 1e-6);
        assert!((edit.delta_ty - (-10.0)).abs() < 1e-6);
    }

    #[test]
    fn test_h_failed_move_leaves_instructions_and_plan_empty() {
        let stream = b"BT\n12 0 0 12 72 720 Tm\n(A) Tj\n(B) Tj\nET\n";
        let instructions = parse(stream);
        let target = TextEditTarget::new(0, 0, 2, 0);
        let res = plan_move_edits(&instructions, &[&target], 10.0, 10.0);
        assert!(res.is_err());
    }
}
