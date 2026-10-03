//! Printing helpers the smoke_a_* binaries share. This directory holds no
//! main.rs, so cargo builds no binary from it; each binary includes it with
//! `#[path]`.
#![allow(dead_code)]

use valgen::{bits, c_string};

/// `#define NAME true|false`.
pub fn def_bool(name: &str, value: bool) {
    println!("#define {name} {}", if value { "true" } else { "false" });
}

/// `#define NAME ((size_t)value)`.
pub fn def_usize(name: &str, value: usize) {
    println!("#define {name} ((size_t){value})");
}

/// `#define NAME value` for a signed integer.
pub fn def_i64(name: &str, value: i64) {
    println!("#define {name} ({value})");
}

/// `static const uint64_t NAME = UINT64_C(...)`, a double's bits.
pub fn def_bits(name: &str, value: f64) {
    println!("static const uint64_t {name} = {};", bits(value));
}

/// `static const uint64_t NAME[n] = {...}`, each double's bits.
pub fn def_bits_array(name: &str, values: &[f64]) {
    let body: Vec<String> = values.iter().map(|value| bits(*value)).collect();
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

/// `static const char NAME[] = "..."`.
pub fn def_str(name: &str, text: &str) {
    println!("static const char {name}[] = {};", c_string(text));
}

/// `#define NAME "Debug name"`, the Debug text of an engine value up to its
/// first `{`, `(` or space: the variant name.
pub fn variant_name<T: std::fmt::Debug>(value: &T) -> String {
    let text = format!("{value:?}");
    match text.find(['{', '(', ' ']) {
        Some(end) => text[..end].to_string(),
        None => text,
    }
}

/// `#define NAME "..."` holding a variant name.
pub fn def_variant<T: std::fmt::Debug>(name: &str, value: &T) {
    println!("#define {name} {}", c_string(&variant_name(value)));
}

/// A comment line in the generated header.
pub fn comment(text: &str) {
    println!("/* {text} */");
}
