//! The parser of `docs/grammar.md` section 3 on well-formed input, observed through the range-free
//! `dump` of the AST it builds: precedence, associativity, parentheses, each primary form, the
//! signatures of section 4, clauses, statements and files of several items.

use lemma_diagnostics::{Kind, Source};
use lemma_syntax::{C1Variant, TextRange, dump, parse};

type Outcome = (String, Vec<Kind>);
type Case = (String, String);
type Mismatch = (String, Outcome, String);

const PRECEDENCE_CASES: [(&str, &str); 12] = [
    ("a * b + c", "(+ (* a b) c)"),
    ("a + b * c", "(+ a (* b c))"),
    ("a % b - c / d", "(- (% a b) (/ c d))"),
    ("a + b < c - d", "(< (+ a b) (- c d))"),
    ("a + b >= c", "(>= (+ a b) c)"),
    ("a == b && c != d", "(&& (== a b) (!= c d))"),
    ("a <= b || c > d", "(|| (<= a b) (> c d))"),
    ("a && b || c && d", "(|| (&& a b) (&& c d))"),
    ("a || b && c", "(|| a (&& b c))"),
    ("-a * b", "(* (neg a) b)"),
    ("a * -b", "(* a (neg b))"),
    ("!c && d", "(&& (not c) d)"),
];

const ASSOCIATIVITY_CASES: [(&str, &str); 7] = [
    ("a - b - c", "(- (- a b) c)"),
    ("a - b + c", "(+ (- a b) c)"),
    ("a / b * c", "(* (/ a b) c)"),
    ("a % b % c", "(% (% a b) c)"),
    ("a && b && c", "(&& (&& a b) c)"),
    ("a || b || c", "(|| (|| a b) c)"),
    ("- -a", "(neg (neg a))"),
];

const PARENTHESES_CASES: [(&str, &str); 7] = [
    (
        "(c0 + c1) + (c2 + c3)",
        "(+ (paren (+ c0 c1)) (paren (+ c2 c3)))",
    ),
    ("c0 + c1 + (c2 + c3)", "(+ (+ c0 c1) (paren (+ c2 c3)))"),
    ("a * (b + c)", "(* a (paren (+ b c)))"),
    ("(a * b) + c", "(+ (paren (* a b)) c)"),
    ("((a))", "(paren (paren a))"),
    ("-(x)", "(neg (paren x))"),
    ("!(a == b)", "(not (paren (== a b)))"),
];

const PRIMARY_CASES: [(&str, &str); 20] = [
    ("0", "0"),
    ("42", "42"),
    ("0.5", "0.5"),
    ("1.5e3", "1.5e3"),
    ("true", "true"),
    ("false", "false"),
    ("n", "n"),
    ("a[i]", "(index a i)"),
    ("a[i + 1]", "(index a (+ i 1))"),
    ("fma(x, y, z)", "(call fma x y z)"),
    ("f()", "(call f)"),
    ("f32x4.splat(0.0)", "(member f32x4 splat 0.0)"),
    ("f32x4.load(a, i)", "(member f32x4 load a i)"),
    ("u8x16.zero()", "(member u8x16 zero)"),
    ("!a[i]", "(not (index a i))"),
    ("!f(x) || c", "(|| (not (call f x)) c)"),
    (
        "sum(i: u64 in 0..<n) { a[i] }",
        "(sum i u64 0 n (index a i))",
    ),
    (
        "sum(j: u32 in n - 4..<n * 2) { j } + 1",
        "(+ (sum j u32 (- n 4) (* n 2) j) 1)",
    ),
    (
        "sum(i: u64 in 0..<n) { sum(j: u64 in 0..<i) { a[j] } }",
        "(sum i u64 0 n (sum j u64 0 i (index a j)))",
    ),
    (
        "forall(i: u64 in 0..<n) { a[i] != 0 }",
        "(forall i u64 0 n (!= (index a i) 0))",
    ),
];

