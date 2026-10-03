//! Printing helpers the w6_* valgen binaries share; each binary includes this
//! file with `#[path]`.

#![allow(dead_code)]

use valgen::{bits, c_string};

/// `CamelCase` to `SCREAMING_SNAKE_CASE`, the way cbindgen names a C enum
/// constant after a Rust variant.
pub fn screaming(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
            if prev.is_ascii_lowercase()
                || prev.is_ascii_digit()
                || (prev.is_ascii_uppercase() && next_lower)
            {
                out.push('_');
            }
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

/// The Debug name of a unit variant, up to any field list.
pub fn variant_name<T: std::fmt::Debug>(value: &T) -> String {
    let text = format!("{value:?}");
    let end = text.find(['{', '(', ' ']).unwrap_or(text.len());
    text[..end].to_string()
}

/// The C enum constant `<prefix>_<VARIANT>` for a core variant whose binding
/// counterpart carries the same name.
pub fn c_enum<T: std::fmt::Debug>(prefix: &str, value: &T) -> String {
    format!("{prefix}_{}", screaming(&variant_name(value)))
}

pub fn def(name: &str, value: impl std::fmt::Display) {
    println!("#define {name} {value}");
}

pub fn def_size(name: &str, value: usize) {
    println!("#define {name} ((size_t){value})");
}

pub fn def_bool(name: &str, value: bool) {
    println!("#define {name} {value}");
}

pub fn def_bits(name: &str, value: f64) {
    println!("static const uint64_t {name} = {};", bits(value));
}

pub fn def_bits_array(name: &str, values: &[f64]) {
    let body: Vec<String> = values.iter().map(|v| bits(*v)).collect();
    println!(
        "static const uint64_t {name}[{}] = {{ {} }};",
        values.len().max(1),
        if body.is_empty() {
            "0".to_string()
        } else {
            body.join(", ")
        }
    );
}

pub fn def_str(name: &str, text: &str) {
    println!("static const char {name}[] = {};", c_string(text));
}

pub fn def_bytes(name: &str, data: &[u8]) {
    println!("static const unsigned char {name}[] = {{");
    for chunk in data.chunks(16) {
        let row: Vec<String> = chunk.iter().map(|b| format!("0x{b:02x}")).collect();
        println!("    {},", row.join(", "));
    }
    if data.is_empty() {
        println!("    0,");
    }
    println!("}};");
    println!("#define {name}_LEN ((size_t){})", data.len());
}

pub fn comment(text: &str) {
    println!("/* {text} */");
}

pub fn blank() {
    println!();
}

/// The text of the adjacent C string literals that follow `marker` in the C
/// source file `path`, up to the first `;` outside a literal, with `\n`,
/// `\t`, `\r`, `\"` and `\\` escapes read.
pub fn c_literal(path: &str, marker: &str) -> String {
    let source = valgen::read(path);
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("{marker} in {path}"))
        + marker.len();
    let mut text = String::new();
    let mut chars = source[start..].chars();
    let mut in_literal = false;
    while let Some(c) = chars.next() {
        match (in_literal, c) {
            (false, ';') => return text,
            (false, '"') => in_literal = true,
            (true, '"') => in_literal = false,
            (true, '\\') => match chars.next().expect("escaped character") {
                'n' => text.push('\n'),
                't' => text.push('\t'),
                'r' => text.push('\r'),
                other => text.push(other),
            },
            (true, c) => text.push(c),
            (false, _) => {}
        }
    }
    panic!("no end of {marker} in {path}");
}

use sidereon_core::ephemeris::{
    ContinuityDefect, MergeContinuityViolation, WindowContinuityDecision, WindowContinuityVerdict,
};

