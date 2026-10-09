// SPDX-FileCopyrightText: © 2026 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use super::{Output, OutputSaveConf, Stats};
use anyhow::{ensure, Context};
use parquet::basic::{Compression, LogicalType, Repetition, Type as PhysicalType};
use parquet::column::writer::ColumnCloseResult;
use parquet::data_type::{ByteArray, ByteArrayType, DoubleType, Int64Type};
use parquet::file::properties::WriterProperties;
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::file::writer::SerializedFileWriter;
use parquet::schema::types::Type;
use std::collections::BTreeSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const BATCH_SIZE: usize = 8192;
const STAT_NAMES: [&str; 9] = [
    "min", "mean", "max", "stderr", "p05", "p25", "p50", "p75", "p95",
];

/// Save the same scalar, statistics, and optional time-series tables as
/// `save_outputs`, using the native Apache Parquet writer and Snappy compression.
/// Measurements are DOUBLE, counts are UINT64, and labels/configuration fields
/// are UTF-8 strings. Appending requires an identical schema and copies existing
/// row groups to a temporary file before atomically replacing each destination.
pub fn save_outputs_parquet(outputs: Vec<Output>, conf: OutputSaveConf) -> anyhow::Result<()> {
    let Some(first) = outputs.first() else {
        return Ok(());
    };
    let mut common_headers = split_fields(&conf.additional_header);
    common_headers.extend(split_fields(&conf.config_csv_header));
    if conf.save_config {
        common_headers.extend(split_fields(&conf.user_config_csv_header));
    }
    let prefixes = outputs
        .iter()
        .map(|output| {
            let mut fields = fields_for_header(&conf.additional_header, &conf.additional_fields)?;
            fields.extend(fields_for_header(
                &conf.config_csv_header,
                &output.config_csv,
            )?);
            if conf.save_config {
                fields.extend(fields_for_header(
                    &conf.user_config_csv_header,
                    &output.user_config_csv,
                )?);
            }
            ensure!(
                fields.len() == common_headers.len(),
                "configuration/header field count mismatch"
            );
            Ok(fields)
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let scalar_names: Vec<_> = first.scalar.samples.keys().map(String::as_str).collect();
    let mut scalar = Table::new(&conf, "scalar", &common_headers, false, &scalar_names)?;
    for (output, prefix) in outputs.iter().zip(&prefixes) {
        ensure!(
            output
                .scalar
                .samples
                .keys()
                .map(String::as_str)
                .eq(scalar_names.iter().copied()),
            "scalar metrics differ between outputs"
        );
        let values: Vec<_> = output
            .scalar
            .samples
            .values()
            .map(|sample| sample.get())
            .collect();
        scalar.push(prefix.iter().copied(), None, &values)?;
    }
    scalar.finish()?;

    let names: BTreeSet<_> = outputs
        .iter()
        .flat_map(|output| {
            output
                .series
                .series
                .iter()
                .filter(|(_, series)| !series.values.is_empty())
                .map(|(name, _)| name)
        })
        .collect();
    for name in names {
        let series_outputs: Vec<_> = outputs
            .iter()
            .zip(&prefixes)
            .filter_map(|(output, prefix)| {
                output
                    .series
                    .series
                    .get(name)
                    .filter(|series| !series.values.is_empty())
                    .map(|series| (series, prefix))
            })
            .collect();
        let headers = &series_outputs[0].0.headers;
        let mut text_headers = common_headers.clone();
        text_headers.extend(headers.iter().map(String::as_str));
        let mut stats_file = Table::new(
            &conf,
            &format!("{name}-stats"),
            &text_headers,
            true,
            &STAT_NAMES,
        )?;
        let mut series_file = if conf.save_time_series {
            Some(Table::new(
                &conf,
                name,
                &text_headers,
                false,
                &["time", "value"],
            )?)
        } else {
            None
        };
        for (series, prefix) in series_outputs {
            ensure!(
                &series.headers == headers,
                "headers differ for metric {name}"
            );
            if let Some(file) = &mut series_file {
                for (label_index, time, value) in &series.values {
                    let labels = series
                        .labels
                        .get(*label_index as usize)
                        .context("invalid label index")?;
                    file.push(
                        prefix
                            .iter()
                            .copied()
                            .chain(labels.iter().map(String::as_str)),
                        None,
                        &[*time, *value],
                    )?;
                }
            }
            for (label_index, samples) in Stats::for_series(series) {
                let labels = series
                    .labels
                    .get(label_index as usize)
                    .context("invalid label index")?;
                let percentiles = samples
                    .percentiles
                    .percentiles([0.05, 0.25, 0.50, 0.75, 0.95])
                    .unwrap_or_default()
                    .unwrap_or_default();
                let mut values = vec![
                    samples.summary.min().unwrap_or_default(),
                    samples.summary.mean().unwrap_or_default(),
                    samples.summary.max().unwrap_or_default(),
                    samples.summary.standard_error().unwrap_or_default(),
                ];
                values.extend(percentiles);
                stats_file.push(
                    prefix
                        .iter()
                        .copied()
                        .chain(labels.iter().map(String::as_str)),
                    Some(samples.summary.count()),
                    &values,
                )?;
            }
        }
        if let Some(file) = series_file {
            file.finish()?;
        }
        stats_file.finish()?;
    }
    Ok(())
}

fn split_fields(fields: &str) -> Vec<&str> {
    if fields.is_empty() {
        vec![]
    } else {
        fields.split(',').collect()
    }
}

fn fields_for_header<'a>(header: &str, fields: &'a str) -> anyhow::Result<Vec<&'a str>> {
    let values = if header.is_empty() && fields.is_empty() {
        vec![]
    } else {
        fields.split(',').collect()
    };
    ensure!(
        values.len() == split_fields(header).len(),
        "field count does not match header {header:?}"
    );
    Ok(values)
}

