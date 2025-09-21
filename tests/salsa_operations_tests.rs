use f_ck::dsl::*;
use f_ck::engine::salsa_db::*;
use insta::assert_yaml_snapshot;
use std::collections::HashMap;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn create_test_csv(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

#[test]
fn test_salsa_source_file_tracking() {
    let mut db = DatabaseImpl::default();

    let _csv_file =
        create_test_csv("id,name,email\n1,Alice,alice@example.com\n2,Bob,bob@example.com");
    let path = _csv_file.path().to_path_buf();

    let source_file = SourceFile::new(
        &mut db,
        path.clone(),
        12345, // content_hash
        std::time::SystemTime::now(),
    );

    let metadata = source_metadata(&db, source_file);

    // Should return schema information
    assert!(!metadata.is_empty());
    assert_yaml_snapshot!("source_metadata", metadata);
}

#[test]
fn test_salsa_parsed_source_caching() {
    let mut db = DatabaseImpl::default();

    let _csv_file = create_test_csv("id,name\n1,Alice\n2,Bob");
    let path = _csv_file.path().to_path_buf();

    let source_file = SourceFile::new(&mut db, path.clone(), 67890, std::time::SystemTime::now());

    // First call should compute
    let parsed_id1 = parsed_source_id(&db, source_file);

    // Second call should use cached result
    let parsed_id2 = parsed_source_id(&db, source_file);

    assert_eq!(parsed_id1, parsed_id2);
    assert!(parsed_id1.starts_with("df_"));

    assert_yaml_snapshot!("parsed_source_id", parsed_id1);
}

#[test]
fn test_salsa_query_execution_caching() {
    let mut db = DatabaseImpl::default();

    let _csv_file = create_test_csv("id,value\n1,100\n2,200");
    let path = _csv_file.path().to_path_buf();

    let source_file = SourceFile::new(&mut db, path, 11111, std::time::SystemTime::now());

    let query_json = r#"{
        "sources": {
            "data": {
                "path": "/tmp/test.csv",
                "format": "csv"
            }
        },
        "steps": [
            {
                "id": "load_data",
                "op": "load",
                "source": "data"
            }
        ]
    }"#
    .to_string();

    // First execution
    let result1 = execute_query_cached(&db, query_json.clone(), vec![source_file]);

    // Second execution should be cached
    let result2 = execute_query_cached(&db, query_json, vec![source_file]);

    assert_eq!(result1, result2);
    assert!(result1.starts_with("result_"));

    assert_yaml_snapshot!("query_execution_result", result1);
}

#[test]
fn test_salsa_incremental_invalidation() {
    let mut db = DatabaseImpl::default();

    let _csv_file1 = create_test_csv("id,value\n1,100\n2,200");
    let path = _csv_file1.path().to_path_buf();

    let source_file1 = SourceFile::new(&mut db, path.clone(), 11111, std::time::SystemTime::now());

    let query_json = r#"{
        "sources": {"data": {"path": "/tmp/test.csv", "format": "csv"}},
        "steps": [{"id": "load_data", "op": "load", "source": "data"}]
    }"#
    .to_string();

    let result1 = execute_query_cached(&db, query_json.clone(), vec![source_file1]);

    // Create new source file with different content hash (simulating file change)
    let source_file2 = SourceFile::new(
        &mut db,
        path.clone(),
        22222, // Different hash
        std::time::SystemTime::now(),
    );

    let result2 = execute_query_cached(&db, query_json, vec![source_file2]);

    // Results should be different due to changed input
    assert_ne!(result1, result2);

    assert_yaml_snapshot!("incremental_results", (result1, result2));
}

#[test]
fn test_salsa_multiple_source_dependencies() {
    let mut db = DatabaseImpl::default();

    let _csv1 = create_test_csv("id,name\n1,Alice\n2,Bob");
    let _csv2 = create_test_csv("id,email\n1,alice@example.com\n2,bob@example.com");

    let path1 = _csv1.path().to_path_buf();
    let path2 = _csv2.path().to_path_buf();

    let source1 = SourceFile::new(&mut db, path1, 33333, std::time::SystemTime::now());
    let source2 = SourceFile::new(&mut db, path2, 44444, std::time::SystemTime::now());

    let query_json = r#"{
        "sources": {
            "names": {"path": "/tmp/names.csv", "format": "csv"},
            "emails": {"path": "/tmp/emails.csv", "format": "csv"}
        },
        "steps": [
            {"id": "load_names", "op": "load", "source": "names"},
            {"id": "load_emails", "op": "load", "source": "emails"},
            {"id": "link_data", "op": "link", "left": "load_names", "leftOn": "id", "right": "load_emails", "rightOn": "id"}
        ]
    }"#.to_string();

    let result = execute_query_cached(&db, query_json, vec![source1, source2]);

    assert!(result.starts_with("result_"));
    assert_yaml_snapshot!("multi_source_result", result);
}

#[test]
fn test_source_invalidation() {
    let mut db = DatabaseImpl::default();
    let path = PathBuf::from("/tmp/test.csv");

    // This should trigger a cache invalidation
    invalidate_source_in_db(&mut db, &path);

    // The operation should complete without error
    // In a real implementation, this would clear specific cached values
}
