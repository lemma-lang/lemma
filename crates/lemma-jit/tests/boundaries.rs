//! The edges of the JIT's inputs: names LLVM cannot hold, headers that are not relocatable
//! objects, a symbol with no address, and the host symbols an object is linked against today.

use std::error::Error;

use inkwell::context::Context;
use lemma_codegen::{Target, emit_object};
use lemma_jit::{Jit, JitError};

type CallAbs = extern "C" fn(i32) -> i32;
type Constant = extern "C" fn() -> i64;

const ELF_IDENTIFICATION: [u8; 6] = [0x7F, b'E', b'L', b'F', 2, 1];
const ELF_TYPE_RELOCATABLE: u16 = 1;
const ELF_TYPE_EXECUTABLE: u16 = 2;
const ELF_MACHINE_AARCH64: u16 = 183;
const ELF_PREFIX_WITHOUT_MACHINE: usize = 19;
const MACH_O_MAGIC_64: u32 = 0xFEED_FACF;
const MACH_O_CPU_TYPE_ARM64: u32 = 0x0100_000C;
const MACH_O_FILE_TYPE_OBJECT: u32 = 1;
const MACH_O_FILE_TYPE_EXECUTE: u32 = 2;

/// Module-level assembly defining a global symbol `zero` at the absolute address 0.
#[cfg(target_os = "macos")]
const ABSOLUTE_ZERO_ASSEMBLY: &str = ".globl _zero\n_zero = 0\n";
#[cfg(target_os = "linux")]
const ABSOLUTE_ZERO_ASSEMBLY: &str = ".globl zero\n.set zero, 0\n";

fn host() -> Result<Target, Box<dyn Error>> {
    Ok(Target::host().ok_or("the suite runs on a host with no pinned target")?)
}

/// An object whose `call_abs(value) = abs(value)` calls the C library's `abs`, which it does not
/// define.
fn abs_caller_object() -> Result<Vec<u8>, Box<dyn Error>> {
    let context = Context::create();
    let module = context.create_module("abs_caller");
    let builder = context.create_builder();
    let i32_type = context.i32_type();
    let signature = i32_type.fn_type(&[i32_type.into()], false);
    let abs = module.add_function("abs", signature, None);
    let function = module.add_function("call_abs", signature, None);
    builder.position_at_end(context.append_basic_block(function, "entry"));
    let value = function.get_nth_param(0).ok_or("call_abs: no value")?;
    let result = builder
        .build_call(abs, &[value.into()], "result")?
        .try_as_basic_value()
        .basic()
        .ok_or("call_abs: abs returned nothing")?;
    builder.build_return(Some(&result))?;
    Ok(emit_object(&module, host()?)?)
}

fn absolute_zero_object() -> Result<Vec<u8>, Box<dyn Error>> {
    let context = Context::create();
    let module = context.create_module("absolute_zero");
    module.set_inline_assembly(ABSOLUTE_ZERO_ASSEMBLY);
    Ok(emit_object(&module, host()?)?)
}

/// A 64-byte, 64-bit little-endian ELF header with the given file type and machine.
fn elf_header(file_type: u16, machine: u16) -> Vec<u8> {
    [
        ELF_IDENTIFICATION.as_slice(),
        &[1],
        &[0; 9],
        &file_type.to_le_bytes(),
        &machine.to_le_bytes(),
        &[0; 44],
    ]
    .concat()
}

/// A 32-byte Mach-O header with the given magic bytes, CPU type and file type.
fn mach_o_header(magic: [u8; 4], cpu_type: u32, file_type: u32) -> Vec<u8> {
    [
        magic.as_slice(),
        &cpu_type.to_le_bytes(),
        &[0; 4],
        &file_type.to_le_bytes(),
        &[0; 16],
    ]
    .concat()
}

fn headers_that_are_not_relocatable_objects() -> [(&'static str, Vec<u8>); 4] {
    let relocatable_elf = elf_header(ELF_TYPE_RELOCATABLE, ELF_MACHINE_AARCH64);
    [
        (
            "elf executable",
            elf_header(ELF_TYPE_EXECUTABLE, ELF_MACHINE_AARCH64),
        ),
        (
            "mach-o executable",
            mach_o_header(
                MACH_O_MAGIC_64.to_le_bytes(),
                MACH_O_CPU_TYPE_ARM64,
                MACH_O_FILE_TYPE_EXECUTE,
            ),
        ),
        (
            "mach-o big-endian magic",
            mach_o_header(
                MACH_O_MAGIC_64.to_be_bytes(),
                MACH_O_CPU_TYPE_ARM64,
                MACH_O_FILE_TYPE_OBJECT,
            ),
        ),
        (
            "elf cut before its machine",
            relocatable_elf[..ELF_PREFIX_WITHOUT_MACHINE].to_vec(),
        ),
    ]
}

#[test]
fn an_object_name_with_a_nul_byte_is_an_invalid_name_naming_it() -> Result<(), Box<dyn Error>> {
    let object = abs_caller_object()?;
    let mut jit = Jit::new(host()?)?;

    let result = jit.add_object("a\0b", &object);

    let refusal = JitError::InvalidName {
        name: "a\0b".to_string(),
    };
    assert_eq!(result, Err(refusal));
    Ok(())
}

#[test]
fn a_symbol_with_a_nul_byte_is_a_lookup_error_naming_it() -> Result<(), Box<dyn Error>> {
    let jit = Jit::new(host()?)?;

    // SAFETY: a name holding a NUL byte cannot reach LLVM, so no function is returned.
    let result = unsafe { jit.function::<Constant>("ze\0ro") };

    assert!(matches!(
        result,
        Err(JitError::Lookup { symbol, .. }) if symbol == "ze\0ro"
    ));
    Ok(())
}

#[test]
fn a_symbol_at_address_zero_is_a_lookup_error_with_no_address() -> Result<(), Box<dyn Error>> {
    let mut jit = Jit::new(host()?)?;
    jit.add_object("absolute_zero", &absolute_zero_object()?)?;

    // SAFETY: `zero` is an absolute symbol, not a function; the lookup refuses its address 0, so
    // no function is returned.
    let result = unsafe { jit.function::<Constant>("zero") };

    assert!(matches!(
        result,
        Err(JitError::Lookup { symbol, message }) if symbol == "zero" && message.contains("no address")
    ));
    Ok(())
}

#[test]
fn headers_that_are_not_relocatable_objects_are_refused_naming_the_object()
-> Result<(), Box<dyn Error>> {
    let mut jit = Jit::new(host()?)?;
    let cases = headers_that_are_not_relocatable_objects();

    let results = cases
        .each_ref()
        .map(|(name, bytes)| (*name, jit.add_object(name, bytes)));

    let refusals = cases.each_ref().map(|(name, _)| {
        let refusal = JitError::NotAnObject {
            name: name.to_string(),
        };
        (*name, Err::<(), _>(refusal))
    });
    assert_eq!(results, refusals);
    Ok(())
}

#[test]
fn a_call_to_a_host_libc_function_is_resolved_from_the_process() -> Result<(), Box<dyn Error>> {
    let mut jit = Jit::new(host()?)?;
    jit.add_object("abs_caller", &abs_caller_object()?)?;
    // SAFETY: the loaded object defines `call_abs` as a C function from an i32 to an i32 that
    // only passes its argument to the C library's `abs`.
    let call_abs = unsafe { jit.function::<CallAbs>("call_abs") }?;

    let result = call_abs.call((-5,));

    assert_eq!(result, 5);
    Ok(())
}
