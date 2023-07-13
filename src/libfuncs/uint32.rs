//! # `u32`-related libfuncs

use super::{LibfuncBuilder, LibfuncHelper};
use crate::{
    error::{
        libfuncs::{Error, Result},
        CoreTypeBuilderError,
    },
    metadata::MetadataStorage,
    types::TypeBuilder,
};
use cairo_lang_sierra::{
    extensions::{
        int::{
            unsigned::{
                Uint32Concrete, Uint32Traits, UintConcrete, UintConstConcreteLibfunc,
                UintOperationConcreteLibfunc,
            },
            IntOperator,
        },
        lib_func::SignatureOnlyConcreteLibfunc,
        ConcreteLibfunc, GenericLibfunc, GenericType,
    },
    program_registry::ProgramRegistry,
};
use melior::{
    dialect::{
        arith::{self, CmpiPredicate},
        cf, llvm,
    },
    ir::{
        attribute::{DenseI64ArrayAttribute, IntegerAttribute},
        operation::OperationBuilder,
        r#type::IntegerType,
        Attribute, Block, Location, Value, ValueLike,
    },
    Context,
};

/// Select and call the correct libfunc builder function from the selector.
pub fn build<'ctx, 'this, TType, TLibfunc>(
    context: &'ctx Context,
    registry: &ProgramRegistry<TType, TLibfunc>,
    entry: &'this Block<'ctx>,
    location: Location<'ctx>,
    helper: &LibfuncHelper<'ctx, 'this>,
    metadata: &mut MetadataStorage,
    selector: &Uint32Concrete,
) -> Result<()>
where
    TType: GenericType,
    TLibfunc: GenericLibfunc,
    <TType as GenericType>::Concrete: TypeBuilder<TType, TLibfunc, Error = CoreTypeBuilderError>,
    <TLibfunc as GenericLibfunc>::Concrete: LibfuncBuilder<TType, TLibfunc, Error = Error>,
{
    match selector {
        UintConcrete::Const(info) => {
            build_const(context, registry, entry, location, helper, metadata, info)
        }
        UintConcrete::Operation(info) => {
            build_operation(context, registry, entry, location, helper, info)
        }
        UintConcrete::SquareRoot(_) => todo!(),
        UintConcrete::Equal(info) => build_equal(context, registry, entry, location, helper, info),
        UintConcrete::ToFelt252(info) => {
            build_to_felt252(context, registry, entry, location, helper, metadata, info)
        }
        UintConcrete::FromFelt252(info) => {
            build_from_felt252(context, registry, entry, location, helper, metadata, info)
        }
        UintConcrete::IsZero(info) => {
            build_is_zero(context, registry, entry, location, helper, info)
        }
        UintConcrete::Divmod(info) => {
            build_divmod(context, registry, entry, location, helper, info)
        }
        UintConcrete::WideMul(_) => todo!(),
    }
}

/// Generate MLIR operations for the `u32_const` libfunc.
pub fn build_const<'ctx, 'this, TType, TLibfunc>(
    context: &'ctx Context,
    registry: &ProgramRegistry<TType, TLibfunc>,
    entry: &'this Block<'ctx>,
    location: Location<'ctx>,
    helper: &LibfuncHelper<'ctx, 'this>,
    metadata: &mut MetadataStorage,
    info: &UintConstConcreteLibfunc<Uint32Traits>,
) -> Result<()>
where
    TType: GenericType,
    TLibfunc: GenericLibfunc,
    <TType as GenericType>::Concrete: TypeBuilder<TType, TLibfunc, Error = CoreTypeBuilderError>,
    <TLibfunc as GenericLibfunc>::Concrete: LibfuncBuilder<TType, TLibfunc, Error = Error>,
{
    let value = info.c;
    let value_ty = registry
        .get_type(&info.signature.branch_signatures[0].vars[0].ty)?
        .build(context, helper, registry, metadata)?;

    let op0 = entry.append_operation(arith::constant(
        context,
        Attribute::parse(context, &format!("{value} : {value_ty}")).unwrap(),
        location,
    ));
    entry.append_operation(helper.br(0, &[op0.result(0)?.into()], location));

    Ok(())
}