const SIGNATURE_CASES: [(&str, &str); 7] = [
    (
        "export fn pick(n: u64, k: u64 < n, a: &f32[n] @a4 ro) -> f32",
        concat!(
            "export fn pick\n",
            "  param n u64\n",
            "  param k (bounded u64 < n)\n",
            "  param a (ref f32 n @a4 ro)\n",
            "  result f32\n",
        ),
    ),
    (
        "export fn find(n: u64, a: &u8[n] @a1 ro) -> u64 <= n",
        concat!(
            "export fn find\n",
            "  param n u64\n",
            "  param a (ref u8 n @a1 ro)\n",
            "  result (bounded u64 <= n)\n",
        ),
    ),
    (
        "export fn gather(n: u64, m: u64, a: &f32[n] @a4 ro, idx: &(u64 < n)[m] @a8 ro) -> f32",
        concat!(
            "export fn gather\n",
            "  param n u64\n",
            "  param m u64\n",
            "  param a (ref f32 n @a4 ro)\n",
            "  param idx (ref (bounded u64 < n) m @a8 ro)\n",
            "  result f32\n",
        ),
    ),
    (
        "export fn fill(n: u64, out: &f32[n] @a16 wo) -> u64",
        concat!(
            "export fn fill\n",
            "  param n u64\n",
            "  param out (ref f32 n @a16 wo)\n",
            "  result u64\n",
        ),
    ),
    (
        "fn zero() -> f32x4",
        concat!("fn zero\n", "  result f32x4\n"),
    ),
    (
        "fn blend(flag: bool, v: f32x4, k: u8 <= 15) -> bool",
        concat!(
            "fn blend\n",
            "  param flag bool\n",
            "  param v f32x4\n",
            "  param k (bounded u8 <= 15)\n",
            "  result bool\n",
        ),
    ),
    (
        "fn first(t: &(u8 <= 15)[16] @a1 ro, w: &i32[16] @a64 ro) -> u8",
        concat!(
            "fn first\n",
            "  param t (ref (bounded u8 <= 15) 16 @a1 ro)\n",
            "  param w (ref i32 16 @a64 ro)\n",
            "  result u8\n",
        ),
    ),
];

const PERMISSION_CASES: [(&str, &str); 5] = [
    ("allow()", "equiv"),
    ("allow(fp_reassoc)", "equiv fp_reassoc"),
    ("allow(fp_contract)", "equiv fp_contract"),
    (
        "allow(fp_reassoc, fp_contract)",
        "equiv fp_reassoc fp_contract",
    ),
    (
        "allow(fp_contract, fp_reassoc)",
        "equiv fp_reassoc fp_contract",
    ),
];

const STATEMENT_CASES: [(&str, &str); 5] = [
    ("    return n\n", "    return n\n"),
    (
        concat!(
            "    let x: u64 = n + 1\n",
            "    let flag: bool = x > n\n",
            "    let v: f32x4 = f32x4.splat(1.5)\n",
            "    return x\n",
        ),
        concat!(
            "    let x u64 (+ n 1)\n",
            "    let flag bool (> x n)\n",
            "    let v f32x4 (member f32x4 splat 1.5)\n",
            "    return x\n",
        ),
    ),
    (
        concat!(
            "    loop i: u64 in 0..<n step 1 carry(s: u64 = 0) {\n",
            "      next s = s + a[i]\n",
            "    }\n",
            "    return s\n",
        ),
        concat!(
            "    loop i u64 0 n step 1\n",
            "      carry s u64 0\n",
            "      next s (+ s (index a i))\n",
            "    return s\n",
        ),
    ),
    (
        concat!(
            "    loop i: u64 in n - 8..<n + 8 step 4 carry(lo: u64 = 0, hi: u64 = 1) {\n",
            "      next hi = lo\n",
            "      next lo = hi\n",
            "    }\n",
            "    return lo + hi\n",
        ),
        concat!(
            "    loop i u64 (- n 8) (+ n 8) step 4\n",
            "      carry lo u64 0\n",
            "      carry hi u64 1\n",
            "      next hi lo\n",
            "      next lo hi\n",
            "    return (+ lo hi)\n",
        ),
    ),
    (
        concat!(
            "    let half: u64 = n / 2\n",
            "    loop i: u64 in 0..<half step 1 carry(s: u64 = 0) {\n",
            "      let x: u64 = a[i]\n",
            "      loop j: u64 in 0..<4 step 2 carry(t: u64 = x) {\n",
            "        next t = t + j\n",
            "      }\n",
            "      next s = s + t\n",
            "    }\n",
            "    return s\n",
        ),
        concat!(
            "    let half u64 (/ n 2)\n",
            "    loop i u64 0 half step 1\n",
            "      carry s u64 0\n",
            "      let x u64 (index a i)\n",
            "      loop j u64 0 4 step 2\n",
            "        carry t u64 x\n",
            "        next t (+ t j)\n",
            "      next s (+ s t)\n",
            "    return s\n",
        ),
    ),
];

