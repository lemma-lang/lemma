//! The printer and the formatter of `docs/grammar.md` section 5: the canonical layout of every
//! construct, comments on their own line, both forms of `dot` printed back byte for byte, and the
//! formatter refusing any text with a syntax diagnostic.

use lemma_diagnostics::{Diagnostic, Kind, Source};
use lemma_syntax::{C1Variant, format, parse, print};

type DiagnosticView = (Kind, usize, usize);
type Formatted = Result<String, Vec<DiagnosticView>>;
type Case = (String, String);
type Mismatch = (String, Formatted, String);

const ASCII: C1Variant = C1Variant::Ascii;
const UNICODE: C1Variant = C1Variant::Unicode;

const DOT_ASCII_SPELLINGS: &str = include_str!("../../../examples/dot.lm");
const DOT_UNICODE_SPELLINGS: &str = include_str!("testdata/dot_unicode.lm");

const SMALL_FUNCTION: &str = concat!(
    "fn f(n: u64) -> u64\n",
    "  eff pure\n",
    "  spec n\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return n\n",
    "  }\n",
);

const OTHER_FUNCTION: &str = concat!(
    "fn g() -> bool\n",
    "  eff pure\n",
    "  spec true\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return false\n",
    "  }\n",
);

const LOOP_FUNCTION: &str = concat!(
    "fn f(n: u64) -> u64\n",
    "  eff pure\n",
    "  spec n\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    loop i: u64 in 0..<n step 1 carry(s: u64 = 0) {\n",
    "      next s = s + i\n",
    "    }\n",
    "    return s\n",
    "  }\n",
);

