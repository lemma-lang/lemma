#!/usr/bin/env bash
#
# Feeds sample PreToolUse events to both guards, the way the harness does, and
# checks the outcome. Rerun after any change to the hooks, test-paths.txt or
# the wiring: it is the only way to tell a guard that protects from one that
# no longer exists.
#
#   .claude/hooks/tests/run.sh
#
# Each guard must exit with exactly 0 (allow) or 2 (refuse): any other status
# would let the tool call through, and is a failure here.
#
# bash 3.2.

set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd -P)
hooks=$(cd "$here/.." && pwd -P)
root=$(cd "$hooks/../.." && pwd -P)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

pass=0
failed=0

# $1 = role ("" for the main session), $2 = tool, $3 = path or command, $4 = cwd
event() {
  if [ "$2" = "Bash" ]; then
    jq -n --arg r "$1" --arg c "$3" --arg d "$4" \
      '{tool_name: "Bash", tool_input: {command: $c}, cwd: $d} + (if $r == "" then {} else {agent_type: $r} end)'
  else
    jq -n --arg r "$1" --arg t "$2" --arg f "$3" --arg d "$4" \
      '{tool_name: $t, tool_input: {file_path: $f, content: "x"}, cwd: $d} + (if $r == "" then {} else {agent_type: $r} end)'
  fi
}

# Runs both guards on a payload; prints "allow" or "deny", or "broken:<hook>:<status>".
verdict() { # $1 = payload, extra env in the caller
  local outcome=allow hook status
  for hook in path-guard.sh role-guard.sh; do
    printf '%s' "$1" | "$hooks/$hook" >/dev/null 2>"$tmp/stderr"
    status=$?
    case "$status" in
      0) ;;
      2) outcome=deny ;;
      *) printf 'broken:%s:%s' "$hook" "$status"; return ;;
    esac
  done
  printf '%s' "$outcome"
}

check() { # $1 = expected, $2 = description, $3 = payload
  local got
  got=$(verdict "$3")
  if [ "$got" = "$1" ]; then
    pass=$((pass + 1))
  else
    failed=$((failed + 1))
    printf 'FAIL  %-6s got %-6s %s\n' "$1" "$got" "$2"
  fi
}

case_() { # $1 = expected, $2 = role, $3 = tool, $4 = path or command, [$5 = cwd]
  local cwd=${5:-$root}
  check "$1" "${2:-main}: $3 $4" "$(event "$2" "$3" "$4" "$cwd")"
}

R=$root

# --- The TDD loop -------------------------------------------------------------
case_ allow ""     Write "$R/crates/lemma-codegen/src/tests.rs"
case_ deny  coder  Write "$R/crates/lemma-codegen/src/tests.rs"
case_ allow tester Write "$R/crates/lemma-codegen/src/tests.rs"
case_ allow coder  Write "$R/crates/lemma-codegen/src/lib.rs"
case_ deny  tester Write "$R/crates/lemma-codegen/src/lib.rs"
case_ deny  coder  Edit  "$R/crates/lemma-syntax/src/parser/tests.rs"
case_ deny  coder  Write "$R/crates/lemma-syntax/tests/parse.rs"
case_ allow tester Write "$R/crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Write "$R/crates/lemma-codegen/snapshots/dot.snap"
case_ deny  coder  Write "$R/crates/lemma-verify/testdata/vacuous.lm"
case_ deny  coder  Write "$R/fuzz/fuzz_targets/parse.rs"
case_ allow tester Write "$R/fuzz/fuzz_targets/parse.rs"
case_ deny  coder  Write "src/tests.rs" "$R/crates/lemma-codegen"
case_ deny  coder  Write "$R/crates/lemma-codegen/../lemma-codegen/src/tests.rs"
case_ deny  coder  Write "$R/.claude/plans/LEM-1.md"
case_ allow planner Write "$R/.claude/plans/LEM-1.md"
case_ deny  planner Write "$R/crates/lemma-syntax/src/lib.rs"
case_ allow reviewer Write "$R/.claude/reviews/LEM-1.md"
case_ deny  reviewer Edit "$R/crates/lemma-syntax/src/lib.rs"
case_ allow debugger Write "$R/.claude/diagnostics/z3-timeout.md"
case_ deny  debugger Write "$R/crates/lemma-verify/src/lib.rs"

# --- What the coder may not loosen -------------------------------------------
case_ deny  coder Write "$R/clippy.toml"
case_ deny  coder Edit  "$R/.gitlab-ci.yml"
case_ deny  coder Edit  "$R/deny.toml"
case_ deny  coder Edit  "$R/.cargo/config.toml"
case_ deny  coder Write "$R/crates/lemma-codegen/benches/dot.rs"
case_ deny  coder Edit  "$R/docs/adr/0002-scope.md"
case_ deny  coder Edit  "$R/docs/design.md"
case_ allow coder Edit  "$R/README.md"
case_ allow coder Edit  "$R/examples/dot.lm"
case_ allow coder Edit  "$R/Cargo.toml"

