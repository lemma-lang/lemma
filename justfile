# Single entry point for every development task. CI runs the same recipes.

default:
    @just --list

# Install git hooks and check that the toolchain is available.
setup:
    cog install-hook --all --overwrite
    rustup show active-toolchain
    /opt/homebrew/opt/llvm@22/bin/llvm-config --version || echo "LLVM 22 not found: brew install llvm@22"

# Format Rust and TOML files.
fmt:
    cargo fmt --all
    taplo fmt

# Run every check CI runs.
check: fmt-check lint test doc deny spell rules

fmt-check:
    cargo fmt --all --check
    taplo fmt --check

lint:
    cargo clippy --locked --workspace --all-targets -- -D warnings

test:
    cargo nextest run --locked --workspace --no-fail-fast
    cargo test --locked --workspace --doc

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps

deny:
    cargo deny --locked check

spell:
    typos

# Shape rules' own test cases; the local agent guards' suite when it is installed.
rules:
    ast-grep test -c tools/rules/sgconfig.yml --skip-snapshot-tests
    if [ -x .claude/hooks/tests/run.sh ]; then .claude/hooks/tests/run.sh; fi

# Code coverage, HTML report in target/llvm-cov/html.
cov:
    cargo llvm-cov nextest --workspace --html
