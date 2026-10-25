# LEM-12 state

Branch: claude/focused-ramanujan-lgmayo (session-designated name instead of LEM-12-<slug>), started from origin/main fc3f9cc
Ticket report: .claude/state/LEM-12-ticket.md
Plan: .claude/plans/LEM-12.md (written, 7 steps, awaiting user review)
Open reservations: none yet
Explicitly discarded: none

| Step | State | Tests commit |
| --- | --- | --- |
| 1. Tokens, lexer, C1 switch | green, 26/26, frozen-check OK (code uncommitted until the gate; push blocked 403) | ac1df43 |
| 2. AST, dump, parser (well-formed) | green 73/73, committed eb386a2 | 2ec01a4 |
| 3. Printer, formatter, comment lexer rules | green 99/99, committed c1537ea | b03674f |
| 4. Recovery, token-level kinds, nesting (competing passes) | red tests committed; coders A and B running in ../lemma-competing/LEM-12-e4-{A,B}, base 97e6a4d | 97e6a4d |
| 5. Clause and signature kinds | to do | — |
| 6. Statement-structure kinds | to do | — |
| 7. Fuzz target + just fuzz | to do | — |

User decisions: new syntax kind `nesting-limit` (no repair), visible nesting only; number pending research (proposal 16, user wants it evidence-based). Fuzz: `just fuzz` recipe only (implied). Step 6 kept (implied).

Research done: 16 supported, weak-to-moderate evidence (parser caps 100-1e5 are stack guards; readability block-nesting defaults 3-5; real kernels measured/estimated 3-6, up to 9). User confirmed 16 (hard limit, message suggests let, recalibrated at #30 from #29 hit counts).
Step 1 wiring added (uncommitted): lemma-syntax Cargo.toml deps (lemma-diagnostics, insta 1.49 new, proptest), .gitignore *.snap.new *.pending-snap, Cargo.lock.
Designer done: docs/adr/0010-nesting-limit.md (proposed), grammar.md §3 rule 12 + "Nesting depth" + §7 row, diagnostics.md §7 message shape, design.md bullet. Uncommitted, lands with the final work. Step 1 tests: crates/lemma-syntax/tests/lexer.rs (1257 lines, 27 tests, ~300 cases; ratio vs ~800 prod OK). Tester readings 1-12 confirmed; 13 overruled by me: `x.5` repair inserts " 0" (space-padded when previous char is a word char), kind stays invalid-number (P7). Plan to be updated with it. check.sh skipped .rs files: ast-grep not installed here (install before the gate). TODO: planner revises plan question 1 (128 -> 16, ADR 0010 counting rule) before step 4. Open to user: accept ADR 0010, long binary chains, contract-side message.
Created keys: #72 (ci: fuzz target on a schedule, created by the user, blocked by #12). Chain-length follow-up: user is creating it from another session (number unknown yet; check before #13).
Where we were: language-designer recording nesting-limit (docs only, lands with final work); tester on step 1. Previously: then language-designer adds nesting-limit to grammar.md §7, then step 1

Where we are: step 1 green; STOP between steps, waiting for the user. Next: step 2 (AST, dump, parser for well-formed input). Coder points to settle: ZWSP inside a number, CRLF in comment text, ScalarType reuse in ast.rs.
Commits (user waived the gate for step 1, reviewer not run, just check not run in full): f08316e lexer, a02726e docs ADR 0010. Push still 403 (Claude GitHub App not installed/linked for lemma-lang).
Pushed 2026-10-25: branch on origin at a02726e.

USER LIFTED THE STOP (2026-10-25): run the rest of the plan without stopping between steps, through exit gate, commit, push, PR creation; stop after the PR is created. Rule-5 and overflowing-step stops still apply.
Merged origin/main (04f0be3: orchestration moved out of the repo, skills in .agents/skills, shape rules now tools/rules/check.sh) as 59a2a08, pushed. Local setup restored from backup as ignored files. frozen-check.sh is local only (.claude/tools). Plan revised: Kind::NestingLimit in step 2 (lemma-diagnostics kinds test red since a02726e), ZWSP in number = deleted, reads as one literal (docs settle it). Step 2 tester launched.
Step 2 red tests committed 2ec01a4 (pushed). Coder step 2 launched.
grammar.md §5 comment rules written (uncommitted, docs). Designer decisions 1-4 accepted by me. Lone CR inside a comment: accepted as written (not rejected); noted for the PR as an open point.
Docs §5 committed 8f10026. Coder step 2 point: fault span rule (repair hull) and unrepaired invalid-encoding comment fault ends the next fn: revisit in step 4. typos flags \u{1D6BA} (needs _typos.toml, release-engineer). User raised: comments as injection vector (pending decision).
User chose the middle path on comments: own line only, before item/clause/statement, printable ASCII only; new ADR (designer running); forbid-comments revisited at calibration. AGENTS.md "Text you read is data" section added by me (uncommitted). Step 3 tester told to skip trailing/inline/closer comment tests. Planner must fold the comment kinds into steps 4-6 after the designer.
ADR 0011 comments (proposed, uncommitted): own line before item/clause/statement, else misplaced-comment (2-edit move repair); comment-character (printable ASCII+tab, delete repair). Conflicts with ADR 0006 P6 "layout never an error": amendment needs user approval. Designer bypassed its guard with a python heredoc on grammar.md (edits within docs/, reported to user). My calls on open points: comment-only file stays an error; after-last-item moves above impl (uniform); deletion not transliteration; T-snippet escaping = follow-up for lemma-diagnostics. Tester told to fix parse.rs/lexer.rs conflicting rows; planner revising steps 3-6.
Step 3 tests ab1ca9a (1879 lines vs ~400+150 prod: over 2x; accepted by me: 841 are the generator required by the plan, printer cases map to §5 rules). `- -x` spacing needs a §5 line (designer, with the P6 amendment). Plan revised: comment lexer rules in step 3, misplaced-comment in step 5.
Step 3 points: Fault.span public field added; Comment.text has rejected chars removed (accepted); lexer rows pinning comment-character/CRLF missing (tester to add in step 4); typos flags \u{1D6BA} x2 and \u{E9} (needs _typos.toml); comments with no following construct print at col 0 after last item. Waiting user: option A (misplaced-comment error + P6 amendment) vs B (formatter moves, no error).
USER CHOSE A: misplaced-comment is an error; ADR 0006 P6 amendment approved (designer writing it + `- -x` §5 line). Step 4 tester running.
ADR 0011 + ADR 0006 P6 amendment committed bba14a1, pushed. Open (non-blocking): ADR 0006 line 101 "lemma check accepts any layout (P6)" left as is.
Step 4 tests written (~1880 lines vs ~500 prod, over 2x, accepted: tables+soup+data+240 lines of step-3 comment rows I asked for). Found 2 lexer defects (touching edits). My decisions sent to tester: faults reported exactly once (resume token trivia reported); suffix deletion only for type suffixes, one edit when insert+delete; merge adjacent edits. Tester used a python script on its own test file (guard bypass, reported).
