use super::core::TableShape;
use crate::ast::{BinOp, Expr, UnOp};
use std::collections::BTreeMap;

pub fn const_key_value(e: &Expr) -> Option<i64> {
    match e {
        Expr::Integer { value: i, .. } => Some(*i),
        Expr::BinaryOp {
            op, left, right, ..
        } => {
            let (l, r) = (const_key_value(left)?, const_key_value(right)?);
            match op {
                BinOp::Add => l.checked_add(r),
                BinOp::Sub => l.checked_sub(r),
                BinOp::Mul => l.checked_mul(r),
                _ => None,
            }
        }
        Expr::UnaryOp {
            op: UnOp::Neg,
            expr,
            ..
        } => const_key_value(expr).and_then(|v| v.checked_neg()),
        _ => None,
    }
}

pub fn extract_guard(condition: &Expr) -> Option<(&str, &Expr)> {
    if let Expr::BinaryOp {
        op: BinOp::LessThan,
        left,
        right,
        ..
    } = condition
    {
        if let Expr::Identifier { name, .. } = left.as_ref() {
            Some((name.as_str(), right))
        } else {
            None
        }
    } else {
        None
    }
}

/// The key's relation to a loop guard: the guard name itself, or the
/// name plus or minus a constant literal (both operand orders for the
/// sum; a constant minus the guard is not invertible and declines).
/// `None` is every other shape.
pub fn guard_key_shape(key: &Expr, guard: &str) -> Option<crate::shape::KeyShape> {
    use crate::shape::KeyShape;
    let literal = |e: &Expr| match e {
        Expr::Integer { value, .. } => Some(*value),
        _ => None,
    };
    let is_guard = |e: &Expr| matches!(e, Expr::Identifier { name, .. } if name == guard);
    match key {
        Expr::Identifier { name, .. } if name == guard => Some(KeyShape::Guard),
        Expr::BinaryOp {
            op: BinOp::Add,
            left,
            right,
            ..
        } => {
            if is_guard(left) {
                literal(right).map(KeyShape::GuardOffset)
            } else {
                literal(left)
                    .filter(|_| is_guard(right))
                    .map(KeyShape::GuardOffset)
            }
        }
        Expr::BinaryOp {
            op: BinOp::Sub,
            left,
            right,
            ..
        } => {
            if is_guard(left) {
                literal(right).and_then(|k| k.checked_neg().map(KeyShape::GuardOffset))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn merge_table_scopes(
    into: &mut [BTreeMap<String, TableShape>],
    a: &[BTreeMap<String, TableShape>],
    b: &[BTreeMap<String, TableShape>],
) {
    for ((into_slot, a_slot), b_slot) in into.iter_mut().zip(a).zip(b) {
        for (name, val) in into_slot.iter_mut() {
            if let Some(x) = a_slot.get(name) {
                val.join(x);
            }
            if let Some(x) = b_slot.get(name) {
                val.join(x);
            }
        }
    }
}