/// Generate MLIR operations for the u32 operation libfunc.
pub fn build_operation<'ctx, 'this, TType, TLibfunc>(
    context: &'ctx Context,
    _registry: &ProgramRegistry<TType, TLibfunc>,
    entry: &'this Block<'ctx>,
    location: Location<'ctx>,
    helper: &LibfuncHelper<'ctx, 'this>,
    info: &UintOperationConcreteLibfunc,
) -> Result<()>
where
    TType: GenericType,
    TLibfunc: GenericLibfunc,
    <TType as GenericType>::Concrete: TypeBuilder<TType, TLibfunc, Error = CoreTypeBuilderError>,
    <TLibfunc as GenericLibfunc>::Concrete: LibfuncBuilder<TType, TLibfunc, Error = Error>,
{
    let range_check: Value = entry.argument(0)?.into();
    let lhs: Value = entry.argument(1)?.into();
    let rhs: Value = entry.argument(2)?.into();

    let op_name = match info.operator {
        IntOperator::OverflowingAdd => "llvm.intr.uadd.with.overflow",
        IntOperator::OverflowingSub => "llvm.intr.usub.with.overflow",
    };

    let values_type = lhs.r#type();

    let result_type = llvm::r#type::r#struct(
        context,
        &[values_type, IntegerType::new(context, 1).into()],
        false,
    );

    let op = entry.append_operation(
        OperationBuilder::new(op_name, location)
            .add_operands(&[lhs, rhs])
            .add_results(&[result_type])
            .build(),
    );
    let result = op.result(0)?.into();

    let op = entry.append_operation(llvm::extract_value(
        context,
        result,
        DenseI64ArrayAttribute::new(context, &[0]),
        values_type,
        location,
    ));
    let op_result = op.result(0)?.into();

    let op = entry.append_operation(llvm::extract_value(
        context,
        result,
        DenseI64ArrayAttribute::new(context, &[1]),
        IntegerType::new(context, 1).into(),
        location,
    ));
    let op_overflow = op.result(0)?.into();

    entry.append_operation(helper.cond_br(
        op_overflow,
        [1, 0],
        [&[range_check, op_result], &[range_check, op_result]],
        location,
    ));
    Ok(())
}

/// Generate MLIR operations for the `u32_eq` libfunc.
pub fn build_equal<'ctx, 'this, TType, TLibfunc>(
    context: &'ctx Context,
    _registry: &ProgramRegistry<TType, TLibfunc>,
    entry: &'this Block<'ctx>,
    location: Location<'ctx>,
    helper: &LibfuncHelper<'ctx, 'this>,
    _info: &SignatureOnlyConcreteLibfunc,
) -> Result<()>
where
    TType: GenericType,
    TLibfunc: GenericLibfunc,
    <TType as GenericType>::Concrete: TypeBuilder<TType, TLibfunc, Error = CoreTypeBuilderError>,
    <TLibfunc as GenericLibfunc>::Concrete: LibfuncBuilder<TType, TLibfunc, Error = Error>,
{
    let arg0: Value = entry.argument(0)?.into();
    let arg1: Value = entry.argument(1)?.into();

    let op0 = entry.append_operation(arith::cmpi(
        context,
        CmpiPredicate::Eq,
        arg0,
        arg1,
        location,
    ));

    entry.append_operation(helper.cond_br(op0.result(0)?.into(), [1, 0], [&[]; 2], location));

    Ok(())
}

