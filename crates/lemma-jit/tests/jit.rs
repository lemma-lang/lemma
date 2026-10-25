//! Creating a JIT for a pinned target, refusing a target that is not the host, and dropping it.

use std::error::Error;

use lemma_codegen::Target;
use lemma_jit::{Jit, JitError};

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
const FOREIGN: Target = Target::X86_64Linux;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
const FOREIGN: Target = Target::Aarch64Linux;
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
const FOREIGN: Target = Target::X86_64Linux;

fn host() -> Result<Target, Box<dyn Error>> {
    Ok(Target::host().ok_or("the suite runs on a host with no pinned target")?)
}

#[test]
fn a_jit_is_created_for_the_host_target() -> Result<(), Box<dyn Error>> {
    let host = host()?;

    let result = Jit::new(host);

    assert!(result.is_ok(), "{:?}", result.err());
    Ok(())
}

#[test]
fn a_jit_for_a_foreign_target_is_refused_with_both_targets() -> Result<(), Box<dyn Error>> {
    let host = host()?;

    let result = Jit::new(FOREIGN);

    assert!(matches!(
        result,
        Err(JitError::TargetNotHost { target: FOREIGN, host: Some(reported) }) if reported == host
    ));
    Ok(())
}

#[test]
fn a_jit_is_created_after_two_jits_were_created_and_dropped() -> Result<(), Box<dyn Error>> {
    let host = host()?;
    let first = Jit::new(host)?;
    let second = Jit::new(host)?;
    drop(first);
    drop(second);

    let third = Jit::new(host);

    assert!(third.is_ok(), "{:?}", third.err());
    Ok(())
}
