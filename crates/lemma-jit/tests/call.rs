//! Loading an emitted object into a JIT, looking its functions up and calling them.

use std::error::Error;

use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::FunctionValue;
use lemma_codegen::{Target, emit_object};
use lemma_jit::{Jit, JitError};

type Add = extern "C" fn(i64, i64) -> i64;
type SumBelow = extern "C" fn(u64) -> u64;
type Scale = extern "C" fn(f64, f64) -> f64;
type AddThree = extern "C" fn(i64, i64, i64) -> i64;
type Missing = extern "C" fn() -> i64;

const ADD_CASES: [((i64, i64), i64); 3] = [((2, 3), 5), ((i64::MAX, 1), i64::MIN), ((-7, 7), 0)];
const SUM_BELOW_CASES: [((u64,), u64); 3] = [((0,), 0), ((1,), 0), ((10,), 45)];
const SCALE_CASES: [((f64, f64), f64); 1] = [((1.5, 4.0), 6.0)];
const ADD_THREE_CASES: [((i64, i64, i64), i64); 1] = [((1, 2, 3), 6)];

const OBJECT_NAME: &str = "arithmetic";
const TRUNCATED_LENGTH: usize = 64;

const NOT_OBJECTS: [(&str, &[u8]); 2] = [("text", b"not an object"), ("empty", b"")];

/// The two pinned targets that are not the host, whose objects this host's JIT must refuse.
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
const FOREIGN_TARGETS: (Target, Target) = (Target::X86_64Linux, Target::Aarch64Linux);
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
const FOREIGN_TARGETS: (Target, Target) = (Target::AppleAarch64, Target::Aarch64Linux);
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
const FOREIGN_TARGETS: (Target, Target) = (Target::AppleAarch64, Target::X86_64Linux);

fn host() -> Result<Target, Box<dyn Error>> {
    Ok(Target::host().ok_or("the suite runs on a host with no pinned target")?)
}

/// `add(left, right) = left + right`, wrapping.
fn define_add<'context>(
    context: &'context Context,
    module: &Module<'context>,
    builder: &Builder<'context>,
) -> Result<FunctionValue<'context>, Box<dyn Error>> {
    let i64_type = context.i64_type();
    let signature = i64_type.fn_type(&[i64_type.into(), i64_type.into()], false);
    let function = module.add_function("add", signature, None);
    builder.position_at_end(context.append_basic_block(function, "entry"));
    let left = function.get_nth_param(0).ok_or("add: no left")?;
    let right = function.get_nth_param(1).ok_or("add: no right")?;
    let sum = builder.build_int_add(left.into_int_value(), right.into_int_value(), "sum")?;
    builder.build_return(Some(&sum))?;
    Ok(function)
}

/// `sum_below(limit)`: the sum of every integer from 0 up to `limit` excluded, as a loop whose
/// counter and accumulator are phi nodes.
fn define_sum_below<'context>(
    context: &'context Context,
    module: &Module<'context>,
    builder: &Builder<'context>,
) -> Result<(), Box<dyn Error>> {
    let i64_type = context.i64_type();
    let function = module.add_function(
        "sum_below",
        i64_type.fn_type(&[i64_type.into()], false),
        None,
    );
    let limit = function
        .get_nth_param(0)
        .ok_or("sum_below: no limit")?
        .into_int_value();
    let entry = context.append_basic_block(function, "entry");
    let header = context.append_basic_block(function, "header");
    let body = context.append_basic_block(function, "body");
    let exit = context.append_basic_block(function, "exit");
    let zero = i64_type.const_zero();
    let one = i64_type.const_int(1, false);

    builder.position_at_end(entry);
    builder.build_unconditional_branch(header)?;

    builder.position_at_end(header);
    let counter = builder.build_phi(i64_type, "counter")?;
    let total = builder.build_phi(i64_type, "total")?;
    let counter_value = counter.as_basic_value().into_int_value();
    let total_value = total.as_basic_value().into_int_value();
    let below = builder.build_int_compare(IntPredicate::ULT, counter_value, limit, "below")?;
    builder.build_conditional_branch(below, body, exit)?;

    builder.position_at_end(body);
    let next_total = builder.build_int_add(total_value, counter_value, "next_total")?;
    let next_counter = builder.build_int_add(counter_value, one, "next_counter")?;
    builder.build_unconditional_branch(header)?;

    counter.add_incoming(&[(&zero, entry), (&next_counter, body)]);
    total.add_incoming(&[(&zero, entry), (&next_total, body)]);

    builder.position_at_end(exit);
    builder.build_return(Some(&total_value))?;
    Ok(())
}