const FILE_CASES: [(&str, &str); 2] = [
    ("", ""),
    (
        concat!(
            "export fn first() -> u64\n",
            "  eff pure\n",
            "  spec 1\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return 1\n",
            "  }\n",
            "\n",
            "fn second() -> u64\n",
            "  eff pure\n",
            "  spec 2\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return 2\n",
            "  }\n",
        ),
        concat!(
            "export fn first\n",
            "  result u64\n",
            "  eff pure\n",
            "  spec 1\n",
            "  equiv\n",
            "  impl\n",
            "    return 1\n",
            "fn second\n",
            "  result u64\n",
            "  eff pure\n",
            "  spec 2\n",
            "  equiv\n",
            "  impl\n",
            "    return 2\n",
        ),
    ),
];

const CLAUSES_FUNCTION: &str = concat!(
    "fn clamp(n: u64, a: &u64[n] @a8 ro) -> u64\n",
    "  eff pure\n",
    "  req n > 0\n",
    "  req n % 4 == 0\n",
    "  ens result >= a[0]\n",
    "  ens result != 7\n",
    "  spec a[0]\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return a[0]\n",
    "  }\n",
);

const CLAUSES_DUMP: &str = concat!(
    "fn clamp\n",
    "  param n u64\n",
    "  param a (ref u64 n @a8 ro)\n",
    "  result u64\n",
    "  eff pure\n",
    "  req (> n 0)\n",
    "  req (== (% n 4) 0)\n",
    "  ens (>= result (index a 0))\n",
    "  ens (!= result 7)\n",
    "  spec (index a 0)\n",
    "  equiv\n",
    "  impl\n",
    "    return (index a 0)\n",
);

const COMMENTED_FUNCTION: &str = concat!(
    "// lead\n",
    "fn f() -> u64\n",
    "  // clause\n",
    "  eff pure\n",
    "  spec 0\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return 0\n",
    "  }\n",
);

const PLAIN_CLAUSES: &str = concat!(
    "  eff pure\n",
    "  spec 0\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return 0\n",
    "  }\n",
);

const PLAIN_CLAUSES_DUMP: &str = concat!(
    "  eff pure\n",
    "  spec 0\n",
    "  equiv\n",
    "  impl\n",
    "    return 0\n",
);

fn outcome(text: &str) -> Outcome {
    let source = Source {
        name: "parse.lm",
        text: text.as_bytes(),
    };
    let parsed = parse(source, C1Variant::Ascii);
    let kinds = parsed
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.kind)
        .collect();
    (dump(&parsed.file), kinds)
}

/// The cases whose text does not parse without diagnostics to the expected dump.
fn mismatches(cases: Vec<Case>) -> Vec<Mismatch> {
    cases
        .into_iter()
        .map(|(text, expected)| {
            let actual = outcome(&text);
            (text, actual, expected)
        })
        .filter(|(_, actual, expected)| *actual != (expected.clone(), Vec::new()))
        .collect()
}

