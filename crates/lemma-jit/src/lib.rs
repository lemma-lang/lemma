//! In-memory JIT daemon: loading, hot replacement, cost measurement.

mod object;
mod signature;

use std::ffi::{CStr, CString};
use std::fmt;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr;

use lemma_codegen::{Target, TargetError};
use llvm_sys::core::LLVMCreateMemoryBufferWithMemoryRangeCopy;
use llvm_sys::error::{
    LLVMConsumeError, LLVMDisposeErrorMessage, LLVMErrorRef, LLVMGetErrorMessage,
};
use llvm_sys::orc2::LLVMOrcJITTargetMachineBuilderCreateFromTargetMachine;
use llvm_sys::orc2::lljit::{
    LLVMOrcCreateLLJIT, LLVMOrcCreateLLJITBuilder, LLVMOrcDisposeLLJIT, LLVMOrcLLJITAddObjectFile,
    LLVMOrcLLJITBuilderSetJITTargetMachineBuilder, LLVMOrcLLJITGetMainJITDylib, LLVMOrcLLJITLookup,
    LLVMOrcLLJITRef,
};

pub use signature::{Return, Scalar, Signature};

/// Why a [`Jit`] could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JitError {
    /// The requested target is not the machine running the process, so its code could not run.
    TargetNotHost {
        /// The target that was requested.
        target: Target,
        /// The pinned target of the running machine, `None` when it has none.
        host: Option<Target>,
    },
    /// The target machine for the JIT could not be built.
    TargetMachine(TargetError),
    /// LLVM refused to create the JIT.
    Creation {
        /// What LLVM reported.
        message: String,
    },
    /// An object name contains a NUL byte, which LLVM's C strings cannot hold.
    InvalidName {
        /// The rejected name.
        name: String,
    },
    /// The bytes are not a relocatable object: too short, another file type, or an unknown format.
    NotAnObject {
        /// The name the bytes were added under.
        name: String,
    },
    /// The object is a valid relocatable object, but for another target than the JIT's.
    ObjectTargetMismatch {
        /// The name the object was added under.
        name: String,
        /// The target the JIT compiles for, which the object does not match.
        expected: Target,
    },
    /// LLVM refused to load an object.
    ObjectRejected {
        /// The name the object was added under.
        name: String,
        /// What LLVM reported.
        message: String,
    },
    /// A symbol could not be resolved to a function.
    Lookup {
        /// The symbol that was looked up.
        symbol: String,
        /// What LLVM reported, or a description when it reported nothing.
        message: String,
    },
}

impl fmt::Display for JitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetNotHost { target, host } => match host {
                Some(host) => write!(
                    formatter,
                    "cannot JIT for {}: the host is {}",
                    target.triple(),
                    host.triple()
                ),
                None => write!(
                    formatter,
                    "cannot JIT for {}: the host has no pinned target",
                    target.triple()
                ),
            },
            Self::TargetMachine(error) => error.fmt(formatter),
            Self::Creation { message } => write!(formatter, "cannot create the JIT: {message}"),
            Self::InvalidName { name } => {
                write!(formatter, "object name {name:?} contains a NUL byte")
            }
            Self::NotAnObject { name } => {
                write!(formatter, "{name} is not a relocatable object")
            }
            Self::ObjectTargetMismatch { name, expected } => {
                write!(
                    formatter,
                    "object {name} is not built for {}",
                    expected.triple()
                )
            }
            Self::ObjectRejected { name, message } => {
                write!(formatter, "cannot load object {name}: {message}")
            }
            Self::Lookup { symbol, message } => {
                write!(formatter, "cannot resolve symbol {symbol}: {message}")
            }
        }
    }
}

impl std::error::Error for JitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TargetMachine(error) => Some(error),
            Self::TargetNotHost { .. }
            | Self::Creation { .. }
            | Self::InvalidName { .. }
            | Self::NotAnObject { .. }
            | Self::ObjectTargetMismatch { .. }
            | Self::ObjectRejected { .. }
            | Self::Lookup { .. } => None,
        }
    }
}

/// An ORC LLJIT compiling for one pinned target, which is the host.
///
/// It is neither `Send` nor `Sync`: the underlying LLVM handle is used from one thread only.
#[derive(Debug)]
pub struct Jit {
    handle: LLVMOrcLLJITRef,
    target: Target,
}