/// `scale(value, factor) = value * factor`.
fn define_scale<'context>(
    context: &'context Context,
    module: &Module<'context>,
    builder: &Builder<'context>,
) -> Result<(), Box<dyn Error>> {
    let f64_type = context.f64_type();
    let signature = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
    let function = module.add_function("scale", signature, None);
    builder.position_at_end(context.append_basic_block(function, "entry"));
    let value = function.get_nth_param(0).ok_or("scale: no value")?;
    let factor = function.get_nth_param(1).ok_or("scale: no factor")?;
    let product = builder.build_float_mul(
        value.into_float_value(),
        factor.into_float_value(),
        "product",
    )?;
    builder.build_return(Some(&product))?;
    Ok(())
}

/// `add_three(first, second, third) = add(add(first, second), third)`: two calls to `add`, which
/// the object leaves as relocations for the JIT linker to resolve.
fn define_add_three<'context>(
    context: &'context Context,
    module: &Module<'context>,
    builder: &Builder<'context>,
    add: FunctionValue<'context>,
) -> Result<(), Box<dyn Error>> {
    let i64_type = context.i64_type();
    let signature = i64_type.fn_type(&[i64_type.into(), i64_type.into(), i64_type.into()], false);
    let function = module.add_function("add_three", signature, None);
    builder.position_at_end(context.append_basic_block(function, "entry"));
    let first = function.get_nth_param(0).ok_or("add_three: no first")?;
    let second = function.get_nth_param(1).ok_or("add_three: no second")?;
    let third = function.get_nth_param(2).ok_or("add_three: no third")?;
    let partial = builder
        .build_call(add, &[first.into(), second.into()], "partial")?
        .try_as_basic_value()
        .basic()
        .ok_or("add_three: add returned nothing")?;
    let sum = builder
        .build_call(add, &[partial.into(), third.into()], "sum")?
        .try_as_basic_value()
        .basic()
        .ok_or("add_three: add returned nothing")?;
    builder.build_return(Some(&sum))?;
    Ok(())
}

fn object_for(target: Target) -> Result<Vec<u8>, Box<dyn Error>> {
    let context = Context::create();
    let module = context.create_module(OBJECT_NAME);
    let builder = context.create_builder();
    let add = define_add(&context, &module, &builder)?;
    define_sum_below(&context, &module, &builder)?;
    define_scale(&context, &module, &builder)?;
    define_add_three(&context, &module, &builder, add)?;
    Ok(emit_object(&module, target)?)
}

fn host_object() -> Result<Vec<u8>, Box<dyn Error>> {
    object_for(host()?)
}

fn loaded_jit(object: &[u8]) -> Result<Jit, Box<dyn Error>> {
    let mut jit = Jit::new(host()?)?;
    jit.add_object(OBJECT_NAME, object)?;
    Ok(jit)
}

#[test]
fn add_returns_the_wrapping_sum_of_its_arguments() -> Result<(), Box<dyn Error>> {
    let jit = loaded_jit(&host_object()?)?;
    // SAFETY: the loaded object defines `add` as a C function from two i64 to an i64 that only
    // reads its arguments.
    let add = unsafe { jit.function::<Add>("add") }?;

    let results = ADD_CASES.map(|(arguments, _)| (arguments, add.call(arguments)));

    assert_eq!(results, ADD_CASES);
    Ok(())
}

#[test]
fn sum_below_adds_every_integer_below_its_limit() -> Result<(), Box<dyn Error>> {
    let jit = loaded_jit(&host_object()?)?;
    // SAFETY: the loaded object defines `sum_below` as a C function from a u64 to a u64 that
    // only reads its argument and its own stack.
    let sum_below = unsafe { jit.function::<SumBelow>("sum_below") }?;

    let results = SUM_BELOW_CASES.map(|(arguments, _)| (arguments, sum_below.call(arguments)));

    assert_eq!(results, SUM_BELOW_CASES);
    Ok(())
}

#[test]
fn scale_returns_the_product_of_its_arguments() -> Result<(), Box<dyn Error>> {
    let jit = loaded_jit(&host_object()?)?;
    // SAFETY: the loaded object defines `scale` as a C function from two f64 to an f64 that only
    // reads its arguments.
    let scale = unsafe { jit.function::<Scale>("scale") }?;

    let results = SCALE_CASES.map(|(arguments, _)| (arguments, scale.call(arguments)));

    assert_eq!(results, SCALE_CASES);
    Ok(())
}

