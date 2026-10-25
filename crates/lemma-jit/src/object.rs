//! The identity of a relocatable object, read from its header before ORC sees the bytes.

use lemma_codegen::Target;

use crate::JitError;

/// The first six bytes of a 64-bit little-endian ELF file: magic, class, data encoding.
const ELF_IDENTIFICATION: [u8; 6] = [0x7F, b'E', b'L', b'F', 2, 1];
const ELF_TYPE_OFFSET: usize = 16;
const ELF_MACHINE_OFFSET: usize = 18;
const ELF_TYPE_RELOCATABLE: u16 = 1;
const ELF_MACHINE_X86_64: u16 = 62;
const ELF_MACHINE_AARCH64: u16 = 183;

const MACH_O_MAGIC_64: u32 = 0xFEED_FACF;
const MACH_O_CPU_TYPE_OFFSET: usize = 4;
const MACH_O_FILE_TYPE_OFFSET: usize = 12;
const MACH_O_FILE_TYPE_OBJECT: u32 = 1;
const MACH_O_CPU_TYPE_ARM64: u32 = 0x0100_000C;

/// The container format and machine of a relocatable object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Identity {
    Elf { machine: u16 },
    MachO { cpu_type: u32 },
}

/// Refuses bytes that are not a relocatable object built for `target`.
///
/// Only the header fields that name the format, the kind and the machine are read; the rest of
/// the file is left to LLVM, which rejects a truncated or malformed object itself.
pub(crate) fn check(name: &str, object: &[u8], target: Target) -> Result<(), JitError> {
    let found = identify(object).ok_or_else(|| JitError::NotAnObject {
        name: name.to_string(),
    })?;
    if found == expected(target) {
        return Ok(());
    }
    Err(JitError::ObjectTargetMismatch {
        name: name.to_string(),
        expected: target,
    })
}

fn expected(target: Target) -> Identity {
    match target {
        Target::AppleAarch64 => Identity::MachO {
            cpu_type: MACH_O_CPU_TYPE_ARM64,
        },
        Target::X86_64Linux => Identity::Elf {
            machine: ELF_MACHINE_X86_64,
        },
        Target::Aarch64Linux => Identity::Elf {
            machine: ELF_MACHINE_AARCH64,
        },
    }
}

fn identify(object: &[u8]) -> Option<Identity> {
    elf_identity(object).or_else(|| mach_o_identity(object))
}

fn elf_identity(object: &[u8]) -> Option<Identity> {
    if object.get(..ELF_IDENTIFICATION.len())? != ELF_IDENTIFICATION {
        return None;
    }
    if read_u16(object, ELF_TYPE_OFFSET)? != ELF_TYPE_RELOCATABLE {
        return None;
    }
    Some(Identity::Elf {
        machine: read_u16(object, ELF_MACHINE_OFFSET)?,
    })
}

fn mach_o_identity(object: &[u8]) -> Option<Identity> {
    if read_u32(object, 0)? != MACH_O_MAGIC_64 {
        return None;
    }
    if read_u32(object, MACH_O_FILE_TYPE_OFFSET)? != MACH_O_FILE_TYPE_OBJECT {
        return None;
    }
    Some(Identity::MachO {
        cpu_type: read_u32(object, MACH_O_CPU_TYPE_OFFSET)?,
    })
}

fn read_u16(object: &[u8], offset: usize) -> Option<u16> {
    let bytes = object.get(offset..)?.first_chunk::<2>()?;
    Some(u16::from_le_bytes(*bytes))
}

fn read_u32(object: &[u8], offset: usize) -> Option<u32> {
    let bytes = object.get(offset..)?.first_chunk::<4>()?;
    Some(u32::from_le_bytes(*bytes))
}
