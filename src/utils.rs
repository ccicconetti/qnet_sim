// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::Rng;
use serde::Serialize;
use std::io::Write;

static GIGA: u64 = 1000000000;
static SPEED_OF_LIGHT: f64 = 299792458.0;

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

/// Return the latency to cross a distance at the speed of light.
pub fn distance_to_latency(distance: f64) -> f64 {
    distance / SPEED_OF_LIGHT
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
pub fn fidelity_decay(f_init: f64, decay_rate: f64, time: f64) -> f64 {
    0.25 + (f_init - 0.25) * (-decay_rate * time).exp()
}

/// Compute the fidelity of the EPR pair resulting from the entanglement
/// swapping of two EPR pairs using (19) in 10.1103/PhysRevA.59.169.
///
/// Parameters:
/// - `f1`: fidelity of one EPR pair.
/// - `f2`: fidelity of the other EPR pair.
/// - `p1`: noise of 1-qubit operations.
/// - `p2`: noise of 2-qubit operations.
/// - `eta`: measurement noise (error rate).
pub fn fidelity_swapping(f1: f64, f2: f64, p1: f64, p2: f64, eta: f64) -> f64 {
    0.25 * (1.0
        + 1.0 / 9.0 * (p1 * p2 * (4.0 * eta * eta - 1.0)) * (4.0 * f1 - 1.0) * (4.0 * f2 - 1.0))
}

pub fn open_output_file(
    path: &str,
    filename: &str,
    append: bool,
    header: &str,
) -> anyhow::Result<std::fs::File> {
    let full_path = format!("{path}{filename}");

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
        writeln!(&mut f, "{header}")?;
    }
    Ok(f)
}

pub fn struct_to_csv<T: Serialize>(s: T) -> anyhow::Result<String> {
    let fields = struct_to_map(s)?;
    let mut ret = vec![];
    for (_name, value) in fields {
        ret.push(format!("{value}").replace(",", ";"));
    }
    Ok(ret.join(","))
}

pub fn struct_to_csv_header<T: Serialize>(s: T) -> anyhow::Result<String> {
    let fields = struct_to_map(s)?;

    let mut ret = vec![];
    for (name, _value) in fields {
        ret.push(name.replace(",", ";"));
    }
    Ok(ret.join(","))
}

fn struct_to_map<T: Serialize>(s: T) -> anyhow::Result<serde_json::Map<String, serde_json::Value>> {
    let value = serde_json::to_value(s)?;
    let mut fields = json_unflattening::flattening::flatten(&value)?;
    fields.sort_keys();
    Ok(fields)
}

/// Shuffle a container, see:
/// https://en.wikipedia.org/wiki/Fisher%E2%80%93Yates_shuffle
pub fn shuffle<T>(v: &mut [T], rng: &mut rand::rngs::StdRng) {
    for i in 0..v.len() {
        let i = v.len() - i - 1; // i goes from n-1 to 1
        let j = rng.gen_range(0..=i);
        v.swap(i, j);
    }
}

pub struct RemoveMeDir {
    test_dir: std::path::PathBuf,
}

impl RemoveMeDir {
    pub fn new(test_name: &str) -> anyhow::Result<Self> {
        let mut test_dir = std::env::temp_dir();
        test_dir.push(test_name);
        println!("temp dir created: {:?}", test_dir);
        if test_dir.exists() {
            std::fs::remove_dir_all(test_dir.to_str().unwrap())?;
        }
        std::fs::create_dir_all(test_dir.to_str().unwrap())?;

        Ok(RemoveMeDir { test_dir })
    }

    pub fn dir(&self) -> std::path::PathBuf {
        self.test_dir.clone()
    }
}

impl Drop for RemoveMeDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.test_dir);
        println!("temp dir deleted: {:?}", self.test_dir);
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::{fidelity_decay, fidelity_swapping};

    use super::{to_nanoseconds, to_seconds};

    #[test]
    fn test_to_from_nanosecs() {
        assert_eq!(42.0, to_seconds(to_nanoseconds(42.0)));
    }

    #[test]
    fn test_fidelity_decay() {
        assert_float_eq::assert_f64_near!(0.9, fidelity_decay(0.9, 0.1, 0.0));
        assert_float_eq::assert_f64_near!(0.4891216367614375, fidelity_decay(0.9, 0.1, 10.0));
        assert_float_eq::assert_f64_near!(0.41554574852714904, fidelity_decay(0.7, 0.1, 10.0));
        assert_float_eq::assert_f64_near!(0.25002042996839313, fidelity_decay(0.7, 0.1, 100.0));
    }
    #[test]
    fn test_fidelity_swapping() {
        assert_float_eq::assert_f64_near!(1.0, fidelity_swapping(1.0, 1.0, 1.0, 1.0, 1.0));
        assert_float_eq::assert_f64_near!(0.9, fidelity_swapping(0.9, 1.0, 1.0, 1.0, 1.0));
        assert_float_eq::assert_f64_near!(0.9, fidelity_swapping(1.0, 0.9, 1.0, 1.0, 1.0));
        assert_float_eq::assert_f64_near!(0.52, fidelity_swapping(0.7, 0.7, 1.0, 1.0, 1.0));
        assert_float_eq::assert_f64_near!(
            0.6706222222222222,
            fidelity_swapping(0.9, 0.9, 1.0, 1.0, 0.9)
        );
        assert_float_eq::assert_f64_near!(
            0.48554844444444445,
            fidelity_swapping(0.9, 0.9, 0.8, 0.7, 0.9)
        );
        assert_float_eq::assert_f64_near!(
            0.48554844444444445,
            fidelity_swapping(0.9, 0.9, 0.7, 0.8, 0.9)
        );
    }
}
