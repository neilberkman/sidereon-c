//! Helpers the smoke_b_* valgen binaries share. Each binary includes this
//! file with `#[path = "../smoke_b_shared.rs"] mod shared;`.

#![allow(dead_code)]

use sidereon_core::GnssSystem;

/// `#define NAME value`.
pub fn define(name: &str, value: impl std::fmt::Display) {
    println!("#define {name} {value}");
}

/// A boolean as the C header states it.
pub fn define_bool(name: &str, value: bool) {
    println!("#define {name} {}", if value { "true" } else { "false" });
}

/// An integer with an explicit C type cast, e.g. `((size_t)4)`.
pub fn define_typed(name: &str, c_type: &str, value: impl std::fmt::Display) {
    println!("#define {name} (({c_type}){value})");
}

/// A signed 64-bit integer constant.
pub fn define_i64(name: &str, value: i64) {
    println!("#define {name} INT64_C({value})");
}

/// The exact bit pattern of a double.
pub fn define_bits(name: &str, value: f64) {
    println!("static const uint64_t {name} = {};", valgen::bits(value));
}

/// An array of double bit patterns.
pub fn define_bits_array(name: &str, values: &[f64]) {
    let items: Vec<String> = values.iter().map(|v| valgen::bits(*v)).collect();
    println!(
        "static const uint64_t {name}[{}] = {{ {} }};",
        values.len().max(1),
        if items.is_empty() {
            "0".to_string()
        } else {
            items.join(", ")
        }
    );
}

/// A C string literal holding `text` exactly.
pub fn define_text(name: &str, text: &str) {
    println!("static const char {name}[] = {};", valgen::c_string(text));
}

/// The C enumerator name of a GNSS system.
pub fn gnss_system_c(system: GnssSystem) -> &'static str {
    match system {
        GnssSystem::Gps => "SIDEREON_GNSS_SYSTEM_GPS",
        GnssSystem::Glonass => "SIDEREON_GNSS_SYSTEM_GLONASS",
        GnssSystem::Galileo => "SIDEREON_GNSS_SYSTEM_GALILEO",
        GnssSystem::BeiDou => "SIDEREON_GNSS_SYSTEM_BEI_DOU",
        GnssSystem::Qzss => "SIDEREON_GNSS_SYSTEM_QZSS",
        GnssSystem::Navic => "SIDEREON_GNSS_SYSTEM_NAVIC",
        GnssSystem::Sbas => "SIDEREON_GNSS_SYSTEM_SBAS",
    }
}

/// The name of an enum variant: its Debug text up to the first `(`, `{` or
/// space.
pub fn variant_name(value: &impl std::fmt::Debug) -> String {
    let text = format!("{value:?}");
    let end = text.find(['(', '{', ' ']).unwrap_or(text.len());
    text[..end].to_string()
}

/// UPPER_SNAKE_CASE of a CamelCase variant name, as cbindgen writes C
/// enumerators (`NearestPrior` -> `NEAREST_PRIOR`, `BeiDou` -> `BEI_DOU`).
pub fn upper_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

/// The C enumerator `prefix` + UPPER_SNAKE of the variant name of `value`.
pub fn c_enum(prefix: &str, value: &impl std::fmt::Debug) -> String {
    format!("{prefix}{}", upper_snake(&variant_name(value)))
}

/// The text between `NAME` (as a whole identifier followed by `[` or ` =`)
/// and the `;` that ends its definition in a generated C header.
fn header_definition(header: &str, name: &str) -> String {
    let text = valgen::read(&valgen::tests_path(header));
    let mut from = 0;
    loop {
        let at = text[from..]
            .find(name)
            .map(|i| i + from)
            .unwrap_or_else(|| panic!("{name} in {header}"));
        let before_ok = at == 0
            || !text.as_bytes()[at - 1].is_ascii_alphanumeric() && text.as_bytes()[at - 1] != b'_';
        let after = &text[at + name.len()..];
        if before_ok && (after.starts_with('[') || after.starts_with(" =")) {
            let end = after.find(';').expect("end of definition");
            return after[..end].to_string();
        }
        from = at + name.len();
    }
}

/// The `UINT64_C(0x...)` values of a scalar or array in a generated header.
pub fn header_u64s(header: &str, name: &str) -> Vec<u64> {
    let definition = header_definition(header, name);
    let body = &definition[definition.find('=').expect("initializer") + 1..];
    body.split("UINT64_C(")
        .skip(1)
        .map(|chunk| {
            let token = &chunk[..chunk.find(')').expect("UINT64_C close")];
            let token = token.trim();
            match token.strip_prefix("0x") {
                Some(hex) => u64::from_str_radix(hex, 16).expect("hex u64"),
                None => token.parse().expect("decimal u64"),
            }
        })
        .collect()
}

/// The doubles a generated header states as bit patterns.
pub fn header_f64s(header: &str, name: &str) -> Vec<f64> {
    header_u64s(header, name)
        .into_iter()
        .map(f64::from_bits)
        .collect()
}

/// The string literals of an array (or the one of a scalar) in a generated
/// header.
pub fn header_strings(header: &str, name: &str) -> Vec<String> {
    let definition = header_definition(header, name);
    let body = &definition[definition.find('=').expect("initializer") + 1..];
    body.split('"')
        .skip(1)
        .step_by(2)
        .map(|s| s.to_string())
        .collect()
}

/// The string a `#define NAME "text"` line states in a generated header.
pub fn header_define_string(header: &str, name: &str) -> String {
    let text = valgen::read(&valgen::tests_path(header));
    let key = format!("#define {name} ");
    let line = text
        .lines()
        .find(|line| line.starts_with(&key))
        .unwrap_or_else(|| panic!("{key} in {header}"));
    let rest = &line[key.len()..];
    rest.trim().trim_matches('"').to_string()
}

/// The integer a `#define NAME value` line states in a generated header.
pub fn header_define_int(header: &str, name: &str) -> i64 {
    let text = valgen::read(&valgen::tests_path(header));
    let key = format!("#define {name} ");
    let line = text
        .lines()
        .find(|line| line.starts_with(&key))
        .unwrap_or_else(|| panic!("{key} in {header}"));
    let value = line[key.len()..]
        .trim()
        .trim_start_matches("((size_t)")
        .trim_end_matches(')');
    value.parse().expect("integer define")
}
