use crate::dsl::Source;
use salsa::Database;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

// Use the salsa::DatabaseImpl directly
pub type DatabaseImpl = salsa::DatabaseImpl;

// Salsa input representing a source file
#[salsa::input]
pub struct SourceFile {
    pub path: PathBuf,
    pub content_hash: u64,
    pub modified_time: SystemTime,
}

// Simple tracked function that returns metadata as a string
#[salsa::tracked]
pub fn source_metadata(db: &dyn salsa::Database, source_file: SourceFile) -> String {
    let path = source_file.path(db);
    
    // Create a temporary Source for compatibility with existing DataReader
    let temp_source = Source {
        id: "temp".to_string(),
        path: path.clone(),
        format: if path.extension().and_then(|s| s.to_str()) == Some("csv") {
            "csv".to_string()
        } else {
            "tsv".to_string()
        },
    };

    match crate::engine::DataReader::get_schema(&temp_source) {
        Ok(schema) => {
            // Convert schema to a simple string representation since Schema doesn't implement Serialize
            let schema_map: std::collections::HashMap<String, String> = schema
                .iter()
                .map(|(name, dtype)| (name.to_string(), format!("{:?}", dtype)))
                .collect();
            serde_json::to_string(&schema_map).unwrap_or_default()
        },
        Err(_) => "error".to_string(),
    }
}

// Tracked function for parsing data source  
#[salsa::tracked]
pub fn parsed_source_id(db: &dyn salsa::Database, source_file: SourceFile) -> String {
    // Trigger metadata computation first (dependency)
    let _metadata = source_metadata(db, source_file);
    
    // Generate a unique ID for this parsed source
    format!("df_{}", source_file.content_hash(db))
}

// Tracked function for query execution
#[salsa::tracked] 
pub fn execute_query_cached(
    db: &dyn salsa::Database,
    query_json: String,
    source_files: Vec<SourceFile>,
) -> String {
    // Ensure all sources are processed (creates dependencies)
    for source_file in &source_files {
        let _parsed_id = parsed_source_id(db, *source_file);
        let _metadata = source_metadata(db, *source_file);
    }
    
    // Generate a result ID based on query and inputs
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    query_json.hash(&mut hasher);
    for sf in &source_files {
        sf.content_hash(db).hash(&mut hasher);
    }
    
    format!("result_{}", hasher.finish())
}

// Helper functions for cache management
pub fn invalidate_source_in_db(db: &mut DatabaseImpl, _path: &PathBuf) {
    // In a real implementation, we'd track and invalidate specific SourceFile inputs
    // For now, we'll use a simple approach
    db.synthetic_write(salsa::Durability::LOW);
}

pub fn get_cache_stats(_db: &DatabaseImpl) -> HashMap<String, usize> {
    // Return basic cache statistics
    // In a real implementation, this would use Salsa's introspection APIs
    let mut stats = HashMap::new();
    stats.insert("total_inputs".to_string(), 0);
    stats.insert("computed_values".to_string(), 0);
    stats
}