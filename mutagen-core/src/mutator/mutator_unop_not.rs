//! Mutator for binary operation `+`.

use std::convert::TryFrom;
use std::ops::Deref;
use std::ops::Not;

use proc_macro2::Span;
use quote::quote_spanned;
use syn::spanned::Spanned;
use syn::{Expr, UnOp};

use crate::comm::Mutation;
use crate::transformer::transform_info::SharedTransformInfo;
use crate::transformer::TransformContext;

use crate::MutagenRuntimeConfig;

pub fn run<T: Not>(
    mutator_id: usize,
    val: T,
    runtime: impl Deref<Target = MutagenRuntimeConfig>,
) -> <T as Not>::Output {
    runtime.covered(mutator_id);
    if runtime.is_mutation_active(mutator_id) {
        val.may_none()
    } else {
        !val
    }
}

pub fn run_native_num<I: Not<Output = I>>(
    mutator_id: usize,
    val: I,
    runtime: impl Deref<Target = MutagenRuntimeConfig>,
) -> I {
    runtime.covered(mutator_id);
    if runtime.is_mutation_active(mutator_id) {
        val
    } else {
        !val
    }
}

pub fn transform(
    e: Expr,
    transform_info: &SharedTransformInfo,
    context: &TransformContext,
) -> Expr {
    let e = match ExprUnopNot::try_from(e) {
        Ok(e) => e,
        Err(e) => return e,
    };

    let mutator_id = transform_info.add_mutation(Mutation::new_spanned(
        &context,
        "unop_not".to_owned(),
        "!".to_owned(),
        "".to_owned(),
        e.span,
    ));

    let expr = &e.expr;

    // if the current expression is based on numbers, use the function `run_native_num` instead
    let run_fn = if context.is_num_expr() {
        quote_spanned! {e.span=> run_native_num}
    } else {
        quote_spanned! {e.span=> run}
    };

    syn::parse2(quote_spanned! {e.span=>
        ::mutagen::mutator::mutator_unop_not::#run_fn(
                #mutator_id,
                #expr,
                ::mutagen::MutagenRuntimeConfig::get_default()
            )
    })
    .expect("transformed code invalid")
}

#[derive(Clone, Debug)]
pub struct ExprUnopNot {
    pub expr: Expr,
    pub span: Span,
}

impl TryFrom<Expr> for ExprUnopNot {
    type Error = Expr;
    fn try_from(expr: Expr) -> Result<Self, Expr> {
        match expr {
            Expr::Unary(expr) => match expr.op {
                UnOp::Not(t) => Ok(ExprUnopNot {
                    expr: *expr.expr,
                    span: t.span(),
                }),
                _ => Err(Expr::Unary(expr)),
            },
            e => Err(e),
        }
    }
}

/// trait that is used to optimistically remove a negation `!` from an expression
///
/// This trait provides a function `may_none` that passes the input value unchanged
/// If the value cannot be converted to the output type of the negation using `Into`, the optimistic assumption fails.
pub trait NotToNone {
    type Output;
    // do nothing
    fn may_none(self) -> Self::Output;
}

impl<T> NotToNone for T
where
    T: Not,
{
    type Output = <T as Not>::Output;

    default fn may_none(self) -> <T as Not>::Output {
        MutagenRuntimeConfig::get_default().optimistic_assmuption_failed();
    }
}

impl<T> NotToNone for T
where
    T: Not,
    T: Into<<T as Not>::Output>,
{
    fn may_none(self) -> Self::Output {
        self.into()
    }
}

/// types for testing the optimistic mutator that removes the negation
#[cfg(any(test, feature = "self_test"))]
pub mod optimistc_types {

    use std::ops::Not;

    #[derive(Debug, PartialEq)]
    pub struct TypeWithNotOtherOutput();
    #[derive(Debug, PartialEq)]
    pub struct TypeWithNotTarget();

    impl Not for TypeWithNotOtherOutput {
        type Output = TypeWithNotTarget;

        fn not(self) -> <Self as Not>::Output {
            TypeWithNotTarget()
        }
    }
}

#[cfg(test)]
mod tests {

    use super::optimistc_types::*;
    use super::*;

    #[test]
    fn boolnot_inactive() {
        // input is true, but will be negated by non-active mutator
        let result = run(1, true, &MutagenRuntimeConfig::without_mutation());
        assert_eq!(result, false);
    }
    #[test]
    fn boolnot_active() {
        let result = run(1, true, &MutagenRuntimeConfig::with_mutation_id(1));
        assert_eq!(result, true);
    }
    #[test]
    fn intnot_active() {
        let result = run_native_num(1, 1, &MutagenRuntimeConfig::with_mutation_id(1));
        assert_eq!(result, 1);
    }

    #[test]
    fn optimistic_incorrect_inactive() {
        let result = run(
            1,
            TypeWithNotOtherOutput(),
            &MutagenRuntimeConfig::without_mutation(),
        );
        assert_eq!(result, TypeWithNotTarget());
    }
    #[test]
    #[should_panic]
    fn optimistic_incorrect_active() {
        run(
            1,
            TypeWithNotOtherOutput(),
            &MutagenRuntimeConfig::with_mutation_id(1),
        );
    }
}
