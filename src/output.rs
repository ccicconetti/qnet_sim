// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::io::Write;

use crate::utils::CsvFriend;

#[derive(Debug)]
pub enum Sample {
    ScalarOneTime(String, f64),
    ScalarAvg(String, f64),
    ScalarTimeAvg(String, f64),
    ScalarCount(String),
    Series(String, Vec<String>, f64),
}

#[derive(Default)]
struct Count {
    num: u64,
}

impl Count {
    pub fn add(&mut self) {
        self.num += 1;
    }
    pub fn tot(&self) -> f64 {
        self.num as f64
    }
}

#[derive(Default)]
struct Avg {
    sum: kahan::KahanSum<f64>,
    num: u64,
}

impl Avg {
    pub fn add(&mut self, value: f64) {
        self.sum += value;
        self.num += 1;
    }
    pub fn avg(&self) -> f64 {
        self.sum.sum() / self.num as f64
    }
}

struct TimeAvg {
    last_update: u64,
    last_value: f64,
    sum_values: f64,
    sum_time: f64,
}

impl Default for TimeAvg {
    fn default() -> Self {
        Self {
            last_update: u64::MAX,
            last_value: 0.0,
            sum_values: 0.0,
            sum_time: 0.0,
        }
    }
}

impl TimeAvg {
    pub fn add(&mut self, now: u64, value: f64) {
        if self.last_update != u64::MAX {
            let delta = (now - self.last_update) as f64;
            self.sum_values += delta * self.last_value;
            self.sum_time += delta;
        }
        self.last_update = now;
        self.last_value = value;
    }
    pub fn enable(&mut self, now: u64) {
        self.last_update = now;
    }
    pub fn update_value(&mut self, value: f64) {
        self.last_value = value;
    }
    pub fn finish(&mut self, now: u64) {
        self.add(now, self.last_value);
    }
    pub fn avg(&self) -> f64 {
        self.sum_values / self.sum_time
    }
}

#[derive(Default)]
pub struct OutputScalar {
    enabled: bool,
    warmup: u64,
    one_time: std::collections::BTreeMap<String, f64>,
    avg: std::collections::BTreeMap<String, Avg>,
    time_avg: std::collections::BTreeMap<String, TimeAvg>,
    count: std::collections::BTreeMap<String, Count>,
}

pub enum ScalarMetricType {
    Avg,
    TimeAvg,
    Count,
}

impl OutputScalar {
    pub fn one_time(&mut self, name: &str, value: f64) {
        if self.enabled {
            self.one_time.insert(name.to_string(), value);
        }
    }

    pub fn init(&mut self, name: &str, metric_type: ScalarMetricType) {
        match metric_type {
            ScalarMetricType::Avg => {
                self.avg.insert(name.to_string(), Avg::default());
            }
            ScalarMetricType::TimeAvg => {
                self.time_avg.insert(name.to_string(), TimeAvg::default());
            }
            ScalarMetricType::Count => {
                self.count.insert(name.to_string(), Count::default());
            }
        };
    }

    pub fn avg(&mut self, name: &str, value: f64) {
        let entry = self
            .avg
            .get_mut(name)
            .unwrap_or_else(|| panic!("uninitialized metric {name}"));
        if self.enabled {
            entry.add(value);
        }
    }

    pub fn time_avg(&mut self, name: &str, now: u64, value: f64) {
        let entry = self
            .time_avg
            .get_mut(name)
            .unwrap_or_else(|| panic!("uninitialized metric {name}"));
        if self.enabled {
            entry.add(now, value);
        } else {
            entry.update_value(value);
        }
    }

    pub fn count(&mut self, name: &str) {
        let entry = self
            .count
            .get_mut(name)
            .unwrap_or_else(|| panic!("uninitialized metric {name}"));
        if self.enabled {
            entry.add();
        }
    }

    pub fn enable(&mut self, now: u64) {
        self.enabled = true;
        self.warmup = now;
        for elem in &mut self.time_avg.values_mut() {
            elem.enable(now);
        }
    }

