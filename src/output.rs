// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::io::Write;

use crate::utils::CsvFriend;

#[derive(Debug)]
pub enum Sample {
    SingleOneTime(String, f64),
    SingleTimeAvg(String, f64),
    Series(String, Vec<String>, f64),
}

struct TimeAvg {
    last_update: u64,
    last_value: f64,
    sum_values: f64,
    sum_time: f64,
}

impl TimeAvg {
    pub fn new(last_update: u64) -> Self {
        Self {
            last_update,
            last_value: 0.0,
            sum_values: 0.0,
            sum_time: 0.0,
        }
    }
    pub fn avg(&self) -> f64 {
        self.sum_values / self.sum_time
    }
}

#[derive(Default)]
pub struct OutputSingle {
    enabled: bool,
    warmup: u64,
    one_time: std::collections::BTreeMap<String, f64>,
    time_avg: std::collections::BTreeMap<String, TimeAvg>,
}

impl OutputSingle {
    pub fn one_time(&mut self, name: &str, value: f64) {
        if self.enabled {
            self.one_time.insert(name.to_string(), value);
        }
    }

    pub fn time_avg(&mut self, name: &str, now: u64, value: f64) {
        let entry = self
            .time_avg
            .entry(name.to_string())
            .or_insert_with(|| TimeAvg::new(self.warmup));
        if self.enabled {
            let delta = (now - entry.last_update) as f64;
            entry.sum_values += delta * entry.last_value;
            entry.sum_time += delta;
            entry.last_update = now;
        }
        entry.last_value = value;
    }

    pub fn enable(&mut self, now: u64) {
        self.enabled = true;
        self.warmup = now;
        for elem in &mut self.time_avg.values_mut() {
            elem.last_update = now;
        }
    }

    pub fn finish(&mut self, now: u64) {
        for entry in &mut self.time_avg.values_mut() {
            let delta = (now - entry.last_update) as f64;
            entry.sum_values += delta * entry.last_value;
            entry.sum_time += delta;
            entry.last_update = now;
        }
    }
}

impl CsvFriend for OutputSingle {
    fn header(&self) -> String {
        format!(
            "{},{}",
            self.one_time
                .keys()
                .cloned()
                .collect::<Vec<String>>()
                .join(","),
            self.time_avg
                .keys()
                .cloned()
                .collect::<Vec<String>>()
                .join(",")
        )
    }
    fn to_csv(&self) -> String {
        format!(
            "{},{}",
            self.one_time
                .values()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(","),
            self.time_avg
                .values()
                .map(|x| x.avg().to_string())
                .collect::<Vec<String>>()
                .join(",")
        )
    }
}

pub struct OutputSeriesSingle {
    /// CSV headers, which explains the meaning of the labels.
    pub headers: Vec<String>,
    /// Time series. Each sample is associated with:
    /// - a vector of string labels
    /// - the time when the sample was collected
    /// - the value of the sample
    pub values: Vec<(Vec<String>, f64, f64)>,
}

impl Default for OutputSeriesSingle {
    fn default() -> Self {
        Self {
            headers: vec![],
            values: vec![],
        }
    }
}

/// Series of values.
/// The values are not recorded until `enabled()` is called.
/// Each series is associated with a name (with optional header) and a label.
#[derive(Default)]
pub struct OutputSeries {
    enabled: bool,
    ignore: std::collections::HashSet<String>,
    pub series: std::collections::HashMap<String, OutputSeriesSingle>,
}

impl OutputSeries {
    pub fn new(ignore: std::collections::HashSet<String>) -> Self {
        Self {
            enabled: false,
            ignore,
            series: std::collections::HashMap::new(),
        }
    }

    /// Add a new value to a series metric.
    ///
    /// Parameters:
    /// - `name`: the metric name.
    /// - `labels`: the labels associated with the value.
    /// - `time`: timestamp of the value.
    /// - `value`: the value added, if collection is enabled.
    ///
    /// The function panics if the headers have not been set or if number of
    /// labels is different from the number of elements expected based on the
    /// headers.
    pub fn add(&mut self, name: &str, labels: Vec<String>, time: f64, value: f64) {
        if self.enabled && !self.ignore.contains(name) {
            let series_single = self
                .series
                .get_mut(name)
                .unwrap_or_else(|| panic!("uninitialized metric {}", name));
            assert!(
                series_single.headers.len() == labels.len(),
                "wrong numbers of labels for metric {}: expected {}, found {}",
                name,
                series_single.headers.len(),
                labels.len()
            );
            series_single.values.push((labels, time, value));
        }
    }

