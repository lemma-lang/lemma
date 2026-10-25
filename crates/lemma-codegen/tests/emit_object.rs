//! Object emission for each pinned target: the header of the emitted bytes, and a refused module.

use std::error::Error;

use inkwell::context::Context;
use inkwell::module::Module;
use lemma_codegen::{EmitError, Target, emit_object};

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LITTLE_ENDIAN: u8 = 1;
const ELF_TYPE_RELOCATABLE: u16 = 1;
const ELF_MACHINE_X86_64: u16 = 62;
const ELF_MACHINE_AARCH64: u16 = 183;
const MACH_O_MAGIC_64: u32 = 0xFEED_FACF;
const MACH_O_CPU_TYPE_ARM64: u32 = 0x0100_000C;
const MACH_O_FILE_TYPE_OBJECT: u32 = 1;

fn add_module(context: &Context) -> Result<Module<'_>, Box<dyn Error>> {
    let module = context.create_module("add");
    let i64_type = context.i64_type();
    let signature = i64_type.fn_type(&[i64_type.into(), i64_type.into()], false);
    let function = module.add_function("add", signature, None);
    let builder = context.create_builder();
    builder.position_at_end(context.append_basic_block(function, "entry"));
    let left = function
        .get_nth_param(0)
        .ok_or("add has no first parameter")?
        .into_int_value();
    let right = function
        .get_nth_param(1)
        .ok_or("add has no second parameter")?
        .into_int_value();
    let sum = builder.build_int_add(left, right, "sum")?;
    builder.build_return(Some(&sum))?;
    Ok(module)
}

fn add_object(target: Target) -> Result<Vec<u8>, Box<dyn Error>> {
    let context = Context::create();
    Ok(emit_object(&add_module(&context)?, target)?)
}

/// Magic, class, data encoding, file type and machine of an ELF header.
type ElfHeader = ([u8; 4], u8, u8, u16, u16);

fn elf_header(object: &[u8]) -> Result<ElfHeader, Box<dyn Error>> {
    Ok((
        object[0..4].try_into()?,
        object[4],
        object[5],
        u16::from_le_bytes(object[16..18].try_into()?),
        u16::from_le_bytes(object[18..20].try_into()?),
    ))
}

/// Magic, CPU type and file type of a 64-bit little-endian Mach-O header.
fn mach_o_header(object: &[u8]) -> Result<(u32, u32, u32), Box<dyn Error>> {
    Ok((
        u32::from_le_bytes(object[0..4].try_into()?),
        u32::from_le_bytes(object[4..8].try_into()?),
        u32::from_le_bytes(object[12..16].try_into()?),
    ))
}

#[test]
fn x86_64_linux_emits_a_relocatable_elf64_object_for_x86_64() -> Result<(), Box<dyn Error>> {
    let object = add_object(Target::X86_64Linux)?;

    assert_eq!(
        elf_header(&object)?,
        (
            ELF_MAGIC,
            ELF_CLASS_64,
            ELF_DATA_LITTLE_ENDIAN,
            ELF_TYPE_RELOCATABLE,
            ELF_MACHINE_X86_64,
        )
    );
    Ok(())
}

#[test]
fn aarch64_linux_emits_a_relocatable_elf64_object_for_aarch64() -> Result<(), Box<dyn Error>> {
    let object = add_object(Target::Aarch64Linux)?;

    assert_eq!(
        elf_header(&object)?,
        (
            ELF_MAGIC,
            ELF_CLASS_64,
            ELF_DATA_LITTLE_ENDIAN,
            ELF_TYPE_RELOCATABLE,
            ELF_MACHINE_AARCH64,
        )
    );
    Ok(())
}

#[test]
fn apple_aarch64_emits_a_mach_o_object_for_arm64() -> Result<(), Box<dyn Error>> {
    let object = add_object(Target::AppleAarch64)?;

    assert_eq!(
        mach_o_header(&object)?,
        (
            MACH_O_MAGIC_64,
            MACH_O_CPU_TYPE_ARM64,
            MACH_O_FILE_TYPE_OBJECT
        )
    );
    Ok(())
}

#[test]
fn a_function_without_a_terminator_is_an_invalid_module_naming_that_function() {
    let context = Context::create();
    let module = context.create_module("unterminated");
    let function =
        module.add_function("unterminated", context.i64_type().fn_type(&[], false), None);
    context.append_basic_block(function, "entry");

    let result = emit_object(&module, Target::X86_64Linux);

    assert!(matches!(
        result,
        Err(EmitError::InvalidModule { message }) if message.contains("'unterminated'")
    ));
}