#[test]
fn add_three_resolves_its_calls_to_add() -> Result<(), Box<dyn Error>> {
    let jit = loaded_jit(&host_object()?)?;
    // SAFETY: the loaded object defines `add_three` as a C function from three i64 to an i64
    // that only reads its arguments and calls `add`, defined in the same object.
    let add_three = unsafe { jit.function::<AddThree>("add_three") }?;

    let results = ADD_THREE_CASES.map(|(arguments, _)| (arguments, add_three.call(arguments)));

    assert_eq!(results, ADD_THREE_CASES);
    Ok(())
}

#[test]
fn looking_up_an_undefined_symbol_is_a_lookup_error_naming_it() -> Result<(), Box<dyn Error>> {
    let jit = loaded_jit(&host_object()?)?;

    // SAFETY: no loaded object defines `missing`, so the lookup fails and no function is returned.
    let result = unsafe { jit.function::<Missing>("missing") };

    assert!(matches!(
        result,
        Err(JitError::Lookup { symbol, .. }) if symbol == "missing"
    ));
    Ok(())
}

#[test]
fn a_truncated_object_is_rejected_when_added() -> Result<(), Box<dyn Error>> {
    let object = host_object()?;
    let mut jit = Jit::new(host()?)?;

    let result = jit.add_object("truncated", &object[..TRUNCATED_LENGTH]);

    assert!(matches!(
        result,
        Err(JitError::ObjectRejected { name, .. }) if name == "truncated"
    ));
    Ok(())
}

#[test]
fn an_object_added_twice_is_rejected_the_second_time() -> Result<(), Box<dyn Error>> {
    let object = host_object()?;
    let mut jit = loaded_jit(&object)?;

    let result = jit.add_object("again", &object);

    assert!(matches!(
        result,
        Err(JitError::ObjectRejected { name, .. }) if name == "again"
    ));
    Ok(())
}

#[test]
fn one_object_loaded_into_two_jits_is_callable_from_both() -> Result<(), Box<dyn Error>> {
    let object = host_object()?;
    let first_jit = loaded_jit(&object)?;
    let second_jit = loaded_jit(&object)?;
    // SAFETY: the object loaded into the first JIT defines `add` as a C function from two i64 to
    // an i64 that only reads its arguments.
    let first_add = unsafe { first_jit.function::<Add>("add") }?;
    // SAFETY: the same object, loaded into the second JIT, defines the same `add`.
    let second_add = unsafe { second_jit.function::<Add>("add") }?;

    let sums = (first_add.call((2, 3)), second_add.call((2, 3)));

    assert_eq!(sums, (5, 5));
    Ok(())
}

#[test]
fn bytes_that_are_not_an_object_are_refused_naming_the_object() -> Result<(), Box<dyn Error>> {
    let mut jit = Jit::new(host()?)?;

    let results = NOT_OBJECTS.map(|(name, bytes)| (name, jit.add_object(name, bytes)));

    let refusals = NOT_OBJECTS.map(|(name, _)| {
        let refusal = JitError::NotAnObject {
            name: name.to_string(),
        };
        (name, Err::<(), _>(refusal))
    });
    assert_eq!(results, refusals);
    Ok(())
}

#[test]
fn an_object_for_the_first_foreign_target_is_refused_naming_the_jit_target()
-> Result<(), Box<dyn Error>> {
    let host = host()?;
    let object = object_for(FOREIGN_TARGETS.0)?;
    let mut jit = Jit::new(host)?;

    let result = jit.add_object("foreign", &object);

    let refusal = JitError::ObjectTargetMismatch {
        name: "foreign".to_string(),
        expected: host,
    };
    assert_eq!(result, Err(refusal));
    Ok(())
}

#[test]
fn an_object_for_the_second_foreign_target_is_refused_naming_the_jit_target()
-> Result<(), Box<dyn Error>> {
    let host = host()?;
    let object = object_for(FOREIGN_TARGETS.1)?;
    let mut jit = Jit::new(host)?;

    let result = jit.add_object("foreign", &object);

    let refusal = JitError::ObjectTargetMismatch {
        name: "foreign".to_string(),
        expected: host,
    };
    assert_eq!(result, Err(refusal));
    Ok(())
}
