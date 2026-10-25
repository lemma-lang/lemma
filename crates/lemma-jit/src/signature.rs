//! The closed set of function signatures a [`Function`](crate::Function) can be called through.

use std::mem;
use std::ptr;

mod sealed {
    pub trait Sealed {}
}

/// A value type that crosses the C calling boundary by value: a fixed-width integer or a float.
///
/// Sealed. `bool` is absent because an out-of-range byte is undefined behavior on the Rust side,
/// and so are pointers and `usize`, which would let the callee reach Rust memory.
pub trait Scalar: sealed::Sealed {}

/// A type a function may return: any [`Scalar`], or `()`.
///
/// Sealed.
pub trait Return: sealed::Sealed {}

/// An `extern "C" fn` type with up to four [`Scalar`] parameters and a [`Return`] result.
///
/// Sealed: it is implemented for exactly those function pointer types.
pub trait Signature: sealed::Sealed + Copy {
    /// The parameters as a tuple, `()` for none and `(A,)` for one.
    type Arguments;
    /// What the function returns.
    type Output;

    /// Reinterprets `address` as a function of this signature.
    ///
    /// # Safety
    ///
    /// `address` must be the entry point of a function with the C calling convention and exactly
    /// this signature, and it must stay mapped and executable for as long as the result is called.
    #[doc(hidden)]
    unsafe fn from_address(address: usize) -> Self;

    /// Calls the function with `arguments`.
    #[doc(hidden)]
    fn invoke(self, arguments: Self::Arguments) -> Self::Output;
}

macro_rules! implement_scalars {
    ($($scalar:ty),*) => {
        $(
            impl sealed::Sealed for $scalar {}
            impl Scalar for $scalar {}
            impl Return for $scalar {}
        )*
    };
}

implement_scalars!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

impl sealed::Sealed for () {}
impl Return for () {}

macro_rules! implement_signature {
    ($(($parameter:ident, $argument:ident)),*) => {
        impl<ReturnType: Return, $($parameter: Scalar),*> sealed::Sealed
            for extern "C" fn($($parameter),*) -> ReturnType
        {
        }

        impl<ReturnType: Return, $($parameter: Scalar),*> Signature
            for extern "C" fn($($parameter),*) -> ReturnType
        {
            type Arguments = ($($parameter,)*);
            type Output = ReturnType;

            unsafe fn from_address(address: usize) -> Self {
                let entry = ptr::with_exposed_provenance::<()>(address);
                // SAFETY: the caller guarantees `address` is the entry point of a function of
                // exactly this signature; a function pointer and a data pointer have the same size.
                unsafe { mem::transmute::<*const (), Self>(entry) }
            }

            fn invoke(self, ($($argument,)*): Self::Arguments) -> ReturnType {
                self($($argument),*)
            }
        }
    };
}

implement_signature!();
implement_signature!((A0, a0));
implement_signature!((A0, a0), (A1, a1));
implement_signature!((A0, a0), (A1, a1), (A2, a2));
implement_signature!((A0, a0), (A1, a1), (A2, a2), (A3, a3));
