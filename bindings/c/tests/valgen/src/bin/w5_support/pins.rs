//! Printing helpers the w5_* valgen binaries share. Every pin is a C macro or
//! static constant named with the binary's upper-case prefix.

#![allow(dead_code)]

use valgen::{bits, c_string};

/// Collects the lines of one generated header.
pub struct Pins {
    prefix: String,
}

impl Pins {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }

    fn name(&self, name: &str) -> String {
        format!("{}_{}", self.prefix, name)
    }

    /// A comment line block.
    pub fn comment(&self, text: &str) {
        println!();
        let mut lines = text.lines();
        if let Some(first) = lines.next() {
            println!("/* {first}");
        }
        for line in lines {
            println!(" * {line}");
        }
        println!(" */");
    }

    /// A double as its exact bit pattern.
    pub fn f64(&self, name: &str, value: f64) {
        println!(
            "static const uint64_t {} = {};",
            self.name(name),
            bits(value)
        );
    }

    /// Doubles as exact bit patterns.
    pub fn f64s(&self, name: &str, values: &[f64]) {
        let body: Vec<String> = values.iter().map(|v| bits(*v)).collect();
        println!(
            "static const uint64_t {}[{}] = {{ {} }};",
            self.name(name),
            values.len().max(1),
            if body.is_empty() {
                "0".to_string()
            } else {
                body.join(", ")
            }
        );
        println!(
            "#define {}_COUNT ((size_t){})",
            self.name(name),
            values.len()
        );
    }

    /// An integer (count, index, field value).
    pub fn int(&self, name: &str, value: i128) {
        println!("#define {} ({})", self.name(name), value);
    }

    /// Integers as a static array.
    pub fn ints(&self, name: &str, c_type: &str, values: &[i128]) {
        let body: Vec<String> = values.iter().map(|v| v.to_string()).collect();
        println!(
            "static const {c_type} {}[{}] = {{ {} }};",
            self.name(name),
            values.len().max(1),
            if body.is_empty() {
                "0".to_string()
            } else {
                body.join(", ")
            }
        );
        println!(
            "#define {}_COUNT ((size_t){})",
            self.name(name),
            values.len()
        );
    }

    /// A boolean.
    pub fn bool(&self, name: &str, value: bool) {
        println!(
            "#define {} ({})",
            self.name(name),
            if value { "true" } else { "false" }
        );
    }

    /// A text as a C string literal plus its byte length.
    pub fn str(&self, name: &str, value: &str) {
        println!(
            "static const char {}[] = {};",
            self.name(name),
            c_string(value)
        );
        println!("#define {}_LEN ((size_t){})", self.name(name), value.len());
    }

    /// The C enum constant that names an engine enum value: `c_prefix` plus
    /// the variant's Debug name in screaming snake case (the naming the
    /// generated header follows for the binding's mirror enums).
    pub fn variant<T: std::fmt::Debug>(&self, name: &str, c_prefix: &str, value: &T) {
        println!(
            "#define {} {}{}",
            self.name(name),
            c_prefix,
            screaming_snake(&variant_name(value))
        );
    }

    /// The C enum constant for a variant given by name.
    pub fn variant_named(&self, name: &str, c_prefix: &str, variant: &str) {
        println!(
            "#define {} {}{}",
            self.name(name),
            c_prefix,
            screaming_snake(variant)
        );
    }

    /// An engine outcome: true when the call succeeded, and the error's
    /// variant name otherwise (as a string).
    pub fn outcome<T, E: std::fmt::Debug>(&self, name: &str, result: &Result<T, E>) {
        self.bool(&format!("{name}_OK"), result.is_ok());
        let text = match result {
            Ok(_) => String::new(),
            Err(err) => variant_name(err),
        };
        self.str(&format!("{name}_ERROR_VARIANT"), &text);
    }
}

/// The Debug text of `value` up to its first payload delimiter.
pub fn variant_name<T: std::fmt::Debug>(value: &T) -> String {
    let text = format!("{value:?}");
    match text.find(['{', '(', ' ']) {
        Some(end) => text[..end].to_string(),
        None => text,
    }
}

/// `CamelCase` to `CAMEL_CASE`, with digit runs attached to the preceding
/// word (`Wgs84Oblate` -> `WGS84_OBLATE`).
pub fn screaming_snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).map_or(false, |n| n.is_ascii_lowercase());
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