    /// Enable the collection of values.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Set the headers for a given metric and reset any previous values.
    /// Parameters:
    /// - `name`: the name of the metric.
    /// - `headers`: the header to be used for serializing values.
    pub fn set_headers(&mut self, name: &str, headers: &[&str]) {
        if !self.ignore.contains(name) {
            let series_single = self.series.entry(name.to_string()).or_default();
            series_single.headers = headers.iter().map(|x| x.to_string()).collect();
            series_single.values.clear();
        }
    }
}

pub struct Output {
    pub single: OutputSingle,
    pub series: OutputSeries,
    pub config_csv: String,
}

/// Save all the outputs to files.
pub fn save_outputs(
    outputs: Vec<Output>,
    output_path: &str,
    append: bool,
    config_csv_header: &str,
    additional_header: &str,
    additional_fields: &str,
) -> anyhow::Result<()> {
    let mut single_file = crate::utils::open_output_file(
        output_path,
        "single.csv",
        append,
        format!(
            "{}{},{}",
            additional_header,
            config_csv_header,
            outputs.first().unwrap().single.header()
        )
        .as_str(),
    )?;

    for output in outputs {
        writeln!(
            &mut single_file,
            "{}{},{}",
            additional_fields,
            output.config_csv,
            output.single.to_csv()
        )?;

        for (name, elem) in &output.series.series {
            if elem.values.is_empty() {
                continue;
            }
            let mut series_file = crate::utils::open_output_file(
                output_path,
                format!("{name}.csv").as_str(),
                append,
                format!(
                    "{}{},{},time,value",
                    additional_header,
                    &config_csv_header,
                    elem.headers.join(",")
                )
                .as_str(),
            )?;
            for (labels, time, value) in &elem.values {
                writeln!(
                    &mut series_file,
                    "{}{},{},{},{}",
                    additional_fields,
                    output.config_csv,
                    labels.join(","),
                    time,
                    value
                )?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_avg() -> anyhow::Result<()> {
        let warmups = [0, 5];
        let expected_values = [1.9, 2.0];
        for (warmup, expected_value) in warmups.iter().zip(expected_values.iter()) {
            let mut single = OutputSingle::default();
            single.enable(*warmup);
            single.time_avg("metric", 20, 1.0);
            single.time_avg("metric", 30, 2.0);
            single.time_avg("metric", 40, 1.0);
            single.time_avg("metric", 50, 3.0);
            single.finish(100);

            let metric = single.time_avg.get("metric").unwrap();

            assert!(
                metric.avg() == *expected_value,
                "{} != {} (sum {}, time {}, warmup {})",
                metric.avg(),
                *expected_value,
                metric.sum_values,
                metric.sum_time,
                warmup
            );
        }

        Ok(())
    }

    #[test]
    fn test_output_series() -> anyhow::Result<()> {
        let mut output_series = OutputSeries::new(std::collections::HashSet::from([
            "to-be-ignored".to_string(),
        ]));

        output_series.set_headers("my-metric-0", &[]);
        output_series.set_headers("my-metric-1", &["x"]);
        output_series.set_headers("my-metric-2", &["x", "y"]);

        assert!(!output_series.enabled);

        output_series.add("my-metric-0", vec![], 1.0, 1.1);
        output_series.add("my-metric-1", vec!["a".to_string()], 2.0, 2.1);
        output_series.add(
            "my-metric-2",
            vec!["a".to_string(), "b".to_string()],
            3.0,
            3.1,
        );

        for single in output_series.series.values() {
            assert_eq!(0, single.values.len());
        }

        output_series.enable();

        output_series.add("to-be-ignored", vec![], 1.0, 1.1);
        assert!(output_series
            .series
            .keys()
            .find(|x| *x == "to-be-ignored")
            .is_none());

        for _ in 0..10 {
            output_series.add("my-metric-0", vec![], 1.0, 1.1);
            output_series.add("my-metric-1", vec!["a".to_string()], 2.0, 2.1);
            output_series.add(
                "my-metric-2",
                vec!["a".to_string(), "b".to_string()],
                3.0,
                3.1,
            );
        }

        for single in output_series.series.values() {
            assert_eq!(10, single.values.len());
        }

        Ok(())
    }
}