    pub fn finish(&mut self, now: u64) {
        for elem in &mut self.time_avg.values_mut() {
            elem.finish(now);
        }
    }
}

impl CsvFriend for OutputScalar {
    fn header(&self) -> String {
        format!(
            "{},{},{},{}",
            self.one_time
                .keys()
                .cloned()
                .collect::<Vec<String>>()
                .join(","),
            self.avg.keys().cloned().collect::<Vec<String>>().join(","),
            self.time_avg
                .keys()
                .cloned()
                .collect::<Vec<String>>()
                .join(","),
            self.count
                .keys()
                .cloned()
                .collect::<Vec<String>>()
                .join(",")
        )
    }
    fn to_csv(&self) -> String {
        format!(
            "{},{},{},{}",
            self.one_time
                .values()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(","),
            self.avg
                .values()
                .map(|x| x.avg().to_string())
                .collect::<Vec<String>>()
                .join(","),
            self.time_avg
                .values()
                .map(|x| x.avg().to_string())
                .collect::<Vec<String>>()
                .join(","),
            self.count
                .values()
                .map(|x| x.tot().to_string())
                .collect::<Vec<String>>()
                .join(",")
        )
    }
}

#[derive(Default)]
pub struct OutputSeriesSingle {
    /// CSV headers, which explains the meaning of the labels.
    pub headers: Vec<String>,
    /// Time series. Each sample is associated with:
    /// - a vector of string labels
    /// - the time when the sample was collected
    /// - the value of the sample
    pub values: Vec<(Vec<String>, f64, f64)>,
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
                .unwrap_or_else(|| panic!("uninitialized metric {name}"));
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
    pub scalar: OutputScalar,
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
    save_config: bool,
) -> anyhow::Result<()> {
    let header_comma = if additional_header.is_empty() {
        ""
    } else {
        ","
    };

    // Open all the files.
    let mut scalar_file = crate::utils::open_output_file(
        output_path,
        "scalar.csv",
        append,
        format!(
            "{}{}{}{}{}",
            additional_header,
            header_comma,
            if save_config { config_csv_header } else { "" },
            if save_config { "," } else { "" },
            outputs.first().unwrap().scalar.header()
        )
        .as_str(),
    )?;
    let mut series_files = std::collections::HashMap::new();
    for output in &outputs {
        for (name, elem) in &output.series.series {
            if elem.values.is_empty() || series_files.contains_key(name) {
                continue;
            }
            let series_file = crate::utils::open_output_file(
                output_path,
                format!("{name}.csv").as_str(),
                append,
                format!(
                    "{}{}{}{}{},time,value",
                    additional_header,
                    header_comma,
                    if save_config { &config_csv_header } else { "" },
                    if save_config { "," } else { "" },
                    elem.headers.join(",")
                )
                .as_str(),
            )?;
            series_files.insert(name.clone(), series_file);
        }
    }

    // Dump the data to files.
    for output in outputs {
        let config_csv = if save_config { &output.config_csv } else { "" };
        let config_comma = if save_config { "," } else { "" };
        writeln!(
            &mut scalar_file,
            "{}{}{}{}{}",
            additional_fields,
            header_comma,
            config_csv,
            config_comma,
            output.scalar.to_csv()
        )?;

        for (name, elem) in &output.series.series {
            if let Some(series_file) = series_files.get_mut(name) {
                for (labels, time, value) in &elem.values {
                    writeln!(
                        series_file,
                        "{}{}{}{}{},{},{}",
                        additional_fields,
                        header_comma,
                        config_csv,
                        config_comma,
                        labels.join(","),
                        time,
                        value
                    )?;
                }
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
            let mut scalar = OutputScalar::default();
            scalar.init("metric", ScalarMetricType::TimeAvg);
            scalar.enable(*warmup);
            scalar.time_avg("metric", 20, 1.0);
            scalar.time_avg("metric", 30, 2.0);
            scalar.time_avg("metric", 40, 1.0);
            scalar.time_avg("metric", 50, 3.0);
            scalar.finish(100);

            let metric = scalar.time_avg.get("metric").unwrap();

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
