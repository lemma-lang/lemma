//! The pinned configuration of each target, and the target machine built from it.

use std::fmt;
use std::sync::Once;

use inkwell::OptimizationLevel;
use inkwell::targets::{
    CodeModel, InitializationConfig, RelocMode, Target as LlvmTarget, TargetMachine, TargetTriple,
};

static INITIALIZE_LLVM_TARGETS: Once = Once::new();

/// A target the compiler is pinned to: triple, CPU and features never come from the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// Apple Silicon running macOS.
    AppleAarch64,
    /// 64-bit x86 running Linux with the glibc ABI.
    X86_64Linux,
    /// 64-bit Arm running Linux with the glibc ABI.
    Aarch64Linux,
}

/// Why a target machine could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetError {
    /// The target whose machine was requested.
    pub target: Target,
    /// What LLVM reported, or a description when it reported nothing.
    pub message: String,
}

impl fmt::Display for TargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot build a target machine for {}: {}",
            self.target.triple(),
            self.message
        )
    }
}

impl std::error::Error for TargetError {}

impl Target {
    /// The LLVM target triple.
    pub const fn triple(self) -> &'static str {
        match self {
            Self::AppleAarch64 => "aarch64-apple-darwin",
            Self::X86_64Linux => "x86_64-unknown-linux-gnu",
            Self::Aarch64Linux => "aarch64-unknown-linux-gnu",
        }
    }

    /// The LLVM CPU name code is generated for.
    pub const fn cpu(self) -> &'static str {
        match self {
            Self::AppleAarch64 => "apple-m1",
            Self::X86_64Linux => "x86-64-v3",
            Self::Aarch64Linux => "generic",
        }
    }

    /// The LLVM feature string, empty when the CPU name says it all.
    pub const fn features(self) -> &'static str {
        match self {
            Self::AppleAarch64 | Self::X86_64Linux => "",
            Self::Aarch64Linux => "+neon,+fp-armv8",
        }
    }

    /// The pinned target matching the machine running this process, if there is one.
    pub fn host() -> Option<Self> {
        match (std::env::consts::ARCH, std::env::consts::OS) {
            ("aarch64", "macos") => Some(Self::AppleAarch64),
            ("x86_64", "linux") => Some(Self::X86_64Linux),
            ("aarch64", "linux") => Some(Self::Aarch64Linux),
            _ => None,
        }
    }

    /// An LLVM target machine for this target: position-independent code, small code model,
    /// highest optimization level.
    ///
    /// Every LLVM target is initialized on the first call, so any target can be built on any host.
    ///
    /// # Errors
    ///
    /// [`TargetError`] when LLVM does not know the triple or refuses the configuration.
    pub fn target_machine(self) -> Result<TargetMachine, TargetError> {
        INITIALIZE_LLVM_TARGETS
            .call_once(|| LlvmTarget::initialize_all(&InitializationConfig::default()));
        let triple = TargetTriple::create(self.triple());
        let llvm_target = LlvmTarget::from_triple(&triple).map_err(|message| TargetError {
            target: self,
            message: message.to_string(),
        })?;
        llvm_target
            .create_target_machine(
                &triple,
                self.cpu(),
                self.features(),
                OptimizationLevel::Aggressive,
                RelocMode::PIC,
                CodeModel::Small,
            )
            .ok_or_else(|| TargetError {
                target: self,
                message: "LLVM returned no target machine".to_string(),
            })
    }
}