/// bindings/c/src/sp3.rs continuity_defect_json.
pub fn continuity_defect_json(defect: &ContinuityDefect) -> serde_json::Value {
    // `from_j2000_s`, `to_j2000_s`, `magnitude` and `bound` summarize every
    // kind alike; the kind's own fields follow under their core names.
    let satellite = defect.satellite().to_string();
    match defect {
        ContinuityDefect::DuplicateEpoch {
            epoch_j2000_s,
            occurrences,
            ..
        } => serde_json::json!({
            "kind": "duplicate_epoch",
            "satellite": satellite,
            "from_j2000_s": epoch_j2000_s,
            "to_j2000_s": epoch_j2000_s,
            "magnitude": *occurrences as f64,
            "bound": serde_json::Value::Null,
            "epoch_j2000_s": epoch_j2000_s,
            "occurrences": occurrences,
        }),
        ContinuityDefect::SingleSampleSeries { .. } => serde_json::json!({
            "kind": "single_sample_series",
            "satellite": satellite,
            "from_j2000_s": serde_json::Value::Null,
            "to_j2000_s": serde_json::Value::Null,
            "magnitude": serde_json::Value::Null,
            "bound": serde_json::Value::Null,
        }),
        ContinuityDefect::UnusableSample {
            sample_index,
            epoch_j2000_s,
            reason,
            ..
        } => serde_json::json!({
            "kind": "unusable_sample",
            "satellite": satellite,
            "from_j2000_s": epoch_j2000_s,
            "to_j2000_s": epoch_j2000_s,
            "magnitude": serde_json::Value::Null,
            "bound": serde_json::Value::Null,
            "sample_index": sample_index,
            "epoch_j2000_s": epoch_j2000_s,
            "reason": match reason {
                sidereon_core::ephemeris::UnusableSampleReason::EpochNotPlaced => "epoch_not_placed",
                sidereon_core::ephemeris::UnusableSampleReason::NonFinitePosition => "non_finite_position",
                _ => "unknown",
            },
        }),
        ContinuityDefect::SpeedBound {
            from_j2000_s,
            to_j2000_s,
            interval_s,
            displacement_m,
            implied_speed_m_s,
            bound_m_s,
            ..
        } => serde_json::json!({
            "kind": "speed_bound",
            "satellite": satellite,
            "from_j2000_s": from_j2000_s,
            "to_j2000_s": to_j2000_s,
            "magnitude": implied_speed_m_s,
            "bound": bound_m_s,
            "interval_s": interval_s,
            "displacement_m": displacement_m,
            "implied_speed_m_s": implied_speed_m_s,
            "bound_m_s": bound_m_s,
        }),
        ContinuityDefect::HoldOutResidual {
            epoch_j2000_s,
            preceding_j2000_s,
            residual_m,
            tolerance_m,
            node_epochs_j2000_s,
            ..
        } => serde_json::json!({
            "kind": "hold_out_residual",
            "satellite": satellite,
            "from_j2000_s": preceding_j2000_s,
            "to_j2000_s": epoch_j2000_s,
            "magnitude": residual_m,
            "bound": tolerance_m,
            "epoch_j2000_s": epoch_j2000_s,
            "preceding_j2000_s": preceding_j2000_s,
            "residual_m": residual_m,
            "tolerance_m": tolerance_m,
            "node_epochs_j2000_s": node_epochs_j2000_s,
        }),
    }
}

pub fn merge_cell_selection_json(
    selection: &sidereon_core::ephemeris::CellSelection,
) -> serde_json::Value {
    use sidereon_core::ephemeris::{CellSelection, MergeCombine};
    match selection {
        CellSelection::SingleSource { source } => serde_json::json!({
            "kind": "single_source",
            "source": source,
        }),
        CellSelection::Precedence { source, members } => serde_json::json!({
            "kind": "precedence",
            "source": source,
            "members": members,
        }),
        CellSelection::Combined { rule, members } => serde_json::json!({
            "kind": "combined",
            "rule": match rule {
                MergeCombine::Mean => "mean",
                MergeCombine::Median => "median",
                MergeCombine::Precedence => "precedence",
            },
            "members": members,
        }),
    }
}

pub fn merge_continuity_cell_json(
    cell: &sidereon_core::ephemeris::MergeContinuityCell,
) -> serde_json::Value {
    use sidereon_core::ephemeris::MergeContinuityCellRole as Role;
    serde_json::json!({
        "epoch_j2000_s": cell.epoch_j2000_s,
        "role": match cell.role {
            Role::HeldOut => "held_out",
            Role::InterpolationNode => "interpolation_node",
            Role::PairEnd => "pair_end",
            Role::RepeatedEpoch => "repeated_epoch",
        },
        "selection": cell
            .selection
            .as_ref()
            .map_or(serde_json::Value::Null, merge_cell_selection_json),
    })
}

