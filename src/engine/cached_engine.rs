use crate::dsl::QueryPlan;
use crate::engine::{salsa_db::*, JoinEngine};
use anyhow::Result;
use polars::lazy::frame::LazyFrame;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct CachedEngine {
    db: DatabaseImpl,
    source_cache: HashMap<PathBuf, SourceFile>,
}

impl CachedEngine {
    pub fn new() -> Self {
        Self {
            db: DatabaseImpl::default(),
            source_cache: HashMap::new(),
        }
    }

    pub fn execute_query_cached(&mut self, query: &QueryPlan) -> Result<LazyFrame> {
        // Convert sources to SourceFiles and register them with Salsa
        let mut source_files = Vec::new();
        
        for source in &query.sources {
            let source_file = self.get_or_create_source_file(&source.path)?;
            source_files.push(source_file);
        }

        // Serialize the query plan for caching
        let query_json = serde_json::to_string(query)?;
        
        // Execute through Salsa (this will use incremental computation)
        let _execution_result = execute_query_cached(&self.db, query_json, source_files);
        
        // For now, fall back to the regular engine for the actual execution
        // In a full implementation, we'd cache the LazyFrame itself
        JoinEngine::execute_query(query)
    }

    fn get_or_create_source_file(&mut self, path: &PathBuf) -> Result<SourceFile> {
        if let Some(&existing) = self.source_cache.get(path) {
            // Check if file has been modified
            let metadata = std::fs::metadata(path)?;
            let current_mtime = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            
            if existing.modified_time(&self.db) >= current_mtime {
                return Ok(existing);
            }
        }

        // File doesn't exist in cache or has been modified
        let content_hash = self.compute_content_hash(path)?;
        let metadata = std::fs::metadata(path)?;
        let modified_time = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        let source_file = SourceFile::new(
            &mut self.db,
            path.clone(),
            content_hash,
            modified_time,
        );

        self.source_cache.insert(path.clone(), source_file);
        Ok(source_file)
    }

    fn compute_content_hash(&self, path: &Path) -> Result<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::fs;

        let content = fs::read(path)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(hasher.finish())
    }

    pub fn invalidate_source(&mut self, path: &PathBuf) {
        self.source_cache.remove(path);
        invalidate_source_in_db(&mut self.db, path);
    }

    pub fn cache_stats(&self) -> HashMap<String, usize> {
        let mut stats = get_cache_stats(&self.db);
        stats.insert("source_files_cached".to_string(), self.source_cache.len());
        stats
    }

    pub fn clear_cache(&mut self) {
        self.source_cache.clear();
        // Reset the database - in practice, you might want more granular control
        self.db = DatabaseImpl::default();
    }

    /// Warm up the cache by pre-loading source metadata
    pub fn warmup_sources(&mut self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            let source_file = self.get_or_create_source_file(path)?;
            // Trigger metadata computation
            let _metadata = source_metadata(&self.db, source_file);
        }
        Ok(())
    }

    /// Get detailed information about a source file from cache
    pub fn get_source_info(&mut self, path: &PathBuf) -> Result<SourceInfo> {
        let source_file = self.get_or_create_source_file(path)?;
        let metadata_str = source_metadata(&self.db, source_file);
        let parsed_id = parsed_source_id(&self.db, source_file);

        Ok(SourceInfo {
            path: path.clone(),
            content_hash: source_file.content_hash(&self.db),
            modified_time: source_file.modified_time(&self.db),
            schema: metadata_str,
            row_count: 0, // TODO: Extract from metadata string
            file_size: std::fs::metadata(path)?.len(),
            dataframe_id: parsed_id,
        })
    }
}

impl Default for CachedEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct SourceInfo {
    pub path: PathBuf,
    pub content_hash: u64,
    pub modified_time: SystemTime,
    pub schema: String,
    pub row_count: usize,
    pub file_size: u64,
    pub dataframe_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_cached_engine_creation() {
        let engine = CachedEngine::new();
        assert_eq!(engine.source_cache.len(), 0);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut engine = CachedEngine::new();
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.csv");
        
        // Create a test file
        fs::write(&file_path, "id,name\n1,test").unwrap();
        
        // Load it into cache
        let _source_file = engine.get_or_create_source_file(&file_path).unwrap();
        assert_eq!(engine.source_cache.len(), 1);
        
        // Invalidate cache
        engine.invalidate_source(&file_path);
        assert_eq!(engine.source_cache.len(), 0);
    }
}