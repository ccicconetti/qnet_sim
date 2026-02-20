// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::io::Write;

use crate::utils::CsvFriend;

#[derive(Debug, Clone, Default)]
pub struct MetricMetadata {
    /// Unit of measurement.
    unit: String,
    /// Brief description.
    brief: String,
    /// Always collect, regardless of the warm-up period.
    always_collect: bool,
}

impl MetricMetadata {
    pub fn new(unit: &str, brief: &str, always_collect: bool) -> Self {
        Self {
            unit: unit.to_string(),
            brief: brief.to_string(),
            always_collect,
        }
    }
}

#[derive(Debug)]
pub struct MetricDescriptionRow {
    /// Name of the metric.
    name: String,
    /// Extra data
    extra: String,
    /// Unit of measurement.
    unit: String,
    /// Brief description.
    brief: String,
}

impl markdown_tables::MarkdownTableRow for MetricDescriptionRow {
    fn column_names() -> Vec<&'static str> {
        vec!["Name", "Extra", "Unit", "Description"]
    }

    fn column_values(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.extra.clone(),
            self.unit.clone(),
            self.brief.clone(),
        ]
    }
}

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

pub enum ScalarMetricType {
    Avg,
    TimeAvg,
    Count,
    OneTime,
}

enum ScalarMetricData {
    Avg(Avg, MetricMetadata),
    TimeAvg(TimeAvg, MetricMetadata),
    Count(Count, MetricMetadata),
    OneTime(f64, MetricMetadata),
}

impl ScalarMetricData {
    fn get(&self) -> f64 {
        match &self {
            ScalarMetricData::Avg(avg, _) => avg.avg(),
            ScalarMetricData::TimeAvg(time_avg, _) => time_avg.avg(),
            ScalarMetricData::Count(count, _) => count.tot(),
            ScalarMetricData::OneTime(one_time, _) => *one_time,
        }
    }
    fn metadata(&self) -> MetricMetadata {
        match &self {
            ScalarMetricData::Avg(_, metadata) => metadata.clone(),
            ScalarMetricData::TimeAvg(_, metadata) => metadata.clone(),
            ScalarMetricData::Count(_, metadata) => metadata.clone(),
            ScalarMetricData::OneTime(_, metadata) => metadata.clone(),
        }
    }
}

#[derive(Default)]
pub struct OutputScalar {
    enabled: bool,
    warmup: u64,
    samples: std::collections::BTreeMap<String, ScalarMetricData>,
}

impl OutputScalar {
    pub fn one_time(&mut self, name: &str, value: f64) {
        if let Some(ScalarMetricData::OneTime(data, metadata)) = &mut self.samples.get_mut(name) {
            if self.enabled || metadata.always_collect {
                *data = value;
            }
        }
    }

    pub fn init(&mut self, name: &str, metric_type: ScalarMetricType, description: MetricMetadata) {
        match metric_type {
            ScalarMetricType::Avg => self.samples.insert(
                name.to_string(),
                ScalarMetricData::Avg(Avg::default(), description),
            ),
            ScalarMetricType::TimeAvg => self.samples.insert(
                name.to_string(),
                ScalarMetricData::TimeAvg(TimeAvg::default(), description),
            ),
            ScalarMetricType::Count => self.samples.insert(
                name.to_string(),
                ScalarMetricData::Count(Count::default(), description),
            ),
            ScalarMetricType::OneTime => self.samples.insert(
                name.to_string(),
                ScalarMetricData::OneTime(f64::NAN, description),
            ),
        };
    }

    pub fn avg(&mut self, name: &str, value: f64) {
        if let Some(ScalarMetricData::Avg(data, metadata)) = self.samples.get_mut(name) {
            if self.enabled || metadata.always_collect {
                data.add(value);
            }
        }
    }

    pub fn time_avg(&mut self, name: &str, now: u64, value: f64) {
        if let Some(ScalarMetricData::TimeAvg(data, metadata)) = self.samples.get_mut(name) {
            if self.enabled || metadata.always_collect {
                data.add(now, value);
            } else {
                data.update_value(value);
            }
        }
    }

    pub fn count(&mut self, name: &str) {
        if let Some(ScalarMetricData::Count(data, metadata)) = self.samples.get_mut(name) {
            if self.enabled || metadata.always_collect {
                data.add();
            }
        }
    }

    pub fn enable(&mut self, now: u64) {
        self.enabled = true;
        self.warmup = now;
        for elem in &mut self.samples.values_mut() {
            if let ScalarMetricData::TimeAvg(data, _) = elem {
                data.enable(now);
            }
        }
    }

    pub fn finish(&mut self, now: u64) {
        for elem in &mut self.samples.values_mut() {
            if let ScalarMetricData::TimeAvg(data, _) = elem {
                data.finish(now);
            }
        }
    }

    pub fn values(&self) -> std::collections::HashMap<String, f64> {
        let mut ret = std::collections::HashMap::new();
        for (name, val) in &self.samples {
            ret.insert(name.clone(), val.get());
        }
        ret
    }

