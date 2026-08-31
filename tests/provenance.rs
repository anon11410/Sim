//! CORE-11 clause (a) at the artefact level: every value in the shipped config
//! states where it came from, and says honestly whether it has been checked.
//!
//! These checks read `config/baseline.toml` as **raw text**. The TOML parser
//! discards comments, so a parsed `Params` cannot see an annotation at all — a
//! test built on the parser would pass over a config with no provenance
//! whatsoever. The scanner below is a handful of lines and pulls in no
//! regular-expression dependency: it walks the file, tracks the current table
//! header, and for each key assignment inspects the line above it.
//!
//! **A note for a future reader on `attributed_rows_are_still_marked_unverified`
//! (test 6).** That test fails the moment a provenance row attributed to the
//! published literature is marked verified. This is deliberate and it is not an
//! obstacle to route around: it forces an upgrade and its evidence to land in
//! one reviewable commit. It is expected to be amended by **Phase 6** — the
//! phase carrying the blocking verification gate per `01-CONTEXT.md` D-19 — and
//! by no earlier phase. Amending it means editing it in the same commit as the
//! rows it releases, with the recorded outcome written into
//! `config/PROVENANCE.md` per the procedure in that file's section 3.

use std::fs;

const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/baseline.toml");
const PROVENANCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/PROVENANCE.md");

/// The grade vocabulary defined at `.planning/research/SUMMARY.md:169`. Reused
/// here verbatim, never extended: A = the model authors' own code, B = an
/// annotated replication citing the paper's table and equation numbers,
/// C = derived arithmetic, PROJECT = a choice with no published precedent.
const VOCABULARY: [&str; 4] = ["A", "B", "C", "PROJECT"];

/// The prefix that marks the machine-checkable half of an annotation block.
const MARKER: &str = "# GRADE: ";

/// One leaf key of the shipped config, located in the raw text.
struct Key {
    /// Full dotted path, e.g. `firm.dividend_buffer_ppm`.
    path: String,
    /// Zero-based index of the assignment line.
    line: usize,
}

fn shipped() -> String {
    fs::read_to_string(CONFIG).expect("the shipped config must be readable")
}

fn provenance() -> String {
    fs::read_to_string(PROVENANCE).expect("config/PROVENANCE.md must be readable")
}

/// True when `line` is a key assignment rather than a comment, a table header
/// or blank space.
fn is_assignment(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('#')
        && !trimmed.starts_with('[')
        && trimmed.contains('=')
}

/// Every leaf key in `raw`, in file order, each with its dotted path.
fn keys(raw: &str) -> Vec<Key> {
    let mut table = String::new();
    let mut found = Vec::new();

    for (line, text) in raw.lines().enumerate() {
        let trimmed = text.trim();
        if trimmed.starts_with('[') {
            table = trimmed.trim_matches(['[', ']']).to_string();
        } else if is_assignment(text) {
            let name = trimmed
                .split('=')
                .next()
                .expect("split always yields one element")
                .trim();
            found.push(Key {
                path: format!("{table}.{name}"),
                line,
            });
        }
    }

    found
}

/// The nearest non-blank line strictly above `line`, if there is one.
fn preceding_non_blank<'a>(lines: &[&'a str], line: usize) -> Option<&'a str> {
    lines[..line]
        .iter()
        .rev()
        .find(|candidate| !candidate.trim().is_empty())
        .copied()
}

/// Every annotation line in `raw`, with its zero-based line index.
fn annotations(raw: &str) -> Vec<(usize, &str)> {
    raw.lines()
        .enumerate()
        .filter(|(_, text)| text.starts_with(MARKER))
        .collect()
}

/// The three fields of an annotation line, in order, or `Err` describing what
/// is wrong with it.
fn fields(annotation: &str) -> Result<(String, String, String), String> {
    let parts: Vec<&str> = annotation.split(" | ").collect();
    if parts.len() != 3 {
        return Err(format!(
            "expected three ` | `-separated fields, found {}",
            parts.len()
        ));
    }

    let grade = parts[0]
        .strip_prefix(MARKER)
        .ok_or_else(|| format!("does not begin with `{MARKER}`"))?
        .trim()
        .to_string();
    let source = parts[1]
        .strip_prefix("SOURCE: ")
        .ok_or_else(|| "the second field is not `SOURCE: …`".to_string())?
        .trim()
        .to_string();
    let cadence = parts[2]
        .strip_prefix("CADENCE: ")
        .ok_or_else(|| "the third field is not `CADENCE: …`".to_string())?
        .trim()
        .to_string();

    Ok((grade, source, cadence))
}

/// The cells of a provenance table row keyed by a dotted config path, or
/// `None` when `line` is not such a row.
///
/// Section 4's constants table is deliberately excluded: its rows are keyed by
/// a Rust `const` name, which carries no dot, and those constants have no
/// config key by design.
fn provenance_row(line: &str) -> Option<Vec<String>> {
    if !line.starts_with("| `") {
        return None;
    }
    let cells: Vec<String> = line
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect();
    if cells.len() != 6 {
        return None;
    }
    let key = cells[0].trim_matches('`');
    if !key.contains('.') {
        return None;
    }
    Some(cells)
}

