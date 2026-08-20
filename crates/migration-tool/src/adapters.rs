use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::MigrationError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSchema {
    pub columns: Vec<String>,
    pub sample_rows: Vec<HashMap<String, String>>,
    pub estimated_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRecord {
    pub row_no: u64,
    pub fields: HashMap<String, String>,
}

#[async_trait]
pub trait SourceAdapter: Send + Sync {
    fn kind(&self) -> String;
    async fn probe(&self) -> Result<SourceSchema, MigrationError>;
    async fn read_records(&self) -> Result<Vec<RawRecord>, MigrationError>;
    async fn count(&self) -> Result<u64, MigrationError>;
}

// ------------------------------------------------------------------------------------------------
// CSV Source Adapter
// ------------------------------------------------------------------------------------------------
pub struct CsvSourceAdapter {
    pub csv_content: String,
}

impl CsvSourceAdapter {
    pub fn new(csv_content: String) -> Self {
        Self { csv_content }
    }
}

#[async_trait]
impl SourceAdapter for CsvSourceAdapter {
    fn kind(&self) -> String {
        "csv".into()
    }

    async fn probe(&self) -> Result<SourceSchema, MigrationError> {
        let mut rdr = csv::Reader::from_reader(self.csv_content.as_bytes());
        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| MigrationError::Source(e.to_string()))?
            .iter()
            .map(|s| s.trim().to_string())
            .collect();

        let mut samples = Vec::new();
        let mut count = 0u64;

        for result in rdr.records() {
            let record = result.map_err(|e| MigrationError::Source(e.to_string()))?;
            count += 1;
            if samples.len() < 5 {
                let mut map = HashMap::new();
                for (i, h) in headers.iter().enumerate() {
                    if let Some(val) = record.get(i) {
                        map.insert(h.clone(), val.to_string());
                    }
                }
                samples.push(map);
            }
        }

        Ok(SourceSchema {
            columns: headers,
            sample_rows: samples,
            estimated_count: count,
        })
    }

    async fn read_records(&self) -> Result<Vec<RawRecord>, MigrationError> {
        let mut rdr = csv::Reader::from_reader(self.csv_content.as_bytes());
        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| MigrationError::Source(e.to_string()))?
            .iter()
            .map(|s| s.trim().to_string())
            .collect();

        let mut records = Vec::new();

        for (row_idx, result) in rdr.records().enumerate() {
            let record = result.map_err(|e| MigrationError::Source(e.to_string()))?;
            let row_no = (row_idx + 1) as u64;
            let mut map = HashMap::new();
            for (i, h) in headers.iter().enumerate() {
                if let Some(val) = record.get(i) {
                    map.insert(h.clone(), val.to_string());
                }
            }
            records.push(RawRecord {
                row_no,
                fields: map,
            });
        }

        Ok(records)
    }

    async fn count(&self) -> Result<u64, MigrationError> {
        let schema = self.probe().await?;
        Ok(schema.estimated_count)
    }
}

// ------------------------------------------------------------------------------------------------
// JSON Source Adapter
// ------------------------------------------------------------------------------------------------
pub struct JsonSourceAdapter {
    pub json_content: String,
}

impl JsonSourceAdapter {
    pub fn new(json_content: String) -> Self {
        Self { json_content }
    }
}

#[async_trait]
impl SourceAdapter for JsonSourceAdapter {
    fn kind(&self) -> String {
        "json".into()
    }

    async fn probe(&self) -> Result<SourceSchema, MigrationError> {
        let parsed: serde_json::Value = serde_json::from_str(&self.json_content)
            .map_err(|e| MigrationError::Source(e.to_string()))?;

        let arr = parsed.as_array().ok_or_else(|| {
            MigrationError::Source("JSON source must be an array of objects".into())
        })?;

        let mut columns = Vec::new();
        let mut samples = Vec::new();

        if let Some(first) = arr.first() {
            if let Some(obj) = first.as_object() {
                columns = obj.keys().cloned().collect();
            }
        }

        for (i, val) in arr.iter().enumerate() {
            if i < 5 {
                if let Some(obj) = val.as_object() {
                    let mut map = HashMap::new();
                    for (k, v) in obj {
                        map.insert(k.clone(), v.as_str().unwrap_or(&v.to_string()).to_string());
                    }
                    samples.push(map);
                }
            }
        }

        Ok(SourceSchema {
            columns,
            sample_rows: samples,
            estimated_count: arr.len() as u64,
        })
    }

    async fn read_records(&self) -> Result<Vec<RawRecord>, MigrationError> {
        let parsed: serde_json::Value = serde_json::from_str(&self.json_content)
            .map_err(|e| MigrationError::Source(e.to_string()))?;

        let arr = parsed.as_array().ok_or_else(|| {
            MigrationError::Source("JSON source must be an array of objects".into())
        })?;

        let mut records = Vec::new();
        for (i, val) in arr.iter().enumerate() {
            if let Some(obj) = val.as_object() {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    let s = match v {
                        serde_json::Value::String(st) => st.clone(),
                        other => other.to_string(),
                    };
                    map.insert(k.clone(), s);
                }
                records.push(RawRecord {
                    row_no: (i + 1) as u64,
                    fields: map,
                });
            }
        }

        Ok(records)
    }

    async fn count(&self) -> Result<u64, MigrationError> {
        let schema = self.probe().await?;
        Ok(schema.estimated_count)
    }
}

// ------------------------------------------------------------------------------------------------
// Generic Memory Source Adapter
// ------------------------------------------------------------------------------------------------
pub struct MemorySourceAdapter {
    pub kind_name: String,
    pub records: Vec<HashMap<String, String>>,
}

impl MemorySourceAdapter {
    pub fn new(kind_name: &str, records: Vec<HashMap<String, String>>) -> Self {
        Self {
            kind_name: kind_name.to_string(),
            records,
        }
    }
}

#[async_trait]
impl SourceAdapter for MemorySourceAdapter {
    fn kind(&self) -> String {
        self.kind_name.clone()
    }

    async fn probe(&self) -> Result<SourceSchema, MigrationError> {
        let mut columns = Vec::new();
        if let Some(first) = self.records.first() {
            columns = first.keys().cloned().collect();
        }
        let samples = self.records.iter().take(5).cloned().collect();

        Ok(SourceSchema {
            columns,
            sample_rows: samples,
            estimated_count: self.records.len() as u64,
        })
    }

    async fn read_records(&self) -> Result<Vec<RawRecord>, MigrationError> {
        let mut list = Vec::new();
        for (i, map) in self.records.iter().enumerate() {
            list.push(RawRecord {
                row_no: (i + 1) as u64,
                fields: map.clone(),
            });
        }
        Ok(list)
    }

    async fn count(&self) -> Result<u64, MigrationError> {
        Ok(self.records.len() as u64)
    }
}
