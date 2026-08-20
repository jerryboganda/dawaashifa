//! Data Migration Toolkit for importing product masters, customers, and historical orders into Shifa.

pub mod adapters;
pub mod aliases;
pub mod engine;
pub mod error;
pub mod mapping;
pub mod transforms;

pub use adapters::{
    CsvSourceAdapter, JsonSourceAdapter, MemorySourceAdapter, RawRecord, SourceAdapter,
    SourceSchema,
};
pub use aliases::AliasGenerator;
pub use engine::{MigrationEngine, MigrationReport};
pub use error::MigrationError;
pub use mapping::MappingConfig;
pub use transforms::TransformEngine;