// --- 1. Every key states its grade ----------------------------------------

#[test]
fn every_key_has_a_source_grade() {
    let raw = shipped();
    let lines: Vec<&str> = raw.lines().collect();

    let mut unannotated = Vec::new();
    for key in keys(&raw) {
        let above = preceding_non_blank(&lines, key.line);
        if !above.is_some_and(|text| text.starts_with(MARKER)) {
            unannotated.push(format!(
                "{} (line {}): the line above is {:?}",
                key.path,
                key.line + 1,
                above.unwrap_or("<start of file>")
            ));
        }
    }

    assert!(
        unannotated.is_empty(),
        "these config keys carry no `{MARKER}…` annotation:\n  {}",
        unannotated.join("\n  ")
    );
}

// --- 2. No invented grade letters -----------------------------------------

#[test]
fn every_grade_letter_is_in_the_vocabulary() {
    let raw = shipped();

    let mut outside = Vec::new();
    for (line, annotation) in annotations(&raw) {
        match fields(annotation) {
            Ok((grade, _, _)) if VOCABULARY.contains(&grade.as_str()) => {}
            Ok((grade, _, _)) => outside.push(format!(
                "line {}: grade {grade:?} is not one of {VOCABULARY:?}",
                line + 1
            )),
            Err(why) => outside.push(format!("line {}: {why}", line + 1)),
        }
    }

    assert!(
        outside.is_empty(),
        "the grade vocabulary is defined at `.planning/research/SUMMARY.md:169` \
         and is not extended here:\n  {}",
        outside.join("\n  ")
    );
}

// --- 3. All three fields, in order, each non-empty -------------------------

#[test]
fn every_annotation_has_a_source_and_a_cadence() {
    let raw = shipped();

    let mut malformed = Vec::new();
    for (line, annotation) in annotations(&raw) {
        match fields(annotation) {
            Ok((grade, source, cadence)) => {
                if grade.is_empty() || source.is_empty() || cadence.is_empty() {
                    malformed.push(format!(
                        "line {}: an empty field in {annotation:?}",
                        line + 1
                    ));
                }
            }
            Err(why) => malformed.push(format!("line {}: {why} — {annotation:?}", line + 1)),
        }
    }

    assert!(
        malformed.is_empty(),
        "every annotation must read `{MARKER}… | SOURCE: … | CADENCE: …`:\n  {}",
        malformed.join("\n  ")
    );
}

// --- 4. No annotation has drifted from its key ----------------------------

#[test]
fn no_annotation_is_orphaned() {
    let raw = shipped();
    let lines: Vec<&str> = raw.lines().collect();

    let mut drifted = Vec::new();
    for (line, annotation) in annotations(&raw) {
        let below = lines.get(line + 1).copied();
        if !below.is_some_and(is_assignment) {
            drifted.push(format!(
                "line {}: {annotation:?} is followed by {:?}, not by a key assignment",
                line + 1,
                below.unwrap_or("<end of file>")
            ));
        }
    }

    assert!(
        drifted.is_empty(),
        "an annotation separated from its key reads as covering the key below it, \
         which is worse than a missing one:\n  {}",
        drifted.join("\n  ")
    );
}

// --- 5. The config and the provenance record stay in step -----------------

#[test]
fn every_config_key_has_a_provenance_row() {
    let raw = shipped();
    let record = provenance();

    let mut missing = Vec::new();
    for key in keys(&raw) {
        if !record.contains(&format!("`{}`", key.path)) {
            missing.push(key.path);
        }
    }

    assert!(
        missing.is_empty(),
        "these config keys have no row in config/PROVENANCE.md:\n  {}",
        missing.join("\n  ")
    );
}

// --- 6. No silent verification upgrade ------------------------------------

#[test]
fn attributed_rows_are_still_marked_unverified() {
    let record = provenance();

    let mut rows = 0usize;
    let mut upgraded = Vec::new();
    for line in record.lines() {
        let Some(cells) = provenance_row(line) else {
            continue;
        };
        // Grade B is, by the vocabulary's own definition, a value taken from an
        // annotated replication rather than read from the published paper —
        // which is exactly the condition `UNVERIFIED` records.
        if cells[2] != "B" {
            continue;
        }
        rows += 1;
        if !cells[5].contains("UNVERIFIED") {
            upgraded.push(format!(
                "{}: verification state is {:?}",
                cells[0].trim_matches('`'),
                cells[5]
            ));
        }
    }

    assert!(
        rows > 0,
        "no grade-B rows were found in config/PROVENANCE.md — the scanner or the \
         table shape has drifted, and this check is silently passing on nothing"
    );
    assert!(
        upgraded.is_empty(),
        "a row attributed to the published literature was marked verified. \
         Per config/PROVENANCE.md section 3 step 4 this may only happen in the \
         Phase 6 commit that also records the primary-source outcome, and this \
         test is amended in that same commit:\n  {}",
        upgraded.join("\n  ")
    );
}