/// Generate MLIR operations for the `u32_is_zero` libfunc.
pub fn build_is_zero<'ctx, 'this, TType, TLibfunc>(
    context: &'ctx Context,
    _registry: &ProgramRegistry<TType, TLibfunc>,
    entry: &'this Block<'ctx>,
    location: Location<'ctx>,
    helper: &LibfuncHelper<'ctx, 'this>,
    _info: &SignatureOnlyConcreteLibfunc,
) -> Result<()>
where
    TType: GenericType,
    TLibfunc: GenericLibfunc,
    <TType as GenericType>::Concrete: TypeBuilder<TType, TLibfunc, Error = CoreTypeBuilderError>,
    <TLibfunc as GenericLibfunc>::Concrete: LibfuncBuilder<TType, TLibfunc, Error = Error>,
{
    let arg0: Value = entry.argument(0)?.into();

    let op = entry.append_operation(arith::constant(
        context,
        IntegerAttribute::new(0, arg0.r#type()).into(),
        location,
    ));
    let const_0 = op.result(0)?.into();

    let op = entry.append_operation(arith::cmpi(
        context,
        CmpiPredicate::Eq,
        arg0,
        const_0,
        location,
    ));
    let condition = op.result(0)?.into();

    entry.append_operation(helper.cond_br(condition, [0, 1], [&[], &[arg0]], location));

    Ok(())
}

/// Generate MLIR operations for the `u32_safe_divmod` libfunc.
pub fn build_divmod<'ctx, 'this, TType, TLibfunc>(
    _context: &'ctx Context,
    _registry: &ProgramRegistry<TType, TLibfunc>,
    entry: &'this Block<'ctx>,
    location: Location<'ctx>,
    helper: &LibfuncHelper<'ctx, 'this>,
    _info: &SignatureOnlyConcreteLibfunc,
) -> Result<()>
where
    TType: GenericType,
    TLibfunc: GenericLibfunc,
    <TType as GenericType>::Concrete: TypeBuilder<TType, TLibfunc, Error = CoreTypeBuilderError>,
    <TLibfunc as GenericLibfunc>::Concrete: LibfuncBuilder<TType, TLibfunc, Error = Error>,
{
    let lhs: Value = entry.argument(1)?.into();
    let rhs: Value = entry.argument(2)?.into();

    let op = entry.append_operation(arith::divui(lhs, rhs, location));

    let result_div = op.result(0)?.into();
    let op = entry.append_operation(arith::remui(lhs, rhs, location));
    let result_rem = op.result(0)?.into();

    entry.append_operation(helper.br(
        0,
        &[entry.argument(0)?.into(), result_div, result_rem],
        location,
    ));
    Ok(())
}

/// Generate MLIR operations for the `u32_to_felt252` libfunc.
pub fn build_to_felt252<'ctx, 'this, TType, TLibfunc>(
    context: &'ctx Context,
    registry: &ProgramRegistry<TType, TLibfunc>,
    entry: &'this Block<'ctx>,
    location: Location<'ctx>,
    helper: &LibfuncHelper<'ctx, 'this>,
    metadata: &mut MetadataStorage,
    info: &SignatureOnlyConcreteLibfunc,
) -> Result<()>
where
    TType: GenericType,
    TLibfunc: GenericLibfunc,
    <TType as GenericType>::Concrete: TypeBuilder<TType, TLibfunc, Error = CoreTypeBuilderError>,
    <TLibfunc as GenericLibfunc>::Concrete: LibfuncBuilder<TType, TLibfunc, Error = Error>,
{
    let felt252_ty = registry
        .get_type(&info.branch_signatures()[0].vars[0].ty)?
        .build(context, helper, registry, metadata)?;
    let value: Value = entry.argument(0)?.into();

    let op = entry.append_operation(arith::extui(value, felt252_ty, location));

    let result = op.result(0)?.into();

    entry.append_operation(helper.br(0, &[result], location));

    Ok(())
}

/// Generate MLIR operations for the `u32_from_felt252` libfunc.
pub fn build_from_felt252<'ctx, 'this, TType, TLibfunc>(
    context: &'ctx Context,
    registry: &ProgramRegistry<TType, TLibfunc>,
    entry: &'this Block<'ctx>,
    location: Location<'ctx>,
    helper: &LibfuncHelper<'ctx, 'this>,
    metadata: &mut MetadataStorage,
    info: &SignatureOnlyConcreteLibfunc,
) -> Result<()>
where
    TType: GenericType,
    TLibfunc: GenericLibfunc,
    <TType as GenericType>::Concrete: TypeBuilder<TType, TLibfunc, Error = CoreTypeBuilderError>,
    <TLibfunc as GenericLibfunc>::Concrete: LibfuncBuilder<TType, TLibfunc, Error = Error>,
{
    let range_check: Value = entry.argument(0)?.into();
    let value: Value = entry.argument(1)?.into();

    let felt252_ty = registry
        .get_type(&info.param_signatures()[1].ty)?
        .build(context, helper, registry, metadata)?;
    let result_ty = registry
        .get_type(&info.branch_signatures()[0].vars[1].ty)?
        .build(context, helper, registry, metadata)?;

    let op = entry.append_operation(arith::constant(
        context,
        Attribute::parse(context, &format!("{} : {}", u32::MAX, felt252_ty)).unwrap(),
        location,
    ));
    let const_max = op.result(0)?.into();

    let op = entry.append_operation(arith::cmpi(
        context,
        CmpiPredicate::Ule,
        value,
        const_max,
        location,
    ));
    let is_ule = op.result(0)?.into();

    let block_success = helper.append_block(Block::new(&[]));
    let block_failure = helper.append_block(Block::new(&[]));

    entry.append_operation(cf::cond_br(
        context,
        is_ule,
        block_success,
        block_failure,
        &[],
        &[],
        location,
    ));

    let op = block_success.append_operation(arith::trunci(value, result_ty, location));
    let value = op.result(0)?.into();
    block_success.append_operation(helper.br(0, &[range_check, value], location));

    block_failure.append_operation(helper.br(1, &[range_check], location));

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{
        types::felt252::PRIME,
        utils::test::{load_cairo, run_program},
    };
    use cairo_lang_sierra::program::Program;
    use lazy_static::lazy_static;
    use num_bigint::{BigInt, Sign};
    use serde_json::json;
    use std::ops::Neg;

    lazy_static! {
        static ref U32_OVERFLOWING_ADD: (String, Program) = load_cairo! {
            fn run_test(lhs: u32, rhs: u32) -> u32 {
                lhs + rhs
            }
        };
        static ref U32_OVERFLOWING_SUB: (String, Program) = load_cairo! {
            fn run_test(lhs: u32, rhs: u32) -> u32 {
                lhs - rhs
            }
        };
        static ref U32_SAFE_DIVMOD: (String, Program) = load_cairo! {
            fn run_test(lhs: u32, rhs: u32) -> (u32, u32) {
                let q = lhs / rhs;
                let r = lhs % rhs;

                (q, r)
            }
        };
        static ref U32_EQUAL: (String, Program) = load_cairo! {
            fn run_test(lhs: u32, rhs: u32) -> bool {
                lhs == rhs
            }
        };
        static ref U32_IS_ZERO: (String, Program) = load_cairo! {
            use zeroable::IsZeroResult;

            extern fn u32_is_zero(a: u32) -> IsZeroResult<u32> implicits() nopanic;

            fn run_test(value: u32) -> bool {
                match u32_is_zero(value) {
                    IsZeroResult::Zero(_) => true,
                    IsZeroResult::NonZero(_) => false,
                }
            }
        };
    }

    // Parse numeric string into felt, wrapping negatives around the prime modulo.
    fn f(value: &str) -> [u32; 8] {
        let value = value.parse::<BigInt>().unwrap();
        let value = match value.sign() {
            Sign::Minus => &*PRIME - value.neg().to_biguint().unwrap(),
            _ => value.to_biguint().unwrap(),
        };

        let mut u32_digits = value.to_u32_digits();
        u32_digits.resize(8, 0);
        u32_digits.try_into().unwrap()
    }

    #[test]
    fn u32_const_min() {
        let program = load_cairo!(
            fn run_test() -> u32 {
                0_u32
            }
        );
        let result = run_program(&program, "run_test", json!([]));

        assert_eq!(result, json!([0]));
    }

    #[test]
    fn u32_const_max() {
        let program = load_cairo!(
            fn run_test() -> u32 {
                4294967295_u32
            }
        );
        let result = run_program(&program, "run_test", json!([]));

        assert_eq!(result, json!([4294967295u32]));
    }

    #[test]
    fn u32_to_felt252() {
        let program = load_cairo!(
            use traits::Into;

            fn run_test() -> felt252 {
                2_u32.into()
            }
        );
        let result = run_program(&program, "run_test", json!([]));

        assert_eq!(result, json!([[2, 0, 0, 0, 0, 0, 0, 0]]));
    }

    #[test]
    fn u32_from_felt252() {
        let program = load_cairo!(
            use traits::TryInto;

            fn run_test() -> (Option<u32>, Option<u32>) {
                (4294967295.try_into(), 4294967296.try_into())
            }
        );
        let result = run_program(&program, "run_test", json!([null]));

        assert_eq!(result, json!([null, [[0, 4294967295u32], [1, []]]]));
    }

    #[test]
    fn u32_overflowing_add() {
        fn run<const LHS: u32, const RHS: u32>() -> serde_json::Value {
            run_program(&U32_OVERFLOWING_ADD, "run_test", json!([(), LHS, RHS]))
        }

        let add_error = f("155785504323917466144735657540098748279");

        assert_eq!(run::<0, 0>(), json!([(), [0, [0]]]));
        assert_eq!(run::<0, 1>(), json!([(), [0, [1]]]));
        assert_eq!(run::<0, 4294967294>(), json!([(), [0, [4294967294u32]]]));
        assert_eq!(run::<0, 4294967295>(), json!([(), [0, [4294967295u32]]]));

        assert_eq!(run::<1, 0>(), json!([(), [0, [1]]]));
        assert_eq!(run::<1, 1>(), json!([(), [0, [2]]]));
        assert_eq!(run::<1, 4294967294>(), json!([(), [0, [4294967295u32]]]));
        assert_eq!(run::<1, 4294967295>(), json!([(), [1, [[], [add_error]]]]));

        assert_eq!(run::<4294967294, 0>(), json!([(), [0, [4294967294u32]]]));
        assert_eq!(run::<4294967294, 1>(), json!([(), [0, [4294967295u32]]]));
        assert_eq!(
            run::<4294967294, 4294967294>(),
            json!([(), [1, [[], [add_error]]]])
        );
        assert_eq!(
            run::<4294967294, 4294967295>(),
            json!([(), [1, [[], [add_error]]]])
        );

        assert_eq!(run::<4294967295, 0>(), json!([(), [0, [4294967295u32]]]));
        assert_eq!(run::<4294967295, 1>(), json!([(), [1, [[], [add_error]]]]));
        assert_eq!(
            run::<4294967295, 4294967294>(),
            json!([(), [1, [[], [add_error]]]])
        );
        assert_eq!(
            run::<4294967295, 4294967295>(),
            json!([(), [1, [[], [add_error]]]])
        );
    }

    #[test]
    fn u32_overflowing_sub() {
        fn run<const LHS: u32, const RHS: u32>() -> serde_json::Value {
            run_program(&U32_OVERFLOWING_SUB, "run_test", json!([(), LHS, RHS]))
        }

        let sub_error = f("155785504329508738615720351733824384887");

        assert_eq!(run::<0, 0>(), json!([(), [0, [0]]]));
        assert_eq!(run::<0, 1>(), json!([(), [1, [[], [sub_error]]]]));
        assert_eq!(run::<0, 4294967294>(), json!([(), [1, [[], [sub_error]]]]));
        assert_eq!(run::<0, 4294967295>(), json!([(), [1, [[], [sub_error]]]]));

        assert_eq!(run::<1, 0>(), json!([(), [0, [1]]]));
        assert_eq!(run::<1, 1>(), json!([(), [0, [0]]]));
        assert_eq!(run::<1, 4294967294>(), json!([(), [1, [[], [sub_error]]]]));
        assert_eq!(run::<1, 4294967295>(), json!([(), [1, [[], [sub_error]]]]));

        assert_eq!(run::<4294967294, 0>(), json!([(), [0, [4294967294u32]]]));
        assert_eq!(run::<4294967294, 1>(), json!([(), [0, [4294967293u32]]]));
        assert_eq!(run::<4294967294, 4294967294>(), json!([(), [0, [0]]]));
        assert_eq!(
            run::<4294967294, 4294967295>(),
            json!([(), [1, [[], [sub_error]]]])
        );

        assert_eq!(run::<4294967295, 0>(), json!([(), [0, [4294967295u32]]]));
        assert_eq!(run::<4294967295, 1>(), json!([(), [0, [4294967294u32]]]));
        assert_eq!(run::<4294967295, 4294967294>(), json!([(), [0, [1]]]));
        assert_eq!(run::<4294967295, 4294967295>(), json!([(), [0, [0]]]));
    }

    #[test]
    fn u32_equal() {
        let r = |lhs, rhs| run_program(&U32_EQUAL, "run_test", json!([lhs, rhs]));

        assert_eq!(r(0, 0), json!([[1, []]]));
        assert_eq!(r(0, 1), json!([[0, []]]));
        assert_eq!(r(1, 0), json!([[0, []]]));
        assert_eq!(r(1, 1), json!([[1, []]]));
    }

    #[test]
    fn u32_is_zero() {
        let r = |value| run_program(&U32_IS_ZERO, "run_test", json!([value]));

        assert_eq!(r(0), json!([[1, []]]));
        assert_eq!(r(1), json!([[0, []]]));
    }

    #[test]
    fn u32_safe_divmod() {
        let r = |lhs, rhs| run_program(&U32_SAFE_DIVMOD, "run_test", json!([(), lhs, rhs]));

        let u32_is_zero = json!([f("8445148841039306800")]);

        assert_eq!(r(0, 0), json!([(), [1, [[], u32_is_zero]]]));
        assert_eq!(r(0, 1), json!([(), [0, [[0u32, 0u32]]]]));
        assert_eq!(r(0, 0xFFFFFFFFu32), json!([(), [0, [[0u32, 0u32]]]]));

        assert_eq!(r(1, 0), json!([(), [1, [[], u32_is_zero]]]));
        assert_eq!(r(1, 1), json!([(), [0, [[1u32, 0u32]]]]));
        assert_eq!(r(1, 0xFFFFFFFFu32), json!([(), [0, [[0u32, 1u32]]]]));

        assert_eq!(r(0xFFFFFFFFu32, 0), json!([(), [1, [[], u32_is_zero]]]));
        assert_eq!(
            r(0xFFFFFFFFu32, 1),
            json!([(), [0, [[0xFFFFFFFFu32, 0u32]]]])
        );
        assert_eq!(
            r(0xFFFFFFFFu32, 0xFFFFFFFFu32),
            json!([(), [0, [[1u32, 0u32]]]])
        );
    }
}