fn spec_cases(table: &[(&str, &str)]) -> Vec<Case> {
    table
        .iter()
        .map(|(expression, expected)| {
            let text = format!(
                "fn f() -> u64\n  eff pure\n  spec {expression}\n  equiv allow()\n  impl {{\n    return 0\n  }}\n"
            );
            let dump = format!(
                "fn f\n  result u64\n  eff pure\n  spec {expected}\n  equiv\n  impl\n    return 0\n"
            );
            (text, dump)
        })
        .collect()
}

fn signature_cases(table: &[(&str, &str)]) -> Vec<Case> {
    table
        .iter()
        .map(|(signature, expected)| {
            (
                format!("{signature}\n{PLAIN_CLAUSES}"),
                format!("{expected}{PLAIN_CLAUSES_DUMP}"),
            )
        })
        .collect()
}

fn permission_cases(table: &[(&str, &str)]) -> Vec<Case> {
    table
        .iter()
        .map(|(allow, expected)| {
            let text = format!(
                "fn f() -> u64\n  eff pure\n  spec 0\n  equiv {allow}\n  impl {{\n    return 0\n  }}\n"
            );
            let dump = format!(
                "fn f\n  result u64\n  eff pure\n  spec 0\n  {expected}\n  impl\n    return 0\n"
            );
            (text, dump)
        })
        .collect()
}

fn statement_cases(table: &[(&str, &str)]) -> Vec<Case> {
    table
        .iter()
        .map(|(body, expected)| {
            let text = format!(
                "fn f(n: u64, a: &u64[n] @a8 ro) -> u64\n  eff pure\n  spec 0\n  equiv allow()\n  impl {{\n{body}  }}\n"
            );
            let dump = format!(
                "fn f\n  param n u64\n  param a (ref u64 n @a8 ro)\n  result u64\n  eff pure\n  spec 0\n  equiv\n  impl\n{expected}"
            );
            (text, dump)
        })
        .collect()
}

fn file_cases(table: &[(&str, &str)]) -> Vec<Case> {
    table
        .iter()
        .map(|(text, expected)| (text.to_string(), expected.to_string()))
        .collect()
}

#[test]
fn operators_bind_by_the_precedence_table() {
    let cases = spec_cases(&PRECEDENCE_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn binary_operators_of_one_level_associate_to_the_left() {
    let cases = spec_cases(&ASSOCIATIVITY_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn parentheses_are_kept_as_written() {
    let cases = spec_cases(&PARENTHESES_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn each_primary_form_parses_to_its_node() {
    let cases = spec_cases(&PRIMARY_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn signatures_parse_to_their_parameters_and_result() {
    let cases = signature_cases(&SIGNATURE_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn permissions_in_any_order_dump_in_table_order() {
    let cases = permission_cases(&PERMISSION_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn statements_nest_in_their_blocks() {
    let cases = statement_cases(&STATEMENT_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn a_file_dumps_each_of_its_items_in_order() {
    let cases = file_cases(&FILE_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn req_and_ens_clauses_keep_their_source_order() {
    let text = CLAUSES_FUNCTION;

    let outcome = outcome(text);

    assert_eq!(outcome, (CLAUSES_DUMP.to_string(), Vec::new()));
}

#[test]
fn parse_collects_every_comment_with_its_range() {
    let source = Source {
        name: "parse.lm",
        text: COMMENTED_FUNCTION.as_bytes(),
    };

    let comments: Vec<(TextRange, String)> = parse(source, C1Variant::Ascii)
        .comments
        .into_iter()
        .map(|comment| (comment.range, comment.text))
        .collect();

    assert_eq!(
        comments,
        vec![
            (TextRange { start: 0, end: 7 }, "// lead".to_string()),
            (TextRange { start: 24, end: 33 }, "// clause".to_string()),
        ]
    );
}