impl Jit {
    /// Creates a JIT for `target`, which must be the pinned target of the running machine.
    ///
    /// The pinned CPU is not compared with the host CPU: running code built for a CPU newer than
    /// the host's is the caller's responsibility.
    ///
    /// # Errors
    ///
    /// [`JitError::TargetNotHost`] when `target` is not the host, [`JitError::TargetMachine`]
    /// when its target machine cannot be built, [`JitError::Creation`] when LLVM refuses the JIT.
    pub fn new(target: Target) -> Result<Self, JitError> {
        let host = Target::host();
        if host != Some(target) {
            return Err(JitError::TargetNotHost { target, host });
        }
        let target_machine =
            ManuallyDrop::new(target.target_machine().map_err(JitError::TargetMachine)?);
        // SAFETY: the target machine is live, and this call destroys it: `ManuallyDrop` keeps
        // Rust from destroying it a second time.
        let machine_builder = unsafe {
            LLVMOrcJITTargetMachineBuilderCreateFromTargetMachine(target_machine.as_mut_ptr())
        };
        // SAFETY: creating a builder has no precondition; it is owned here until `LLVMOrcCreateLLJIT`.
        let jit_builder = unsafe { LLVMOrcCreateLLJITBuilder() };
        // SAFETY: both handles are live and unshared; the builder takes ownership of the machine builder.
        unsafe { LLVMOrcLLJITBuilderSetJITTargetMachineBuilder(jit_builder, machine_builder) };
        let mut handle: LLVMOrcLLJITRef = ptr::null_mut();
        // SAFETY: `handle` is a valid out-pointer and the builder is live; the call takes ownership
        // of the builder whether it succeeds or not, so it is not used again.
        let error = unsafe { LLVMOrcCreateLLJIT(&mut handle, jit_builder) };
        if !error.is_null() {
            return Err(JitError::Creation {
                message: take_error_message(error),
            });
        }
        Ok(Self { handle, target })
    }

    /// Loads a relocatable object into the JIT under `name`, which locates it in errors.
    ///
    /// The bytes are copied, so `object` need not outlive the call. They must come from
    /// [`lemma_codegen::emit_object`] for this JIT's target, or from a producer the caller
    /// trusts: the header check sees the format and the machine but not the CPU, so an object
    /// built for another CPU of the same architecture loads and may fault when called, and the
    /// bytes past the header go to LLVM's object parser and JITLink, which are not promised safe
    /// on hostile input.
    ///
    /// Undefined symbols the object refers to are resolved against the host process's symbols,
    /// so a call from the object to a libc function links.
    ///
    /// # Errors
    ///
    /// [`JitError::NotAnObject`] when the bytes are not a relocatable object,
    /// [`JitError::ObjectTargetMismatch`] when it is built for another target,
    /// [`JitError::InvalidName`] when `name` contains a NUL byte, [`JitError::ObjectRejected`]
    /// when LLVM refuses the object, for instance because it is truncated or because it defines
    /// a symbol that is already loaded.
    pub fn add_object(&mut self, name: &str, object: &[u8]) -> Result<(), JitError> {
        object::check(name, object, self.target)?;
        let buffer_name = CString::new(name).map_err(|_| JitError::InvalidName {
            name: name.to_string(),
        })?;
        // SAFETY: `object` is readable for its whole length and `buffer_name` is NUL-terminated;
        // both are copied, so neither is needed after the call.
        let buffer = unsafe {
            LLVMCreateMemoryBufferWithMemoryRangeCopy(
                object.as_ptr().cast(),
                object.len(),
                buffer_name.as_ptr(),
            )
        };
        // SAFETY: `handle` is a live LLJIT, which always has a main JITDylib it owns.
        let main_dylib = unsafe { LLVMOrcLLJITGetMainJITDylib(self.handle) };
        // SAFETY: `handle` and `main_dylib` are live and `buffer` is a fresh buffer; the call takes
        // ownership of the buffer whether it succeeds or not, so it is not touched again.
        let error = unsafe { LLVMOrcLLJITAddObjectFile(self.handle, main_dylib, buffer) };
        if error.is_null() {
            return Ok(());
        }
        Err(JitError::ObjectRejected {
            name: name.to_string(),
            message: take_error_message(error),
        })
    }