const LINE_CASES: [(&str, &str); 4] = [
    (
        "fn f(n: u64) -> u64 eff pure spec n equiv allow() impl { return n }\n",
        SMALL_FUNCTION,
    ),
    (
        concat!(
            "    export fn f(n: u64) -> u64\n",
            "eff pure\n",
            "        req n > 0\n",
            " ens result >= n\n",
            "spec n\n",
            "equiv allow()\n",
            "            impl {\n",
            "let m: u64 = n\n",
            "                return m\n",
            "}\n",
        ),
        concat!(
            "export fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  req n > 0\n",
            "  ens result >= n\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    let m: u64 = n\n",
            "    return m\n",
            "  }\n",
        ),
    ),
    (
        concat!(
            "fn f(n: u64) -> u64 eff pure spec n equiv allow() impl { ",
            "loop i: u64 in 0..<n step 1 carry(s: u64 = 0) { ",
            "loop j: u64 in 0..<i step 1 carry(t: u64 = s) { next t = t + j } ",
            "next s = t } return s }\n",
        ),
        concat!(
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    loop i: u64 in 0..<n step 1 carry(s: u64 = 0) {\n",
            "      loop j: u64 in 0..<i step 1 carry(t: u64 = s) {\n",
            "        next t = t + j\n",
            "      }\n",
            "      next s = t\n",
            "    }\n",
            "    return s\n",
            "  }\n",
        ),
    ),
    (
        concat!(
            "fn f(n: u64) -> u64   \n",
            "  eff pure\t\n",
            "  spec n \n",
            "  equiv allow()  \n",
            "  impl {\t \n",
            "    return n   \n",
            "  }  \n",
        ),
        SMALL_FUNCTION,
    ),
];

const ITEMLESS_CASES: [(&str, &str); 3] = [("", ""), ("\n", ""), (" \t\r\n\n  ", "")];

const SUM_BODY_CASES: [(&str, &str); 4] = [
    (
        "sum(i: u64 in 0..<n){a[i]*b[i]}",
        "sum(i: u64 in 0..<n) { a[i] * b[i] }",
    ),
    (
        "forall(i: u64 in 0..<n) {\n    a[i] != 0\n  }",
        "forall(i: u64 in 0..<n) { a[i] != 0 }",
    ),
    (
        "sum(i: u64 in 0..<n) {sum(j: u64 in 0..<i) {a[j]}}",
        "sum(i: u64 in 0..<n) { sum(j: u64 in 0..<i) { a[j] } }",
    ),
    (
        "sum(i: u64 in 0..<n) {   a[i]   } + 1",
        "sum(i: u64 in 0..<n) { a[i] } + 1",
    ),
];

const CARRY_CASES: [(&str, &str); 4] = [
    (
        concat!(
            "    loop i: u64 in 0..<n step 1 carry(\n",
            "      s: u64 = 0\n",
            "    ) {\n",
            "      next s = s + a[i]\n",
            "    }\n",
            "    return s\n",
        ),
        concat!(
            "    loop i: u64 in 0..<n step 1 carry(s: u64 = 0) {\n",
            "      next s = s + a[i]\n",
            "    }\n",
            "    return s\n",
        ),
    ),
    (
        concat!(
            "    loop i: u64 in 0..<n step 1 carry(lo: u64 = 0, hi: u64 = 1) {\n",
            "      next lo = hi\n",
            "      next hi = lo\n",
            "    }\n",
            "    return lo\n",
        ),
        concat!(
            "    loop i: u64 in 0..<n step 1 carry(\n",
            "      lo: u64 = 0,\n",
            "      hi: u64 = 1\n",
            "    ) {\n",
            "      next lo = hi\n",
            "      next hi = lo\n",
            "    }\n",
            "    return lo\n",
        ),
    ),
    (
        concat!(
            "    loop i: u64 in 0..<n step 1 carry(x: u64 = 0,\n",
            "                                     y: u64 = 1, z: u64 = 2\n",
            "  )\n",
            "  {\n",
            "      next z = x\n",
            "      next y = z\n",
            "      next x = y\n",
            "    }\n",
            "    return x\n",
        ),
        concat!(
            "    loop i: u64 in 0..<n step 1 carry(\n",
            "      x: u64 = 0,\n",
            "      y: u64 = 1,\n",
            "      z: u64 = 2\n",
            "    ) {\n",
            "      next z = x\n",
            "      next y = z\n",
            "      next x = y\n",
            "    }\n",
            "    return x\n",
        ),
    ),
    (
        concat!(
            "    loop i: u64 in 0..<n step 1 carry(s: u64 = 0) {\n",
            "      loop j: u64 in 0..<4 step 1 carry(p: u64 = s, q: u64 = 0) {\n",
            "        next p = q\n",
            "        next q = p\n",
            "      }\n",
            "      next s = p\n",
            "    }\n",
            "    return s\n",
        ),
        concat!(
            "    loop i: u64 in 0..<n step 1 carry(s: u64 = 0) {\n",
            "      loop j: u64 in 0..<4 step 1 carry(\n",
            "        p: u64 = s,\n",
            "        q: u64 = 0\n",
            "      ) {\n",
            "        next p = q\n",
            "        next q = p\n",
            "      }\n",
            "      next s = p\n",
            "    }\n",
            "    return s\n",
        ),
    ),
];

const EXPRESSION_SPACE_CASES: [(&str, &str); 9] = [
    ("a [ i ]  *  b[ i ]", "a[i] * b[i]"),
    ("fma ( a[0] , b[0] , n )", "fma(a[0], b[0], n)"),
    ("f32x4 . splat ( 0.0 )", "f32x4.splat(0.0)"),
    ("f32x4.load( a,n )", "f32x4.load(a, n)"),
    ("n%16==0&&n!=0||n<=1", "n % 16 == 0 && n != 0 || n <= 1"),
    ("n-1+n/2", "n - 1 + n / 2"),
    ("- n * n", "-n * n"),
    ("! c || d", "!c || d"),
    (
        "sum ( i : u64 in 0 ..< n ) { a [ i ] }",
        "sum(i: u64 in 0..<n) { a[i] }",
    ),
];

const PARENTHESES_CASES: [(&str, &str); 6] = [
    ("( c0+c1 )+( c2+c3 )", "(c0 + c1) + (c2 + c3)"),
    ("c0 + c1 + (c2 + c3)", "c0 + c1 + (c2 + c3)"),
    ("a[0] * (b[0] + n)", "a[0] * (b[0] + n)"),
    ("( ( n ) )", "((n))"),
    ("-( n )", "-(n)"),
    ("!( n == 0 )", "!(n == 0)"),
];

const SIGNATURE_CASES: [(&str, &str); 5] = [
    (
        "fn  f ( n : u64 , a : & f32 [ n ] @a4 ro , k : u64 < n ) -> u64 <= n",
        "fn f(n: u64, a: &f32[n] @a4 ro, k: u64 < n) -> u64 <= n",
    ),
    (
        "fn g(m:u64,n:u64,t:&(u8<m)[n]@a1 ro)->u8",
        "fn g(m: u64, n: u64, t: &(u8 < m)[n] @a1 ro) -> u8",
    ),
    (
        "export fn h(\n  n: u64,\n  m: u64\n)\n  -> bool",
        "export fn h(n: u64, m: u64) -> bool",
    ),
    ("fn z()->f32x4", "fn z() -> f32x4"),
    (
        "export\nfn w(o: &f32[16] @a16 wo) -> u64",
        "export fn w(o: &f32[16] @a16 wo) -> u64",
    ),
];

const CLAUSE_AND_STATEMENT_SPACES: (&str, &str) = (
    concat!(
        "fn f(n: u64,a:&u64[n]@a8 ro)->u64\n",
        "  eff   pure\n",
        "  req n>0\n",
        "  ens result>=n\n",
        "  spec n\n",
        "  equiv allow ( fp_reassoc , fp_contract )\n",
        "  impl{\n",
        "    let m:u64=n+1\n",
        "    loop i:u64 in 0..<m step 2 carry(s:u64=0){\n",
        "      next s=s+a[i]\n",
        "    }\n",
        "    return s\n",
        "  }\n",
    ),
    concat!(
        "fn f(n: u64, a: &u64[n] @a8 ro) -> u64\n",
        "  eff pure\n",
        "  req n > 0\n",
        "  ens result >= n\n",
        "  spec n\n",
        "  equiv allow(fp_reassoc, fp_contract)\n",
        "  impl {\n",
        "    let m: u64 = n + 1\n",
        "    loop i: u64 in 0..<m step 2 carry(s: u64 = 0) {\n",
        "      next s = s + a[i]\n",
        "    }\n",
        "    return s\n",
        "  }\n",
    ),
);

const PERMISSION_ORDER: (&str, &str) = (
    concat!(
        "fn f(n: u64) -> u64\n",
        "  eff pure\n",
        "  spec n\n",
        "  equiv allow(fp_contract, fp_reassoc)\n",
        "  impl {\n",
        "    return n\n",
        "  }\n",
    ),
    concat!(
        "fn f(n: u64) -> u64\n",
        "  eff pure\n",
        "  spec n\n",
        "  equiv allow(fp_reassoc, fp_contract)\n",
        "  impl {\n",
        "    return n\n",
        "  }\n",
    ),
);

const OWN_LINE_COMMENT_CASES: [(&str, &str); 6] = [
    (
        concat!(
            "   // lead\n",
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return n\n",
            "  }\n",
        ),
        concat!(
            "// lead\n",
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return n\n",
            "  }\n",
        ),
    ),
    (
        concat!(
            "fn f(n: u64) -> u64\n",
            "// effects\n",
            "  eff pure\n",
            "  spec n\n",
            "        // exact\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return n\n",
            "  }\n",
        ),
        concat!(
            "fn f(n: u64) -> u64\n",
            "  // effects\n",
            "  eff pure\n",
            "  spec n\n",
            "  // exact\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return n\n",
            "  }\n",
        ),
    ),
    (
        concat!(
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "// the result\n",
            "    return n\n",
            "  }\n",
        ),
        concat!(
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    // the result\n",
            "    return n\n",
            "  }\n",
        ),
    ),
    (
        concat!(
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "  // the loop\n",
            "    loop i: u64 in 0..<n step 1 carry(s: u64 = 0) {\n",
            "// add\n",
            "      next s = s + i\n",
            "    }\n",
            "    return s\n",
            "  }\n",
        ),
        concat!(
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    // the loop\n",
            "    loop i: u64 in 0..<n step 1 carry(s: u64 = 0) {\n",
            "      // add\n",
            "      next s = s + i\n",
            "    }\n",
            "    return s\n",
            "  }\n",
        ),
    ),
    (
        concat!(
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return n\n",
            "  }\n",
            "// second\n",
            "// still second\n",
            "fn g() -> bool\n",
            "  eff pure\n",
            "  spec true\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return false\n",
            "  }\n",
        ),
        concat!(
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return n\n",
            "  }\n",
            "\n",
            "// second\n",
            "// still second\n",
            "fn g() -> bool\n",
            "  eff pure\n",
            "  spec true\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return false\n",
            "  }\n",
        ),
    ),
    (
        concat!(
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  // one\n",
            "    // two\n",
            "// three\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return n\n",
            "  }\n",
        ),
        concat!(
            "fn f(n: u64) -> u64\n",
            "  eff pure\n",
            "  // one\n",
            "  // two\n",
            "  // three\n",
            "  spec n\n",
            "  equiv allow()\n",
            "  impl {\n",
            "    return n\n",
            "  }\n",
        ),
    ),
];

const COMMENT_TEXT_CASES: [(&str, &str); 7] = [
    ("/// three slashes\n", "/// three slashes\n"),
    ("//no space\n", "//no space\n"),
    ("//   inner  spaces kept\n", "//   inner  spaces kept\n"),
    ("//\tinner\ttabs kept\n", "//\tinner\ttabs kept\n"),
    ("// trailing spaces   \n", "// trailing spaces\n"),
    ("// trailing tabs\t \t\n", "// trailing tabs\n"),
    ("// carriage return\r\n", "// carriage return\n"),
];

const MISPLACED_COMMENT: &str = concat!(
    "fn f(n: u64) -> u64\n",
    "  eff pure\n",
    "  spec n // why\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return n\n",
    "  }\n",
);

const COMMENT_CHARACTER: &str = concat!(
    "// caf\u{E9}\n",
    "fn f(n: u64) -> u64\n",
    "  eff pure\n",
    "  spec n\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return n\n",
    "  }\n",
);

const ONE_LINE_DOT: &str = concat!(
    "// Dot product of two f32 arrays, for 128-bit NEON with four accumulators.\n",
    "export fn dot(n: u64, a: &f32[n] @a128 ro, b: &f32[n] @a128 ro) -> f32 eff pure ",
    "req n % 16 == 0 spec sum(i: u64 in 0..<n) { a[i] * b[i] } ",
    "equiv allow(fp_reassoc, fp_contract) impl { ",
    "loop i: u64 in 0..<n step 16 carry(c0: f32x4 = f32x4.splat(0.0), ",
    "c1: f32x4 = f32x4.splat(0.0), c2: f32x4 = f32x4.splat(0.0), ",
    "c3: f32x4 = f32x4.splat(0.0)) { ",
    "next c0 = fma(f32x4.load(a, i), f32x4.load(b, i), c0) ",
    "next c1 = fma(f32x4.load(a, i + 4), f32x4.load(b, i + 4), c1) ",
    "next c2 = fma(f32x4.load(a, i + 8), f32x4.load(b, i + 8), c2) ",
    "next c3 = fma(f32x4.load(a, i + 12), f32x4.load(b, i + 12), c3) } ",
    "return reduce_sum((c0 + c1) + (c2 + c3)) }",
);

const SPACED_DOT: &str = concat!(
    "\n\n",
    "   // Dot product of two f32 arrays, for 128-bit NEON with four accumulators.\n",
    "\n",
    "export   fn   dot (n:u64 ,a : &f32 [n]   @a128   ro,b:&f32[n]@a128 ro)->f32\n",
    "\teff\tpure\n",
    "      req   n%16==0\n",
    "\n",
    " spec sum( i:u64 in 0 ..<n ){a[i]*b[i]}\n",
    "  equiv   allow( fp_contract ,fp_reassoc )\n",
    "impl{\n",
    "loop i :u64 in 0..< n step 16 carry(\n",
    "c0:f32x4=f32x4.splat(0.0),c1: f32x4 = f32x4 . splat( 0.0 ),\n",
    "        c2:f32x4=f32x4.splat(0.0),\n",
    "\n\n",
    "  c3:f32x4=f32x4.splat(0.0))\n",
    "{\n",
    "next c0=fma(f32x4.load(a,i),f32x4.load(b,i),c0)\n",
    "            next c1 = fma( f32x4.load( a , i+4 ) , f32x4.load(b, i +4), c1 )\n",
    "next c2=fma(f32x4.load(a,i+8),f32x4.load(b,i+8),c2) ",
    "next c3=fma(f32x4.load(a,i+12),f32x4.load(b,i+12),c3)\n",
    "}\n",
    "            return reduce_sum( ( c0+c1 )+( c2+c3 ) )\n",
    "}\n",
    "\n\n\n",
);

const UNREPAIRED_FAULT: &str = concat!(
    "fn f(n: u64) -> u64\n",
    "  eff pure\n",
    "  spec $\n",
    "  equiv allow()\n",
    "  impl {\n",
    "    return n\n",
    "  }\n",
);

fn source(text: &str) -> Source<'_> {
    Source {
        name: "printer.lm",
        text: text.as_bytes(),
    }
}