    pub fn to_markdown_table(&self) -> Vec<MetricDescriptionRow> {
        let mut ret = vec![];
        for (name, desc) in &self.samples {
            let metadata = desc.metadata();
            ret.push(MetricDescriptionRow {
                name: name.to_string(),
                extra: String::default(),
                unit: metadata.unit.to_string(),
                brief: metadata.brief.to_string(),
            });
        }
        ret
    }
}

impl CsvFriend for OutputScalar {
    fn header(&self) -> String {
        self.samples
            .keys()
            .cloned()
            .collect::<Vec<String>>()
            .join(",")
    }
    fn to_csv(&self) -> String {
        self.samples
            .values()
            .map(|x| x.get().to_string())
            .collect::<Vec<String>>()
            .join(",")
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
    /// The metadata of this metric.
    pub metadata: MetricMetadata,
}

impl OutputSeriesSingle {
    /// Return summary status: min, avg, max, and number of samples.
    pub fn stats(&self) -> (f64, f64, f64, usize) {
        (
            self.values
                .iter()
                .map(|x| x.2)
                .fold(f64::INFINITY, |a, b| a.min(b)),
            self.values.iter().map(|x| x.2).fold(0.0, |a, b| a + b) / self.values.len() as f64,
            self.values
                .iter()
                .map(|x| x.2)
                .fold(f64::NEG_INFINITY, |a, b| a.max(b)),
            self.values.len(),
        )
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
        if !self.ignore.contains(name) {
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
            if self.enabled || series_single.metadata.always_collect {
                series_single.values.push((labels, time, value));
            }
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
    /// - `description`: metric metadata.
    pub fn init(&mut self, name: &str, headers: &[&str], description: MetricMetadata) {
        if !self.ignore.contains(name) {
            let series_single = self.series.entry(name.to_string()).or_default();
            series_single.headers = headers.iter().map(|x| x.to_string()).collect();
            series_single.metadata = description;
            series_single.values.clear();
        }
    }

    pub fn to_markdown_table(&self) -> Vec<MetricDescriptionRow> {
        let mut ret = vec![];
        for (name, desc) in &self.series {
            if let Some(val) = self.series.get(name) {
                ret.push(MetricDescriptionRow {
                    name: name.to_string(),
                    extra: val.headers.join(","),
                    unit: desc.metadata.unit.to_string(),
                    brief: desc.metadata.brief.to_string(),
                });
            }
        }
        ret
    }
}

#[derive(Default)]
struct StatsSingle {
    summary: inc_stats::SummStats<f64>,
    percentiles: inc_stats::Percentiles<f64>,
}

type StatsContainer =
    std::collections::HashMap<String, std::collections::HashMap<String, StatsSingle>>;
#[derive(Default)]
struct Stats {
    // hashmap of:
    //   key: metric name
    //   value: hashmap of:
    //     key: label name
    //     value: statistics
    stats: StatsContainer,
}

impl Stats {
    fn new(output_series: &OutputSeries) -> Self {
        let mut stats: StatsContainer = std::collections::HashMap::new();
        for (metric, series) in &output_series.series {
            stats.insert(metric.clone(), std::collections::HashMap::new());
            for (label, _timestamp, value) in &series.values {
                let label = label.join(",");
                let stats_single = stats.get_mut(metric).unwrap().entry(label).or_default();
                stats_single.summary.add(value);
                stats_single.percentiles.add(value);
            }
        }
        Self { stats }
    }
}

pub struct Output {
    pub scalar: OutputScalar,
    pub series: OutputSeries,
    pub config_csv: String,
    pub user_config_csv: String,
}

pub struct OutputSaveConf {
    pub output_path: String,
    pub append: bool,
    pub config_csv_header: String,
    pub user_config_csv_header: String,
    pub additional_header: String,
    pub additional_fields: String,
    pub save_config: bool,
    pub save_time_series: bool,
}

/// Save all the outputs to files.
pub fn save_outputs(outputs: Vec<Output>, conf: OutputSaveConf) -> anyhow::Result<()> {
    let additional_header = if conf.additional_header.is_empty() {
        String::default()
    } else {
        format!("{},", conf.additional_header)
    };
    let user_config_csv_header = if conf.save_config {
        format!("{},", conf.user_config_csv_header)
    } else {
        String::default()
    };
    // Open all the files.
    let mut scalar_file = crate::utils::open_output_file(
        &conf.output_path,
        "scalar.csv",
        conf.append,
        format!(
            "{}{},{}{}",
            additional_header,
            &conf.config_csv_header,
            user_config_csv_header,
            outputs.first().unwrap().scalar.header()
        )
        .as_str(),
    )?;
    let mut series_files = std::collections::HashMap::new();
    let mut series_stats_files = std::collections::HashMap::new();
    for output in &outputs {
        for (name, elem) in &output.series.series {
            if elem.values.is_empty() || series_files.contains_key(name) {
                continue;
            }
            let header_common = format!(
                "{}{},{}{}",
                additional_header,
                &conf.config_csv_header,
                user_config_csv_header,
                elem.headers.join(",")
            );
            if conf.save_time_series {
                series_files.insert(
                    name.clone(),
                    crate::utils::open_output_file(
                        &conf.output_path,
                        format!("{name}.csv").as_str(),
                        conf.append,
                        format!("{},time,value", header_common).as_str(),
                    )?,
                );
            }
            series_stats_files.insert(
                name.clone(),
                crate::utils::open_output_file(
                    &conf.output_path,
                    format!("{name}-stats.csv").as_str(),
                    conf.append,
                    format!(
                        "{},count,min,mean,max,stderr,p05,p25,p50,p75,p95",
                        header_common
                    )
                    .as_str(),
                )?,
            );
        }
    }

    // Dump the data to files.
    for output in outputs {
        let user_config_csv = if conf.save_config {
            format!("{},", output.user_config_csv)
        } else {
            String::default()
        };
        let additional_fields = if conf.additional_fields.is_empty() {
            String::default()
        } else {
            format!("{},", conf.additional_fields)
        };
        writeln!(
            &mut scalar_file,
            "{}{},{}{}",
            additional_fields,
            output.config_csv,
            user_config_csv,
            output.scalar.to_csv()
        )?;

        if conf.save_time_series {
            for (name, elem) in &output.series.series {
                if let Some(series_file) = series_files.get_mut(name) {
                    for (labels, time, value) in &elem.values {
                        writeln!(
                            series_file,
                            "{}{},{}{},{},{}",
                            additional_fields,
                            output.config_csv,
                            user_config_csv,
                            labels.join(","),
                            time,
                            value
                        )?;
                    }
                }
            }
        }

        let stats_all = Stats::new(&output.series);
        for (name, stats) in &stats_all.stats {
            if let Some(series_stats_file) = series_stats_files.get_mut(name) {
                for (label, samples) in stats {
                    let percentiles = samples
                        .percentiles
                        .percentiles([0.05, 0.25, 0.50, 0.75, 0.95])
                        .unwrap_or_default()
                        .unwrap_or_default();
                    writeln!(
                        series_stats_file,
                        "{}{},{}{},{},{},{},{},{},{}",
                        additional_fields,
                        output.config_csv,
                        user_config_csv,
                        label,
                        samples.summary.count(),
                        samples.summary.min().unwrap_or_default(),
                        samples.summary.mean().unwrap_or_default(),
                        samples.summary.max().unwrap_or_default(),
                        samples.summary.standard_error().unwrap_or_default(),
                        percentiles
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<String>>()
                            .join(",")
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
            scalar.init(
                "metric",
                ScalarMetricType::TimeAvg,
                MetricMetadata::default(),
            );
            scalar.enable(*warmup);
            scalar.time_avg("metric", 20, 1.0);
            scalar.time_avg("metric", 30, 2.0);
            scalar.time_avg("metric", 40, 1.0);
            scalar.time_avg("metric", 50, 3.0);
            scalar.finish(100);

            if let ScalarMetricData::TimeAvg(metric, _) = scalar.samples.get("metric").unwrap() {
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
        }

        Ok(())
    }

    #[test]
    fn test_output_series() -> anyhow::Result<()> {
        let mut output_series = OutputSeries::new(std::collections::HashSet::from([
            "to-be-ignored".to_string(),
        ]));

        output_series.init("my-metric-0", &[], MetricMetadata::default());
        output_series.init("my-metric-1", &["x"], MetricMetadata::default());
        output_series.init("my-metric-2", &["x", "y"], MetricMetadata::default());

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

    #[test]
    fn test_output_stats() -> anyhow::Result<()> {
        let mut output_series = OutputSeries::new(std::collections::HashSet::new());

        output_series.init("my-metric-0", &[], MetricMetadata::default());
        output_series.init("my-metric-1", &["x"], MetricMetadata::default());
        output_series.init("my-metric-2", &["x", "y"], MetricMetadata::default());

        output_series.add("my-metric-0", vec![], 1.0, 1.1);
        output_series.add("my-metric-1", vec!["a".to_string()], 2.0, 2.1);
        output_series.add(
            "my-metric-2",
            vec!["a".to_string(), "b".to_string()],
            3.0,
            3.1,
        );

        output_series.enable();

        for i in 0..10 {
            output_series.add("my-metric-0", vec![], 1.0, i as f64);
            output_series.add("my-metric-1", vec!["a".to_string()], 2.0, i as f64);
            output_series.add(
                "my-metric-2",
                vec!["a".to_string(), "b".to_string()],
                3.0,
                i as f64,
            );
        }

        let stats_all = Stats::new(&output_series);

        for (metric, stats) in stats_all.stats {
            println!("{}", metric);
            for (label, samples) in stats {
                let quarts = samples
                    .percentiles
                    .percentiles(&[0.75, 0.25, 0.5])
                    .unwrap()
                    .unwrap();
                println!("{} {:?} {:?}", label, samples.summary, quarts);

                assert_eq!(10, samples.summary.count());
                assert_float_eq::assert_f64_near!(4.5, samples.summary.mean().unwrap());
                assert_float_eq::assert_f64_near!(6.75, quarts[0]);
                assert_float_eq::assert_f64_near!(2.25, quarts[1]);
                assert_float_eq::assert_f64_near!(4.5, quarts[2]);
            }
        }

        Ok(())
    }
}
