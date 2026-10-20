//! The T rendering, per `docs/diagnostics.md` section 6.

use crate::record::{Assignment, Diagnostic};
use crate::records::ordered;
use crate::span::{Source, Span};

/// Renders records in the text layout of `docs/diagnostics.md` section 6, in the order of
/// section 4. The input slices are left as they are.
///
/// A record's snippet is shown when its `span.file` is the name of one of `sources` (the first one
/// of that name) and its end byte does not exceed that source's length.
///
/// # Examples
///
/// ```
/// assert_eq!(lemma_diagnostics::render_text(&[], &[]), "");
/// ```
pub fn render_text(records: &[Diagnostic], sources: &[Source<'_>]) -> String {
    ordered(records)
        .into_iter()
        .map(|(record, _)| render_record(record, sources))
        .collect::<Vec<String>>()
        .join("\n")
}

fn render_record(record: &Diagnostic, sources: &[Source<'_>]) -> String {
    let gutter = " ".repeat(record.span.start.line.to_string().len());
    let source_line = source_line(record, sources);
    let mut lines = vec![
        format!(
            "error[{}]: {}",
            record.kind.name(),
            escape_controls(&record.message)
        ),
        format!(
            "{gutter}--> {}:{}:{}",
            escape_controls(&record.span.file),
            record.span.start.line,
            record.span.start.column
        ),
    ];
    if let Some(text) = &source_line {
        lines.extend(snippet_lines(&gutter, &record.span, text));
    }
    let labelled = labelled_values(record, source_line.is_some());
    lines.extend(labelled.iter().map(|value| format!("{gutter} = {value}")));
    lines
        .iter()
        .map(|line| format!("{}\n", line.trim_end_matches([' ', '\t'])))
        .collect()
}

/// The span's start line as decoded text, without its LF or a final CR, when the snippet can be
/// shown.
fn source_line(record: &Diagnostic, sources: &[Source<'_>]) -> Option<String> {
    let span = &record.span;
    let source = sources.iter().find(|source| source.name == span.file)?;
    if span.end.byte > source.text.len() {
        return None;
    }
    let index = span.start.line.checked_sub(1)?;
    let raw = source.text.split(|&byte| byte == b'\n').nth(index)?;
    let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
    Some(decode_line(raw))
}

/// Decodes a line to one character per column of `docs/diagnostics.md` section 2: a well-formed
/// code point is itself, any other byte is one U+FFFD unless it is a continuation byte, which
/// counts no column and shows as nothing.
fn decode_line(raw: &[u8]) -> String {
    let mut decoded = String::new();
    let mut rest = raw;
    while !rest.is_empty() {
        let Err(error) = std::str::from_utf8(rest) else {
            decoded.push_str(&String::from_utf8_lossy(rest));
            break;
        };
        let (valid, invalid) = rest.split_at(error.valid_up_to());
        decoded.push_str(&String::from_utf8_lossy(valid));
        let Some((&byte, tail)) = invalid.split_first() else {
            break;
        };
        if byte & 0xC0 != 0x80 {
            decoded.push(char::REPLACEMENT_CHARACTER);
        }
        rest = tail;
    }
    decoded
}

fn snippet_lines(gutter: &str, span: &Span, text: &str) -> Vec<String> {
    let shown = text.trim_end_matches([' ', '\t']);
    let source = if shown.is_empty() {
        format!("{} |", span.start.line)
    } else {
        format!("{} | {shown}", span.start.line)
    };
    let pad: String = text
        .chars()
        .chain(std::iter::repeat(' '))
        .take(span.start.column.saturating_sub(1))
        .map(|character| if character == '\t' { '\t' } else { ' ' })
        .collect();
    let carets = "^".repeat(caret_count(span, text));
    vec![
        format!("{gutter} |"),
        source,
        format!("{gutter} | {pad}{carets}"),
    ]
}

fn caret_count(span: &Span, text: &str) -> usize {
    let end_column = if span.end.line > span.start.line {
        text.chars().count() + 1
    } else {
        span.end.column
    };
    end_column.saturating_sub(span.start.column).max(1)
}

fn labelled_values(record: &Diagnostic, snippet_shown: bool) -> Vec<String> {
    let span = &record.span;
    let ends_elsewhere = span.end.line > span.start.line;
    let unshown_and_not_empty = !snippet_shown && span.start.byte != span.end.byte;
    let mut values = Vec::new();
    if ends_elsewhere || unshown_and_not_empty {
        values.push(format!("span ends: {}:{}", span.end.line, span.end.column));
    }
    for related in &record.related {
        values.push(format!(
            "related: {}: {}",
            range(&related.span),
            escape_controls(&related.label)
        ));
    }
    if let Some(obligation) = &record.obligation {
        values.push(format!(
            "obligation: {}",
            escape_controls(&obligation.about)
        ));
        for clause in &obligation.assumes {
            values.push(format!("assumes: {}", escape_controls(clause)));
        }
    }
    if let Some(counterexample) = &record.counterexample {
        values.push(counterexample_value(&counterexample.inputs));
        if let Some(outputs) = &counterexample.outputs {
            values.push(format!("spec: {}", escape_controls(&outputs.spec)));
            values.push(format!(
                "impl: {}",
                escape_controls(&outputs.implementation)
            ));
        }
    }
    for edit in &record.repair {
        values.push(format!(
            "repair: replace {} with {}",
            range(&edit.span),
            quoted(&edit.replacement)
        ));
    }
    values
}

fn counterexample_value(inputs: &[Assignment]) -> String {
    if inputs.is_empty() {
        return "counterexample: (no free variables)".to_string();
    }
    let assignments: Vec<String> = inputs
        .iter()
        .map(|input| {
            format!(
                "{} = {}",
                escape_controls(&input.name),
                escape_controls(&input.value)
            )
        })
        .collect();
    format!("counterexample: {}", assignments.join(", "))
}

fn range(span: &Span) -> String {
    format!(
        "{}:{}:{}-{}:{}",
        escape_controls(&span.file),
        span.start.line,
        span.start.column,
        span.end.line,
        span.end.column
    )
}

fn push_control_escaped(output: &mut String, character: char) {
    match character {
        '\u{8}' => output.push_str("\\b"),
        '\u{c}' => output.push_str("\\f"),
        '\n' => output.push_str("\\n"),
        '\r' => output.push_str("\\r"),
        '\t' => output.push_str("\\t"),
        control if control < ' ' => output.push_str(&format!("\\u{:04x}", u32::from(control))),
        other => output.push(other),
    }
}

/// Escapes the code points below U+0020 as the R rendering does; `"` and `\` stay as they are.
fn escape_controls(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    text.chars()
        .for_each(|character| push_control_escaped(&mut escaped, character));
    escaped
}

/// Writes `text` between double quotes, escaped exactly as an R string.
fn quoted(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 2);
    escaped.push('"');
    for character in text.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            other => push_control_escaped(&mut escaped, other),
        }
    }
    escaped.push('"');
    escaped
}
