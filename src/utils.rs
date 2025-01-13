// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::io::Write;

static GIGA: u64 = 1000000000;

pub fn to_seconds(ns: u64) -> f64 {
    ns as f64 / GIGA as f64
}

pub fn to_nanoseconds(s: f64) -> u64 {
    (s * GIGA as f64).round() as u64
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
            if parent_path.is_dir() {
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