pub fn merge_continuity_violation_json(violation: &MergeContinuityViolation) -> serde_json::Value {
    serde_json::json!({
        "defect": continuity_defect_json(&violation.defect),
        "from_sources": violation.from_sources,
        "to_sources": violation.to_sources,
        "cells": violation
            .cells
            .iter()
            .map(merge_continuity_cell_json)
            .collect::<Vec<_>>(),
        "sources": violation.sources,
        "crosses_contributors": violation.crosses_contributors,
    })
}

/// bindings/c/src/sp3.rs window_continuity_verdict_json.
pub fn window_continuity_verdict_json(verdict: WindowContinuityVerdict<'_>) -> serde_json::Value {
    let decision = match verdict.decision {
        WindowContinuityDecision::Accept => "accept",
        WindowContinuityDecision::Refuse => "refuse",
    };
    serde_json::json!({
        "decision": decision,
        "accepted": verdict.accepted(),
        "influencing_defects": verdict
            .influencing_defects
            .into_iter()
            .map(continuity_defect_json)
            .collect::<Vec<_>>(),
        "influencing_splices": verdict
            .influencing_splices
            .into_iter()
            .map(merge_continuity_violation_json)
            .collect::<Vec<_>>(),
        "all_defects": verdict
            .all_defects
            .iter()
            .map(continuity_defect_json)
            .collect::<Vec<_>>(),
        "all_splices": verdict
            .all_splices
            .into_iter()
            .map(merge_continuity_violation_json)
            .collect::<Vec<_>>(),
    })
}

/// The C status bindings/c/src/sp3.rs map_sp3_argument_error gives a core
/// error.
pub fn sp3_argument_status(err: &sidereon_core::Error) -> &'static str {
    use sidereon_core::Error as E;
    match err {
        E::Parse(_) => "SIDEREON_STATUS_SP3_PARSE",
        E::UnknownSatellite(_) | E::EpochOutOfRange | E::InvalidInput(_) => {
            "SIDEREON_STATUS_INVALID_ARGUMENT"
        }
        E::Ut1OutsideCoverage(_) => "SIDEREON_STATUS_UT1_OUTSIDE_COVERAGE",
        _ => "SIDEREON_STATUS_SOLVE",
    }
}

/// The C status bindings/c/src/sp3.rs map_sp3_interpolation_error gives a
/// core error.
pub fn sp3_interpolation_status(err: &sidereon_core::Error) -> &'static str {
    use sidereon_core::Error as E;
    match err {
        E::Parse(_) => "SIDEREON_STATUS_SP3_PARSE",
        E::UnknownSatellite(_) | E::InvalidInput(_) => "SIDEREON_STATUS_INVALID_ARGUMENT",
        E::EpochOutOfRange => "SIDEREON_STATUS_SOLVE",
        E::Ut1OutsideCoverage(_) => "SIDEREON_STATUS_UT1_OUTSIDE_COVERAGE",
        _ => "SIDEREON_STATUS_SOLVE",
    }
}

/// The numbers of the C array initializer that follows `marker` in the C
/// source file `path` (between the first `{` and the matching `}`), read as a
/// C compiler reads decimal literals.
pub fn c_doubles(path: &str, marker: &str) -> Vec<f64> {
    let source = valgen::read(path);
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("{marker} in {path}"));
    let open = start + source[start..].find('{').expect("initializer");
    let close = open + source[open..].find('}').expect("end of initializer");
    source[open + 1..close]
        .split(',')
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| {
            text.parse::<f64>()
                .unwrap_or_else(|_| panic!("number {text:?} in {marker}"))
        })
        .collect()
}

/// The text between `=` and `;` of the declaration of `name` in the
/// generated C header `path` (a header under bindings/c/tests).
fn header_initializer(path: &str, name: &str) -> String {
    let source = valgen::read(path);
    let mut from = 0;
    loop {
        let at = from
            + source[from..]
                .find(name)
                .unwrap_or_else(|| panic!("{name} in {path}"));
        let after = &source[at + name.len()..];
        let boundary_before = at == 0
            || !source.as_bytes()[at - 1].is_ascii_alphanumeric()
                && source.as_bytes()[at - 1] != b'_';
        let boundary_after = after.starts_with(['[', ' ', '=']);
        if boundary_before && boundary_after && source[..at].rfind('\n').map_or(0, |n| n + 1) <= at
        {
            let line_start = source[..at].rfind('\n').map_or(0, |n| n + 1);
            if source[line_start..at].starts_with("static const") {
                let eq = at + name.len() + after.find('=').expect("initializer") + 1;
                let end = eq + source[eq..].find(';').expect("end of declaration");
                return source[eq..end].to_string();
            }
        }
        from = at + name.len();
    }
}