fn diagnostic_view(diagnostic: &Diagnostic) -> DiagnosticView {
    (
        diagnostic.kind,
        diagnostic.span.start.byte,
        diagnostic.span.end.byte,
    )
}

fn formatted(text: &str, variant: C1Variant) -> Formatted {
    format(source(text), variant)
        .map_err(|diagnostics| diagnostics.iter().map(diagnostic_view).collect())
}

/// The cases whose text does not format under variant A to the expected text.
fn mismatches(cases: Vec<Case>) -> Vec<Mismatch> {
    cases
        .into_iter()
        .map(|(text, expected)| {
            let actual = formatted(&text, ASCII);
            (text, actual, expected)
        })
        .filter(|(_, actual, expected)| *actual != Ok(expected.clone()))
        .collect()
}

fn file_cases(table: &[(&str, &str)]) -> Vec<Case> {
    table
        .iter()
        .map(|(text, expected)| (text.to_string(), expected.to_string()))
        .collect()
}

fn spec_cases(table: &[(&str, &str)]) -> Vec<Case> {
    let function = |expression: &str| {
        std::format!(
            "fn f(n: u64, a: &u64[n] @a8 ro, b: &u64[n] @a8 ro) -> u64\n  eff pure\n  spec {expression}\n  equiv allow()\n  impl {{\n    return 0\n  }}\n"
        )
    };
    table
        .iter()
        .map(|(written, canonical)| (function(written), function(canonical)))
        .collect()
}