# --- The setup itself ---------------------------------------------------------
case_ deny  coder Write "$R/.claude/test-paths.txt"
case_ deny  tester Write "$R/.claude/test-paths.txt"
case_ deny  release-engineer Edit "$R/.claude/hooks/path-guard.sh"
case_ deny  coder Edit  "$R/.claude/settings.json"
case_ deny  coder Edit  "$R/.claude/agents/coder.md"
case_ deny  coder Edit  "$R/CLAUDE.md"
case_ deny  language-designer Edit "$R/AGENTS.md"
case_ allow ""    Edit  "$R/.claude/hooks/path-guard.sh"

# --- Shell: the coder ---------------------------------------------------------
case_ deny  coder  Bash "cargo insta accept"
case_ deny  coder  Bash "cargo insta review"
case_ deny  coder  Bash "cargo insta test --accept"
case_ deny  coder  Bash "INSTA_UPDATE=always cargo test -p lemma-codegen"
case_ deny  coder  Bash "INSTA_FORCE_PASS=1 cargo test"
case_ deny  coder  Bash "UPDATE_EXPECT=1 cargo test"
case_ deny  coder  Bash "BLESS=1 cargo test"
case_ deny  coder  Bash "TRYBUILD=overwrite cargo test"
case_ allow coder  Bash "INSTA_UPDATE=no cargo test"
case_ allow tester Bash "cargo insta accept"
case_ allow tester Bash "INSTA_UPDATE=always cargo test -p lemma-codegen"
case_ deny  debugger Bash "cargo insta accept"
case_ allow coder  Bash "cargo test --workspace 2>&1 | tail -20"
case_ allow coder  Bash "grep -n 'fn parse() -> Ast' crates/lemma-syntax/src/tests.rs 2>/dev/null"
case_ allow coder  Bash "cargo fmt --all"
case_ deny  coder  Bash "echo x > crates/lemma-codegen/src/tests.rs"
case_ deny  coder  Bash "sed -i '' 's/a/b/' crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "cp /tmp/x.rs crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "echo x >> .claude/plans/LEM-1.md"
case_ deny  coder  Bash "echo '[build]' >> .cargo/config.toml"
case_ deny  coder  Bash "ast-grep -p 'x.unwrap()' -r 'x?' -U crates/"
case_ allow coder  Bash "ast-grep -l rust -p '\$X.unwrap()' crates/ | grep -i lemma"

# --- Shell: read-only roles ---------------------------------------------------
case_ deny  reviewer Bash "cargo fmt --all"
case_ allow reviewer Bash "cargo fmt --all --check"
case_ deny  reviewer Bash "cargo clippy --fix --allow-dirty"
case_ allow reviewer Bash "cargo clippy --workspace --all-targets -- -D warnings"
case_ deny  auditor  Bash "cargo add serde"
case_ deny  devsecops Bash "cargo install cargo-audit"
case_ allow devsecops Bash "cargo deny check"
case_ allow devsecops Bash "cargo update --dry-run"
case_ deny  devsecops Bash "cargo update"
case_ allow devsecops Bash "cargo deny check 2>&1 | tee .claude/security/deny.log"
case_ deny  devsecops Bash "echo x > deny.toml"

# --- Prose is data: heredoc bodies and quoted strings are not commands ------------
case_ allow committer Bash $'git commit -F - <<\'EOF\'\nstyle: apply cargo fmt\n\nCo-Authored-By: Claude <noreply@anthropic.com>\nEOF'
case_ allow committer Bash "git commit -m \"ci: install cargo-deny with --locked\""
case_ deny  committer Bash "git commit -m \"style: format\" && cargo fmt --all"
case_ allow ticket-writer Bash $'glab issue create --title "Snapshot review" --description-file - <<\'EOF\'\nRun cargo insta accept only after reading each snapshot.\nEOF'
case_ allow devsecops Bash "grep -n \"cargo install\" .gitlab-ci.yml"
case_ allow reviewer Bash "grep -rn \"cargo add\" crates"
case_ allow planner Bash "grep -rn 'glab issue' .gitlab-ci.yml"
case_ deny  coder Bash "INSTA_UPDATE=\"always\" cargo test"
case_ allow coder Bash "INSTA_UPDATE='no' cargo test"
case_ allow coder Bash "grep -n \"a > b\" crates/lemma-syntax/src/tests.rs"
case_ deny  coder Bash "echo x > \"crates/lemma-syntax/src/tests.rs\""