    /// Looks up `symbol` among the loaded objects and returns it as a callable function.
    ///
    /// `symbol` is the name as written in the object, without the platform's leading underscore.
    /// Only symbols defined by loaded objects are found: a host symbol such as a libc function
    /// cannot be looked up here, although objects are linked against the host process's symbols.
    ///
    /// # Errors
    ///
    /// [`JitError::Lookup`] when `symbol` contains a NUL byte, no loaded object defines it, or
    /// it resolves to no address (address 0).
    ///
    /// # Safety
    ///
    /// If `symbol` resolves, it must be a function with the C calling convention and exactly
    /// the parameter and return types of `F`, and running it for any argument values must have no
    /// undefined behavior and touch no memory outside its own stack and the objects loaded into
    /// this JIT.
    pub unsafe fn function<F: Signature>(&self, symbol: &str) -> Result<Function<'_, F>, JitError> {
        let lookup_error = |message: String| JitError::Lookup {
            symbol: symbol.to_string(),
            message,
        };
        let symbol_name = CString::new(symbol)
            .map_err(|_| lookup_error("the symbol contains a NUL byte".to_string()))?;
        let mut address = 0;
        // SAFETY: `handle` is live, `address` is a valid out-pointer and `symbol_name` is
        // NUL-terminated and outlives the call.
        let error = unsafe { LLVMOrcLLJITLookup(self.handle, &mut address, symbol_name.as_ptr()) };
        if !error.is_null() {
            return Err(lookup_error(take_error_message(error)));
        }
        let address = usize::try_from(address)
            .ok()
            .filter(|&address| address != 0)
            .ok_or_else(|| lookup_error("the symbol resolved to no address".to_string()))?;
        // SAFETY: the caller guarantees the symbol is a function of signature `F`; the code stays
        // mapped until the JIT is dropped, and the returned `Function` borrows the JIT.
        let function = unsafe { F::from_address(address) };
        Ok(Function {
            function,
            jit: PhantomData,
        })
    }
}

/// A function loaded into a [`Jit`], callable while that JIT is alive.
///
/// Dropping the JIT while a function borrowed from it exists does not compile:
///
/// ```compile_fail,E0505
/// use lemma_codegen::Target;
/// use lemma_jit::Jit;
///
/// let jit = Jit::new(Target::host().unwrap()).unwrap();
/// // SAFETY: never reached, the example does not compile.
/// let function = unsafe { jit.function::<extern "C" fn() -> i64>("main") }.unwrap();
/// drop(jit);
/// function.call(());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Function<'jit, F> {
    function: F,
    jit: PhantomData<&'jit Jit>,
}

impl<F: Signature> Function<'_, F> {
    /// Calls the function with `arguments`, a tuple of its parameters.
    pub fn call(&self, arguments: F::Arguments) -> F::Output {
        self.function.invoke(arguments)
    }
}

impl Drop for Jit {
    fn drop(&mut self) {
        // SAFETY: `handle` came from a successful `LLVMOrcCreateLLJIT` and is disposed once, here.
        let error = unsafe { LLVMOrcDisposeLLJIT(self.handle) };
        if error.is_null() {
            return;
        }
        // SAFETY: `error` is a non-null error owned here; drop cannot report it, so it is consumed.
        unsafe { LLVMConsumeError(error) };
    }
}

/// Turns a non-null LLVM error into its message, consuming the error.
fn take_error_message(error: LLVMErrorRef) -> String {
    // SAFETY: `error` is non-null and owned by the caller; `LLVMGetErrorMessage` consumes it, so it
    // must not also be passed to `LLVMConsumeError`.
    let raw_message = unsafe { LLVMGetErrorMessage(error) };
    // SAFETY: `raw_message` is a NUL-terminated string that stays valid until it is disposed below.
    let message = unsafe { CStr::from_ptr(raw_message) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: `raw_message` came from `LLVMGetErrorMessage` and is not used after this call.
    unsafe { LLVMDisposeErrorMessage(raw_message) };
    message
}