fn statement_cases(table: &[(&str, &str)]) -> Vec<Case> {
    let function = |body: &str| {
        std::format!(
            "fn f(n: u64, a: &u64[n] @a8 ro) -> u64\n  eff pure\n  spec 0\n  equiv allow()\n  impl {{\n{body}  }}\n"
        )
    };
    table
        .iter()
        .map(|(written, canonical)| (function(written), function(canonical)))
        .collect()
}

fn signature_cases(table: &[(&str, &str)]) -> Vec<Case> {
    let function = |signature: &str| {
        std::format!(
            "{signature}\n  eff pure\n  spec 0\n  equiv allow()\n  impl {{\n    return 0\n  }}\n"
        )
    };
    table
        .iter()
        .map(|(written, canonical)| (function(written), function(canonical)))
        .collect()
}

fn comment_text_cases(table: &[(&str, &str)]) -> Vec<Case> {
    table
        .iter()
        .map(|(written, canonical)| {
            (
                std::format!("{written}{SMALL_FUNCTION}"),
                std::format!("{canonical}{SMALL_FUNCTION}"),
            )
        })
        .collect()
}

fn empty_line_cases() -> Vec<Case> {
    let two_items = std::format!("{SMALL_FUNCTION}\n{OTHER_FUNCTION}");
    let loop_without_final_line_feed = LOOP_FUNCTION.trim_end_matches('\n');
    vec![
        (
            std::format!("\n\n\n{SMALL_FUNCTION}"),
            SMALL_FUNCTION.to_string(),
        ),
        (
            loop_without_final_line_feed.to_string(),
            LOOP_FUNCTION.to_string(),
        ),
        (
            std::format!("{SMALL_FUNCTION}\n\n\n"),
            SMALL_FUNCTION.to_string(),
        ),
        (
            concat!(
                "fn f(n: u64) -> u64\n",
                "\n",
                "  eff pure\n",
                "\n\n",
                "  spec n\n",
                "  equiv allow()\n",
                "\n",
                "  impl {\n",
                "\n",
                "    loop i: u64 in 0..<n step 1 carry(s: u64 = 0) {\n",
                "\n",
                "      next s = s + i\n",
                "\n",
                "    }\n",
                "\n",
                "    return s\n",
                "\n",
                "  }\n",
            )
            .to_string(),
            LOOP_FUNCTION.to_string(),
        ),
        (
            std::format!("{SMALL_FUNCTION}{OTHER_FUNCTION}"),
            two_items.clone(),
        ),
        (
            std::format!("{SMALL_FUNCTION}\n\n\n\n{OTHER_FUNCTION}"),
            two_items,
        ),
    ]
}

