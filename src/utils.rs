// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::io::Write;

use serde::Serialize;

static GIGA: u64 = 1000000000;

pub trait CsvFriend {
    fn header(&self) -> String;
    fn to_csv(&self) -> String;
}

pub fn to_seconds(ns: u64) -> f64 {
    ns as f64 / GIGA as f64
}

pub fn to_nanoseconds(s: f64) -> u64 {
    (s * GIGA as f64).round() as u64
}

/// Compute the fidelity with an exponential decaying rate.
///
/// Input values are not checked for consistency.
///
/// Parameters:
/// - `f_init`: initial fidelity.
/// - `decay_rate`: the decaying rate, in inverse time units.
/// - `time`: time after which the fidelity is computed.
///
pub fn fidelity(f_init: f64, decay_rate: f64, time: f64) -> f64 {
    0.25 + (f_init - 0.25) * (-decay_rate * time).exp()
}

pub fn open_output_file(
    path: &str,
    filename: &str,
    append: bool,
    header: &str,
) -> anyhow::Result<std::fs::File> {
    let full_path = format!("{}{}", path, filename);

    if let Some(parent_path) = std::path::Path::new(&full_path).parent() {
        if parent_path.exists() {
            if !parent_path.is_dir() {
                anyhow::bail!(
                    "parent exists but is not a directory: {}",
                    parent_path.to_string_lossy()
                );
            }
        } else {
            std::fs::create_dir_all(parent_path)?;
        }
    }

    let add_header = !append
        || match std::fs::metadata(&full_path) {
            Ok(metadata) => metadata.len() == 0,
            Err(_) => true,
        };
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .append(append)
        .create(true)
        .truncate(!append)
        .open(full_path)?;
    if add_header {
        writeln!(&mut f, "{}", header)?;
    }
    Ok(f)
}

pub fn struct_to_csv<T: Serialize>(s: T) -> anyhow::Result<String> {
    let fields = struct_to_map(s)?;
    let mut ret = vec![];
    for (_name, value) in fields {
        ret.push(format!("{}", value));
    }
    Ok(ret.join(","))
}

pub fn struct_to_csv_header<T: Serialize>(s: T) -> anyhow::Result<String> {
    let fields = struct_to_map(s)?;
    let mut ret = vec![];
    for (name, _value) in fields {
        ret.push(format!("{}", name));
    }
    Ok(ret.join(","))
}

fn struct_to_map<T: Serialize>(s: T) -> anyhow::Result<serde_json::Map<String, serde_json::Value>> {
    let mut value = serde_json::to_value(s)?;
    anyhow::ensure!(value.is_object(), "invalid struct");
    let fields = value.as_object_mut().unwrap();
    fields.sort_keys();
    Ok(fields.clone())
}

#[cfg(test)]
mod tests {
    use crate::utils::fidelity;

    use super::{to_nanoseconds, to_seconds};

    #[test]
    fn test_to_from_nanosecs() {
        assert_eq!(42.0, to_seconds(to_nanoseconds(42.0)));
    }

    #[test]
    fn test_fidelity() {
        assert_float_eq::assert_f64_near!(0.9, fidelity(0.9, 0.1, 0.0));
        assert_float_eq::assert_f64_near!(0.4891216367614375, fidelity(0.9, 0.1, 10.0));
        assert_float_eq::assert_f64_near!(0.41554574852714904, fidelity(0.7, 0.1, 10.0));
        assert_float_eq::assert_f64_near!(0.25002042996839313, fidelity(0.7, 0.1, 100.0));
    }
}
