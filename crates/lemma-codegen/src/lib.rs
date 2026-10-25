//! LLVM IR emission with aliasing and alignment metadata.

mod object;
mod target;

pub use object::{EmitError, emit_object};
pub use target::{Target, TargetError};

#[cfg(test)]
mod tests;
