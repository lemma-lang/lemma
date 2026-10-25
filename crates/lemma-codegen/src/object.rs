//! Object emission: the one route from an LLVM module to the bytes of a relocatable object.

use std::fmt;

use inkwell::module::Module;
use inkwell::targets::FileType;

use crate::target::{Target, TargetError};

/// Why a module could not be emitted as an object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitError {
    /// The target machine could not be built.
    Target(TargetError),
    /// LLVM's verifier rejected the module.
    InvalidModule {
        /// The verifier's text, unchanged.
        message: String,
    },
    /// LLVM failed while writing the object.
    Emission {
        /// What LLVM reported.
        message: String,
    },
}

impl fmt::Display for EmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Target(error) => error.fmt(formatter),
            Self::InvalidModule { message } => write!(formatter, "invalid module: {message}"),
            Self::Emission { message } => write!(formatter, "object emission failed: {message}"),
        }
    }
}

impl std::error::Error for EmitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Target(error) => Some(error),
            Self::InvalidModule { .. } | Self::Emission { .. } => None,
        }
    }
}

impl From<TargetError> for EmitError {
    fn from(error: TargetError) -> Self {
        Self::Target(error)
    }
}

/// The relocatable object of `module` for `target`.
///
/// The module's triple and data layout are set from the target machine before emission.
///
/// # Errors
///
/// [`EmitError::Target`] when the target machine cannot be built, [`EmitError::InvalidModule`]
/// when the module does not verify, [`EmitError::Emission`] when LLVM cannot write the object.
pub fn emit_object(module: &Module<'_>, target: Target) -> Result<Vec<u8>, EmitError> {
    let machine = target.target_machine()?;
    module.set_triple(&machine.get_triple());
    module.set_data_layout(&machine.get_target_data().get_data_layout());
    module
        .verify()
        .map_err(|message| EmitError::InvalidModule {
            message: message.to_string(),
        })?;
    let buffer = machine
        .write_to_memory_buffer(module, FileType::Object)
        .map_err(|message| EmitError::Emission {
            message: message.to_string(),
        })?;
    Ok(buffer.as_slice().to_vec())
}
