use crate::{
    error::{MathvizError, MathvizResult},
    evaluator::constants::lookup_constant,
    types::{ASTNode, BinaryOp, NaryOp, UnaryOp},
};

const MAX_AST_DEPTH: usize = 64;

#[inline(always)]
pub fn eval_ast(node: &ASTNode, bindings: &[(&str, f64)], allow_non_finite: bool) -> MathvizResult<f64> {
    eval_ast_inner(node, bindings, allow_non_finite, 0)
}

#[inline(always)]
fn eval_ast_inner(
    node: &ASTNode,
    bindings: &[(&str, f64)],
    allow_non_finite: bool,
    depth: usize,
) -> MathvizResult<f64> {
    if depth > MAX_AST_DEPTH {
        return Err(MathvizError::EvalError(format!(
            "maximum AST depth exceeded ({MAX_AST_DEPTH})"
        )));
    }

    let value = match node {
        ASTNode::Literal { value } => *value,
        ASTNode::Variable { name } => lookup_binding(name, bindings).ok_or_else(|| {
            MathvizError::EvalError(format!("undefined symbol: {name}"))
        })?,
        ASTNode::Unary { op, child } => {
            let v = eval_ast_inner(child, bindings, allow_non_finite, depth + 1)?;
            match op {
                UnaryOp::Neg => -v,
                UnaryOp::Sqrt => v.sqrt(),
                UnaryOp::Abs => v.abs(),
                UnaryOp::Sin => v.sin(),
                UnaryOp::Cos => v.cos(),
                UnaryOp::Tan => v.tan(),
                UnaryOp::Asin => v.asin(),
                UnaryOp::Acos => v.acos(),
                UnaryOp::Atan => v.atan(),
                UnaryOp::Sinh => v.sinh(),
                UnaryOp::Cosh => v.cosh(),
                UnaryOp::Tanh => v.tanh(),
                UnaryOp::Exp => v.exp(),
                UnaryOp::Ln => v.ln(),
                UnaryOp::Log10 => v.log10(),
                UnaryOp::Floor => v.floor(),
                UnaryOp::Ceil => v.ceil(),
                UnaryOp::Sign => v.signum(),
            }
        }
        ASTNode::Binary { op, left, right } => {
            let l = eval_ast_inner(left, bindings, allow_non_finite, depth + 1)?;
            let r = eval_ast_inner(right, bindings, allow_non_finite, depth + 1)?;
            match op {
                BinaryOp::Add => l + r,
                BinaryOp::Sub => l - r,
                BinaryOp::Mul => l * r,
                BinaryOp::Div => l / r,
                BinaryOp::Pow => l.powf(r),
                BinaryOp::Atan2 => l.atan2(r),
                BinaryOp::Mod => l % r,
            }
        }
        ASTNode::Nary { op, children } => {
            if children.is_empty() {
                return Err(MathvizError::EvalError(
                    "n-ary operation received no children".to_string(),
                ));
            }
            match op {
                NaryOp::Sum => {
                    let mut sum = 0.0;
                    for child in children {
                        sum += eval_ast_inner(child, bindings, allow_non_finite, depth + 1)?;
                    }
                    sum
                }
                NaryOp::Product => {
                    let mut product = 1.0;
                    for child in children {
                        product *= eval_ast_inner(child, bindings, allow_non_finite, depth + 1)?;
                    }
                    product
                }
            }
        }
    };

    if !allow_non_finite && !value.is_finite() {
        return Err(MathvizError::EvalError("encountered non-finite value".to_string()));
    }

    Ok(value)
}

#[inline(always)]
fn lookup_binding(name: &str, bindings: &[(&str, f64)]) -> Option<f64> {
    for (k, v) in bindings {
        if *k == name {
            return Some(*v);
        }
    }
    lookup_constant(name)
}