/// The doubles a generated header states as `UINT64_C(0x...)` bit patterns
/// for `name` (one value for a scalar).
pub fn header_bits(path: &str, name: &str) -> Vec<f64> {
    header_initializer(path, name)
        .split("UINT64_C(")
        .skip(1)
        .map(|chunk| {
            let hex = chunk.split(')').next().expect("bits").trim();
            let hex = hex.trim_start_matches("0x");
            f64::from_bits(u64::from_str_radix(hex, 16).expect("hex bits"))
        })
        .collect()
}

/// The string literals a generated header lists for `name`.
pub fn header_strings(path: &str, name: &str) -> Vec<String> {
    header_initializer(path, name)
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// The text of `#define name "..."` in the generated header `path`.
pub fn header_define_string(path: &str, name: &str) -> String {
    let source = valgen::read(path);
    let prefix = format!("#define {name} ");
    let line = source
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("{name} in {path}"));
    let value = line[prefix.len()..].trim();
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .expect("string define")
        .to_string()
}

/// The number of `#define name <number>` in the generated header `path`, read
/// as a C compiler reads the literal.
pub fn header_define_f64(path: &str, name: &str) -> f64 {
    let source = valgen::read(path);
    let prefix = format!("#define {name} ");
    let line = source
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("{name} in {path}"));
    line[prefix.len()..].trim().parse().expect("numeric define")
}

/// The integers listed for `name` (an int64_t array) in the generated header
/// `path`, with or without an INT64_C wrapper.
pub fn header_int64s(path: &str, name: &str) -> Vec<i64> {
    header_initializer(path, name)
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .split(',')
        .map(|v| {
            v.trim()
                .trim_start_matches("INT64_C(")
                .trim_end_matches(')')
                .trim_end_matches("LL")
                .to_string()
        })
        .filter(|v| !v.is_empty())
        .map(|v| v.parse::<i64>().unwrap_or_else(|_| panic!("int64 {v:?}")))
        .collect()
}

/// Precise-ephemeris samples as they come back to the engine after a C
/// caller copies them out with sidereon_sp3_precise_ephemeris_samples and
/// passes them to a from_samples constructor: the epoch goes out as J2000
/// seconds (precise_sample_to_c) and comes back through
/// instant_from_j2000_seconds (bindings/c/src/sp3.rs, lib.rs).
pub fn samples_through_c(
    samples: &[sidereon_core::ephemeris::PreciseEphemerisSample],
) -> Vec<sidereon_core::ephemeris::PreciseEphemerisSample> {
    use sidereon_core::astro::time::civil::j2000_seconds_from_split;
    use sidereon_core::astro::time::{
        split_julian_date_from_j2000_seconds, Instant, InstantRepr, JulianDateSplit,
    };
    use sidereon_core::constants::SECONDS_PER_DAY;
    samples
        .iter()
        .map(|sample| {
            let j2000_s = match sample.epoch.repr {
                InstantRepr::JulianDate(jd) => j2000_seconds_from_split(jd.jd_whole, jd.fraction),
                InstantRepr::Nanos(_) => f64::NAN,
            };
            let whole_s = j2000_s.floor();
            let (jd_whole, day_fraction) = split_julian_date_from_j2000_seconds(whole_s as i64);
            let fraction = day_fraction + (j2000_s - whole_s) / SECONDS_PER_DAY;
            let split = JulianDateSplit::new(jd_whole, fraction).expect("epoch split");
            let mut out = *sample;
            out.epoch = Instant::from_julian_date(sample.epoch.scale, split);
            out
        })
        .collect()
}

/// The J2000 seconds bindings/c/src/sp3.rs precise_sample_to_c reports for a
/// sample's epoch.
pub fn sample_j2000_s(sample: &sidereon_core::ephemeris::PreciseEphemerisSample) -> f64 {
    use sidereon_core::astro::time::civil::j2000_seconds_from_split;
    use sidereon_core::astro::time::InstantRepr;
    match sample.epoch.repr {
        InstantRepr::JulianDate(jd) => j2000_seconds_from_split(jd.jd_whole, jd.fraction),
        InstantRepr::Nanos(_) => f64::NAN,
    }
}
