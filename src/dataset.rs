use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use csv::StringRecord;

#[derive(Debug, Clone)]
pub struct DatasetSpec {
    pub name: String,
    pub csv_path: PathBuf,
    pub image_dir: PathBuf,
}

impl DatasetSpec {
    pub fn new(name: String, root: &Path) -> Self {
        Self {
            csv_path: root.join(format!("{name}.csv")),
            image_dir: root.join(&name),
            name,
        }
    }
}

#[derive(Debug)]
pub struct DatasetState {
    pub spec: DatasetSpec,
    pub rows: Vec<CaseRecord>,
    pub current_index: usize,
    pub dirty: bool,
}

impl DatasetState {
    pub fn load(spec: &DatasetSpec, finding_column: &str) -> Result<Self> {
        let mut reader = csv::Reader::from_path(&spec.csv_path)
            .with_context(|| format!("Read CSV {}", spec.csv_path.display()))?;

        let headers = reader.headers()?.clone();
        let png_idx = column_index(&headers, "png_file", &spec.csv_path)?;
        let acc_idx = column_index(&headers, "acc", &spec.csv_path)?;
        let finding_idx = column_index(&headers, finding_column, &spec.csv_path)?;
        let confirm_idx = column_index(&headers, "confirm", &spec.csv_path)?;

        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record?;
            rows.push(CaseRecord::from_record(
                &record,
                png_idx,
                acc_idx,
                finding_idx,
                confirm_idx,
            ));
        }

        Ok(Self {
            spec: spec.clone(),
            rows,
            current_index: 0,
            dirty: false,
        })
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn current_case(&self) -> Option<&CaseRecord> {
        self.rows.get(self.current_index)
    }

    pub fn current_case_mut(&mut self) -> Option<&mut CaseRecord> {
        self.rows.get_mut(self.current_index)
    }

    pub fn resume_index(&self) -> usize {
        self.rows
            .iter()
            .position(|case| !case.confirm)
            .unwrap_or_else(|| self.rows.len().saturating_sub(1))
    }

    pub fn csv_file_name(&self) -> String {
        self.spec
            .csv_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.spec.name)
            .to_string()
    }

    pub fn image_path(&self, file: &str) -> PathBuf {
        self.spec.image_dir.join(file)
    }

    pub fn save(&mut self, finding_column: &str) -> Result<()> {
        let mut writer = csv::Writer::from_path(&self.spec.csv_path)
            .with_context(|| format!("Write CSV {}", self.spec.csv_path.display()))?;

        writer.write_record(["png_file", "acc", finding_column, "confirm"])?;
        for row in &mut self.rows {
            writer.write_record(row.to_record())?;
            row.dirty = false;
        }
        writer.flush()?;
        self.dirty = false;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CaseRecord {
    pub png_file: String,
    pub accession: String,
    pub finding: bool,
    pub confirm: bool,
    pub dirty: bool,
}

impl CaseRecord {
    pub fn from_record(
        record: &StringRecord,
        png_idx: usize,
        acc_idx: usize,
        finding_idx: usize,
        confirm_idx: usize,
    ) -> Self {
        Self {
            png_file: record.get(png_idx).unwrap_or_default().to_string(),
            accession: record.get(acc_idx).unwrap_or_default().to_string(),
            finding: parse_csv_bool(record.get(finding_idx)),
            confirm: parse_csv_bool(record.get(confirm_idx)),
            dirty: false,
        }
    }

    pub fn to_record(&self) -> [String; 4] {
        [
            self.png_file.clone(),
            self.accession.clone(),
            bool_to_csv_value(self.finding),
            bool_to_csv_value(self.confirm),
        ]
    }
}

pub fn collect_dataset_specs(root: &Path) -> Result<Vec<DatasetSpec>> {
    let mut stems = HashSet::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("csv") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    stems.insert(stem.to_string());
                }
            }
        }
    }

    let mut specs = Vec::new();
    for stem in stems {
        let dir_path = root.join(&stem);
        if dir_path.is_dir() {
            specs.push(DatasetSpec::new(stem, root));
        }
    }

    specs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(specs)
}

fn column_index(headers: &StringRecord, name: &str, csv_path: &Path) -> Result<usize> {
    headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            anyhow!(
                "Column '{name}' not found in {file}",
                file = csv_path.display()
            )
        })
}

fn parse_csv_bool(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim()),
        Some("1") | Some("true") | Some("True") | Some("TRUE")
    )
}

fn bool_to_csv_value(value: bool) -> String {
    if value {
        "1".to_string()
    } else {
        "0".to_string()
    }
}
