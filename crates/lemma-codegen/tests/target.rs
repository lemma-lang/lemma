//! The pinned configuration of each target, and the target machine built from it.

use lemma_codegen::Target;

/// Triple, CPU and features of each target, as `docs/adr/0003-targets.md` pins them.
const PINNED: [(Target, &str, &str, &str); 3] = [
    (Target::AppleAarch64, "aarch64-apple-darwin", "apple-m1", ""),
    (
        Target::X86_64Linux,
        "x86_64-unknown-linux-gnu",
        "x86-64-v3",
        "",
    ),
    (
        Target::Aarch64Linux,
        "aarch64-unknown-linux-gnu",
        "generic",
        "+neon,+fp-armv8",
    ),
];

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
const RUNNING_HOST: Option<Target> = Some(Target::AppleAarch64);
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
const RUNNING_HOST: Option<Target> = Some(Target::X86_64Linux);
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
const RUNNING_HOST: Option<Target> = Some(Target::Aarch64Linux);
#[cfg(not(any(
    all(target_arch = "aarch64", target_os = "macos"),
    all(target_arch = "x86_64", target_os = "linux"),
    all(target_arch = "aarch64", target_os = "linux"),
)))]
const RUNNING_HOST: Option<Target> = None;

type Configuration = (Target, String, String, String);

fn owned((target, triple, cpu, features): (Target, &str, &str, &str)) -> Configuration {
    (
        target,
        triple.to_string(),
        cpu.to_string(),
        features.to_string(),
    )
}

fn machine_configuration(target: Target) -> Configuration {
    let machine = target.target_machine().unwrap();
    (
        target,
        machine.get_triple().as_str().to_str().unwrap().to_string(),
        machine.get_cpu().to_string(),
        machine.get_feature_string().to_str().unwrap().to_string(),
    )
}

#[test]
fn each_target_carries_its_pinned_triple_cpu_and_features() {
    let configured =
        PINNED.map(|(target, ..)| (target, target.triple(), target.cpu(), target.features()));

    assert_eq!(configured, PINNED);
}

#[test]
fn each_target_machine_is_built_from_its_pinned_triple_cpu_and_features() {
    let machines = PINNED.map(|(target, ..)| machine_configuration(target));

    assert_eq!(machines, PINNED.map(owned));
}

#[test]
fn host_is_the_target_of_the_machine_running_the_suite() {
    let host = Target::host();

    assert_eq!(host, RUNNING_HOST);
}
