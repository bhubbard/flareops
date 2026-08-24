pub mod analyzer;
pub mod generator;
pub mod syncer;

pub use analyzer::{TsFileAnalysis, analyze_env_dts};
pub use generator::{
    END_MARKER, GeneratorOptions, START_MARKER, SyncMode, generate_complete_env_dts,
    generate_managed_block,
};
pub use syncer::{SyncResult, sync_env_file};