fn without_first_line(text: &str) -> &str {
    text.split_once('\n').map_or(text, |(_, rest)| rest)
}

#[test]
fn each_construct_starts_its_own_line_at_its_indentation() {
    let cases = file_cases(&LINE_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn one_empty_line_separates_items_and_the_output_ends_with_one_line_feed() {
    let cases = empty_line_cases();

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn a_text_without_item_or_comment_formats_to_the_empty_text() {
    let cases = file_cases(&ITEMLESS_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn sum_and_forall_bodies_stay_on_their_line_between_spaced_braces() {
    let cases = spec_cases(&SUM_BODY_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn one_carry_stays_on_the_loop_line_and_several_take_a_line_each() {
    let cases = statement_cases(&CARRY_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn expression_tokens_are_spaced_as_the_canonical_form_says() {
    let cases = spec_cases(&EXPRESSION_SPACE_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn a_negation_of_a_negation_keeps_its_two_minuses_apart() {
    let cases = spec_cases(&[("- - n", "- -n")]);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn a_signature_prints_on_one_line_spaced_as_the_canonical_form_says() {
    let cases = signature_cases(&SIGNATURE_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn clause_and_statement_tokens_are_spaced_as_the_canonical_form_says() {
    let (written, canonical) = CLAUSE_AND_STATEMENT_SPACES;

    let formatted = formatted(written, ASCII);

    assert_eq!(formatted, Ok(canonical.to_string()));
}

#[test]
fn parentheses_are_neither_added_nor_removed() {
    let cases = spec_cases(&PARENTHESES_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn permissions_print_in_the_order_of_the_permission_table() {
    let (written, canonical) = PERMISSION_ORDER;

    let formatted = formatted(written, ASCII);

    assert_eq!(formatted, Ok(canonical.to_string()));
}

#[test]
fn a_comment_on_its_own_line_is_printed_before_its_construct_at_its_indentation() {
    let cases = file_cases(&OWN_LINE_COMMENT_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn comment_text_is_kept_as_written_minus_its_trailing_spaces_and_tabs() {
    let cases = comment_text_cases(&COMMENT_TEXT_CASES);

    let mismatches = mismatches(cases);

    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

#[test]
fn dot_in_variant_a_formats_to_itself_byte_for_byte() {
    let text = DOT_ASCII_SPELLINGS;

    let formatted = formatted(text, ASCII);

    assert_eq!(formatted, Ok(DOT_ASCII_SPELLINGS.to_string()));
}

#[test]
fn dot_in_variant_u_formats_to_itself_byte_for_byte() {
    let text = DOT_UNICODE_SPELLINGS;

    let formatted = formatted(text, UNICODE);

    assert_eq!(formatted, Ok(DOT_UNICODE_SPELLINGS.to_string()));
}

#[test]
fn scrambled_layouts_of_dot_format_to_examples_dot() {
    let crlf_dot = DOT_ASCII_SPELLINGS.replace('\n', "\r\n");
    let scrambled = [ONE_LINE_DOT, SPACED_DOT, crlf_dot.as_str()];
    let dot: Formatted = Ok(DOT_ASCII_SPELLINGS.to_string());

    let formatted = scrambled.map(|text| formatted(text, ASCII));

    assert_eq!(formatted, [dot.clone(), dot.clone(), dot]);
}

#[test]
fn print_writes_the_c1_spellings_of_the_requested_variant_without_comments() {
    let file = parse(source(DOT_ASCII_SPELLINGS), ASCII).file;

    let printed = [ASCII, UNICODE].map(|variant| print(&file, variant));

    assert_eq!(
        printed,
        [
            without_first_line(DOT_ASCII_SPELLINGS),
            without_first_line(DOT_UNICODE_SPELLINGS),
        ]
    );
}

#[test]
fn format_returns_the_diagnostics_of_a_text_whose_faults_have_repairs() {
    let text = DOT_UNICODE_SPELLINGS;

    let formatted = formatted(text, ASCII);

    assert_eq!(
        formatted,
        Err(vec![
            (Kind::C1OtherVariant, 182, 184),
            (Kind::C1OtherVariant, 192, 195),
            (Kind::C1OtherVariant, 283, 286),
        ])
    );
}

#[test]
fn format_returns_the_diagnostic_of_a_text_with_an_unrepaired_fault() {
    let text = UNREPAIRED_FAULT;

    let formatted = formatted(text, ASCII);

    assert_eq!(formatted, Err(vec![(Kind::InvalidCharacter, 38, 39)]));
}

#[test]
fn format_returns_the_diagnostic_of_a_comment_that_trails_code() {
    let text = MISPLACED_COMMENT;

    let formatted = formatted(text, ASCII);

    assert_eq!(formatted, Err(vec![(Kind::MisplacedComment, 40, 46)]));
}

#[test]
fn format_returns_the_diagnostic_of_a_comment_with_a_non_ascii_character() {
    let text = COMMENT_CHARACTER;

    let formatted = formatted(text, ASCII);

    assert_eq!(formatted, Err(vec![(Kind::CommentCharacter, 6, 8)]));
}