/// Bounded column buffers avoid materializing a second copy of an entire series.
struct Table {
    path: PathBuf,
    temporary: tempfile::NamedTempFile,
    writer: SerializedFileWriter<File>,
    text: Vec<Vec<ByteArray>>,
    counts: Option<Vec<i64>>,
    numbers: Vec<Vec<f64>>,
    rows: usize,
}

impl Table {
    fn new(
        conf: &OutputSaveConf,
        name: &str,
        text: &[&str],
        count: bool,
        numbers: &[&str],
    ) -> anyhow::Result<Self> {
        let mut fields = vec![];
        let mut names = BTreeSet::new();
        for (name, physical, logical) in text
            .iter()
            .map(|name| (*name, PhysicalType::BYTE_ARRAY, Some(LogicalType::String)))
            .chain(count.then_some((
                "count",
                PhysicalType::INT64,
                Some(LogicalType::integer(64, false)),
            )))
            .chain(
                numbers
                    .iter()
                    .map(|name| (*name, PhysicalType::DOUBLE, None)),
            )
        {
            ensure!(
                !name.is_empty() && names.insert(name),
                "empty or duplicate Parquet column name: {name:?}"
            );
            fields.push(Arc::new(
                Type::primitive_type_builder(name, physical)
                    .with_repetition(Repetition::REQUIRED)
                    .with_logical_type(logical)
                    .build()?,
            ));
        }
        ensure!(
            !fields.is_empty(),
            "cannot write a Parquet table without columns"
        );
        let schema = Arc::new(
            Type::group_type_builder("output")
                .with_fields(fields)
                .build()?,
        );
        // Match the CSV writer's output-path prefix semantics.
        let path = PathBuf::from(format!("{}{name}.parquet", conf.output_path));
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent)?;
        let temporary = tempfile::NamedTempFile::new_in(parent)?;
        let properties = Arc::new(
            WriterProperties::builder()
                .set_compression(Compression::SNAPPY)
                .build(),
        );
        let mut writer =
            SerializedFileWriter::new(temporary.reopen()?, schema.clone(), properties)?;
        if conf.append && path.exists() && path.metadata()?.len() > 0 {
            let file = File::open(&path)?;
            let reader = SerializedFileReader::new(file.try_clone()?)?;
            ensure!(
                reader.metadata().file_metadata().schema() == schema.as_ref(),
                "cannot append to {}: Parquet schema differs",
                path.display()
            );
            for group in reader.metadata().row_groups() {
                let mut output_group = writer.next_row_group()?;
                for column in group.columns() {
                    output_group.append_column(
                        &file,
                        ColumnCloseResult {
                            bytes_written: column.compressed_size() as u64,
                            rows_written: group.num_rows() as u64,
                            metadata: column.clone(),
                            bloom_filter: None,
                            column_index: None,
                            offset_index: None,
                        },
                    )?;
                }
                output_group.close()?;
            }
        }
        Ok(Self {
            path,
            temporary,
            writer,
            text: vec![vec![]; text.len()],
            counts: count.then(Vec::new),
            numbers: vec![vec![]; numbers.len()],
            rows: 0,
        })
    }

    fn push<'a>(
        &mut self,
        text: impl Iterator<Item = &'a str>,
        count: Option<u64>,
        numbers: &[f64],
    ) -> anyhow::Result<()> {
        let text: Vec<_> = text.collect();
        ensure!(
            text.len() == self.text.len()
                && numbers.len() == self.numbers.len()
                && count.is_some() == self.counts.is_some(),
            "row does not match Parquet schema for {}",
            self.path.display()
        );
        for (column, value) in self.text.iter_mut().zip(text) {
            column.push(ByteArray::from(value));
        }
        if let (Some(column), Some(count)) = (&mut self.counts, count) {
            // Parquet represents UINT64 using the bit pattern of a physical INT64.
            column.push(count as i64);
        }
        for (column, value) in self.numbers.iter_mut().zip(numbers) {
            column.push(*value);
        }
        self.rows += 1;
        if self.rows == BATCH_SIZE {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        if self.rows == 0 {
            return Ok(());
        }
        let mut group = self.writer.next_row_group()?;
        for values in &mut self.text {
            let mut column = group.next_column()?.context("missing text column")?;
            column
                .typed::<ByteArrayType>()
                .write_batch(values, None, None)?;
            column.close()?;
            values.clear();
        }
        if let Some(values) = &mut self.counts {
            let mut column = group.next_column()?.context("missing count column")?;
            column
                .typed::<Int64Type>()
                .write_batch(values, None, None)?;
            column.close()?;
            values.clear();
        }
        for values in &mut self.numbers {
            let mut column = group.next_column()?.context("missing numeric column")?;
            column
                .typed::<DoubleType>()
                .write_batch(values, None, None)?;
            column.close()?;
            values.clear();
        }
        group.close()?;
        self.rows = 0;
        Ok(())
    }

    fn finish(mut self) -> anyhow::Result<()> {
        self.flush()?;
        self.writer.close()?;
        self.temporary.persist(&self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{MetricMetadata, OutputScalar, OutputSeries, ScalarMetricType};
    use parquet::record::{Row, RowAccessor};

    fn config(path: &Path) -> OutputSaveConf {
        OutputSaveConf {
            output_path: format!("{}/", path.display()),
            append: false,
            config_csv_header: "seed".to_string(),
            user_config_csv_header: "duration".to_string(),
            additional_header: "experiment".to_string(),
            additional_fields: String::new(),
            save_config: true,
            save_time_series: true,
        }
    }

    fn output(seed: u64) -> Output {
        let mut scalar = OutputScalar::default();
        scalar.init(
            "fidelity",
            ScalarMetricType::OneTime,
            MetricMetadata::default(),
        );
        scalar.init(
            "missing",
            ScalarMetricType::OneTime,
            MetricMetadata::default(),
        );
        scalar.enable(0);
        scalar.one_time("fidelity", 0.75);
        let mut series = OutputSeries::default();
        series.init(
            "latency",
            &["source", "destination"],
            MetricMetadata::default(),
        );
        series.init("unlabelled", &[], MetricMetadata::default());
        series.init("empty", &["node"], MetricMetadata::default());
        series.enable();
        let a = vec!["node,α".to_string(), "\"b\"\n".to_string()];
        let b = vec!["c".to_string(), "d".to_string()];
        // Each simulation may assign a different index to the same labels.
        if seed == 0 {
            series.add("latency", a.clone(), 1.0, 2.0);
            series.add("latency", b, 2.0, 8.0);
        } else {
            series.add("latency", b, 2.0, 8.0);
            series.add("latency", a.clone(), 1.0, 2.0);
        }
        series.add("latency", a, 3.0, 4.0);
        series.add("unlabelled", vec![], 5.0, 6.0);
        Output {
            scalar,
            series,
            config_csv: seed.to_string(),
            user_config_csv: "10".to_string(),
        }
    }

    fn reader(path: &Path, name: &str) -> anyhow::Result<SerializedFileReader<File>> {
        Ok(SerializedFileReader::new(File::open(
            path.join(format!("{name}.parquet")),
        )?)?)
    }

    fn rows(path: &Path, name: &str) -> anyhow::Result<Vec<Row>> {
        Ok(reader(path, name)?
            .get_row_iter(None)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    #[test]
    fn round_trip_outputs() -> anyhow::Result<()> {
        for save_time_series in [false, true] {
            for save_config in [false, true] {
                let dir = tempfile::tempdir()?;
                let mut conf = config(dir.path());
                conf.save_time_series = save_time_series;
                conf.save_config = save_config;
                save_outputs_parquet(vec![output(0), output(1)], conf)?;
                let prefix_len = if save_config { 3 } else { 2 };
                let scalars = rows(dir.path(), "scalar")?;
                assert_eq!(scalars.len(), 2);
                for (seed, row) in scalars.iter().enumerate() {
                    assert_eq!(row.get_string(0)?, "");
                    assert_eq!(row.get_string(1)?, &seed.to_string());
                    if save_config {
                        assert_eq!(row.get_string(2)?, "10");
                    }
                    assert_eq!(row.get_double(prefix_len)?, 0.75);
                    assert!(row.get_double(prefix_len + 1)?.is_nan());
                }
                let schema = reader(dir.path(), "latency-stats")?;
                let names: Vec<_> = schema
                    .metadata()
                    .file_metadata()
                    .schema_descr()
                    .columns()
                    .iter()
                    .map(|column| column.name())
                    .collect();
                let mut expected = vec!["experiment", "seed"];
                if save_config {
                    expected.push("duration");
                }
                expected.extend(["source", "destination", "count"]);
                expected.extend(STAT_NAMES);
                assert_eq!(names, expected);
                let stats = rows(dir.path(), "latency-stats")?;
                assert_eq!(stats.len(), 4);
                for row in &stats {
                    let count_col = prefix_len + 2;
                    if row.get_string(prefix_len)? == "node,α" {
                        assert_eq!(row.get_string(prefix_len + 1)?, "\"b\"\n");
                        assert_eq!(row.get_ulong(count_col)?, 2);
                        let expected = [2.0, 3.0, 4.0, 1.0, 2.1, 2.5, 3.0, 3.5, 3.9];
                        for (index, value) in expected.iter().enumerate() {
                            assert!((row.get_double(count_col + 1 + index)? - value).abs() < 1e-12);
                        }
                    } else {
                        assert_eq!(row.get_string(prefix_len)?, "c");
                        assert_eq!(row.get_ulong(count_col)?, 1);
                        assert_eq!(row.get_double(count_col + 2)?, 8.0);
                    }
                }
                assert_eq!(rows(dir.path(), "unlabelled-stats")?.len(), 2);
                assert!(!dir.path().join("empty-stats.parquet").exists());
                assert!(!dir.path().join("empty.parquet").exists());
                assert!(!dir.path().join("scalar.csv").exists());
                if save_time_series {
                    let series = rows(dir.path(), "latency")?;
                    assert_eq!(series.len(), 6);
                    for (row, (label, time, value)) in series.iter().zip([
                        ("node,α", 1.0, 2.0),
                        ("c", 2.0, 8.0),
                        ("node,α", 3.0, 4.0),
                        ("c", 2.0, 8.0),
                        ("node,α", 1.0, 2.0),
                        ("node,α", 3.0, 4.0),
                    ]) {
                        assert_eq!(row.get_string(prefix_len)?, label);
                        assert_eq!(row.get_double(prefix_len + 2)?, time);
                        assert_eq!(row.get_double(prefix_len + 3)?, value);
                    }
                    assert_eq!(rows(dir.path(), "unlabelled")?.len(), 2);
                } else {
                    assert!(!dir.path().join("latency.parquet").exists());
                    assert!(!dir.path().join("unlabelled.parquet").exists());
                }
            }
        }
        Ok(())
    }

    #[test]
    fn append_and_overwrite() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        // Appending to a missing file must also work.
        for seed in 0..3 {
            let mut conf = config(dir.path());
            conf.append = true;
            save_outputs_parquet(vec![output(seed)], conf)?;
        }
        for (name, expected) in [("scalar", 3), ("latency", 9), ("latency-stats", 6)] {
            assert_eq!(rows(dir.path(), name)?.len(), expected);
        }
        let series = rows(dir.path(), "latency")?;
        for (seed, batch) in series.chunks(3).enumerate() {
            assert!(batch
                .iter()
                .all(|row| row.get_string(1).unwrap() == &seed.to_string()));
            assert_eq!(batch[2].get_double(6)?, 4.0);
        }
        let before = std::fs::read(dir.path().join("scalar.parquet"))?;
        let mut conf = config(dir.path());
        conf.append = true;
        conf.save_config = false;
        assert!(save_outputs_parquet(vec![output(3)], conf).is_err());
        assert_eq!(std::fs::read(dir.path().join("scalar.parquet"))?, before);

        save_outputs_parquet(vec![output(4)], config(dir.path()))?;
        assert_eq!(rows(dir.path(), "scalar")?.len(), 1);
        assert_eq!(rows(dir.path(), "latency")?.len(), 3);
        assert_eq!(rows(dir.path(), "latency-stats")?.len(), 2);
        Ok(())
    }

    #[test]
    fn multiple_row_groups_and_empty_outputs() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        save_outputs_parquet(vec![], config(dir.path()))?;
        assert_eq!(std::fs::read_dir(dir.path())?.count(), 0);
        let mut output = output(0);
        output
            .series
            .init("latency", &[], MetricMetadata::default());
        for index in 0..BATCH_SIZE + 3 {
            output
                .series
                .add("latency", vec![], index as f64, index as f64);
        }
        save_outputs_parquet(vec![output], config(dir.path()))?;
        let reader = reader(dir.path(), "latency")?;
        assert_eq!(reader.num_row_groups(), 2);
        let values = rows(dir.path(), "latency")?;
        assert_eq!(values.len(), BATCH_SIZE + 3);
        for (index, row) in values.iter().enumerate() {
            assert_eq!(row.get_double(3)?, index as f64);
            assert_eq!(row.get_double(4)?, index as f64);
        }
        Ok(())
    }
}