# --- glab -----------------------------------------------------------------------
case_ deny  coder Bash "glab issue list"
case_ deny  planner Bash "glab issue view 3"
case_ allow ticket-reader Bash "glab issue view 3 --comments"
case_ allow ticket-reader Bash "glab api projects/:id/issues/3/links"
case_ deny  ticket-reader Bash "rm -rf crates"
case_ deny  ticket-reader Write "$R/.claude/tmp/x.md"
case_ allow ticket-writer Write "$R/.claude/tmp/LEM-3.md"
case_ deny  ticket-writer Bash "glab issue close 3"
case_ allow release-engineer Bash "glab ci lint"
case_ deny  devsecops Bash "glab ci lint"

# --- Merge requests -------------------------------------------------------------
case_ deny  mr-reviewer Bash "glab mr merge 3 --yes"
case_ allow mr-reviewer Bash "glab mr merge 3 --squash"
case_ allow mr-reviewer Bash "RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps"
case_ allow mr-reviewer Bash "cargo fmt --all --check"
case_ deny  mr-reviewer Bash "cargo fmt --all"
case_ deny  mr-reviewer Bash "cargo clippy --fix"
case_ allow mr-reviewer Bash "cog check origin/main..HEAD"
case_ deny  mr-reviewer Bash "npm test"
case_ allow mr-reviewer Write "$R/.claude/reviews/mr/3.md"
case_ deny  mr-writer Bash "git push origin LEM-3-x"

# --- Zones of the pipeline, supply-chain, proof, perf and design roles ---------
case_ allow release-engineer Write "$R/clippy.toml"
case_ allow release-engineer Edit  "$R/cog.toml"
case_ allow release-engineer Edit  "$R/.cargo/config.toml"
case_ deny  release-engineer Edit  "$R/crates/lemma-cli/src/main.rs"
case_ allow devsecops Write "$R/.claude/security/supply-chain.md"
case_ deny  devsecops Edit  "$R/deny.toml"
case_ allow proof-engineer Edit  "$R/examples/dot.lm"
case_ allow proof-engineer Write "$R/.claude/proofs/LEM-4.md"
case_ deny  proof-engineer Edit  "$R/crates/lemma-verify/src/lib.rs"
case_ deny  proof-engineer Write "$R/crates/lemma-verify/src/tests.rs"
case_ allow proof-engineer Bash "cargo test -p lemma-verify > .claude/proofs/run.log 2>&1"
case_ deny  proof-engineer Bash "echo x > crates/lemma-verify/src/lib.rs"
case_ allow perf-engineer Write "$R/crates/lemma-codegen/benches/dot.rs"
case_ allow perf-engineer Write "$R/.claude/perf/LEM-5.md"
case_ deny  perf-engineer Edit  "$R/crates/lemma-codegen/src/lib.rs"
case_ deny  perf-engineer Bash "sudo ./target/release/bench"
case_ allow perf-engineer Bash "cargo bench -p lemma-codegen 2>&1 | tee .claude/perf/run.txt"
case_ allow language-designer Write "$R/docs/adr/0003-memory-effects.md"
case_ allow language-designer Edit  "$R/docs/design.md"
case_ deny  language-designer Edit  "$R/README.md"

# --- Fail closed ----------------------------------------------------------------
check deny "malformed event" '{"agent_type": "coder", "tool_name": "Write",'

nojq="$tmp/nojq"
mkdir -p "$nojq"
for dir in /bin /usr/bin; do
  for tool in "$dir"/*; do
    name=${tool##*/}
    [ "$name" = jq ] || [ -e "$nojq/$name" ] || ln -s "$tool" "$nojq/$name"
  done
done
got=$(PATH="$nojq" verdict "$(event coder Write "$R/crates/lemma-codegen/src/lib.rs" "$R")")
if [ "$got" = deny ]; then pass=$((pass + 1)); else failed=$((failed + 1)); echo "FAIL  deny   got $got  jq missing from PATH"; fi

lost="$tmp/no-project"
mkdir -p "$lost"
payload=$(event coder Write "$lost/src/lib.rs" "$lost")
got=$(unset CLAUDE_PROJECT_DIR; verdict "$payload")
if [ "$got" = deny ]; then pass=$((pass + 1)); else failed=$((failed + 1)); echo "FAIL  deny   got $got  no .claude/ anywhere above the cwd"; fi

# --- Anchoring: the Git toplevel wins over a .claude/ in a parent directory ------
nest="$tmp/parent/project"
mkdir -p "$tmp/parent/.claude" "$nest/.claude" "$nest/src"
git -C "$nest" init -q
printf 'special_*\n' > "$nest/.claude/test-paths.txt"
check deny "coder writes a custom test path under a parent .claude/" \
  "$(event coder Write "$nest/src/special_case.rs" "$nest/src")"
check allow "coder writes production code under a parent .claude/" \
  "$(event coder Write "$nest/src/lib.rs" "$nest/src")"

printf '%s passed, %s failed\n' "$pass" "$failed"
[ "$failed" -eq 0 ]
