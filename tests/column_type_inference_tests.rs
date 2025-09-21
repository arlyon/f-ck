use f_ck::engine::*;
use insta::assert_yaml_snapshot;
use tempfile::NamedTempFile;
use std::io::Write as IoWrite;
use std::path::PathBuf;

fn create_test_csv(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

#[test]
fn test_basic_type_inference() {
    let csv_content = r#"id,name,age,email,active
1,Alice,25,alice@example.com,true
2,Bob,30,bob@example.com,false
3,Charlie,28,charlie@example.com,true"#;
    
    let _csv_file = create_test_csv(csv_content);
    let source = f_ck::dsl::Source {
        id: "test".to_string(),
        path: _csv_file.path().to_path_buf(),
        format: "csv".to_string(),
    };
    
    let schema = DataReader::get_schema(&source).unwrap();
    
    assert_yaml_snapshot!("basic_type_inference", schema.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect::<Vec<_>>());
}

#[test]
fn test_mixed_type_column_detection() {
    let csv_content = r#"id,contact_info,amount
1,alice@example.com,100.50
2,+1-555-0123,200
3,bob@company.org,300.75
4,555-0199,invalid
5,charlie@test.com,450"#;
    
    let _csv_file = create_test_csv(csv_content);
    let source = f_ck::dsl::Source {
        id: "mixed_types".to_string(),
        path: _csv_file.path().to_path_buf(),
        format: "csv".to_string(),
    };
    
    let schema = DataReader::get_schema(&source).unwrap();
    
    // The contact_info column should be detected as mixed types (email/phone)
    // The amount column should handle mixed numeric/text
    assert_yaml_snapshot!("mixed_type_detection", schema.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect::<Vec<_>>());
}

#[test] 
fn test_partial_scanning_optimization() {
    // Create a large CSV where we only need to scan a portion to determine types
    let mut csv_content = String::from("id,category,value\n");
    
    // Add consistent data for first 100 rows
    for i in 1..=100 {
        csv_content.push_str(&format!("{},product,{}.99\n", i, i * 10));
    }
    
    // Add 10000 more rows of the same pattern
    for i in 101..=10100 {
        csv_content.push_str(&format!("{},product,{}.99\n", i, i * 10));
    }
    
    let _csv_file = create_test_csv(&csv_content);
    let source = f_ck::dsl::Source {
        id: "large_consistent".to_string(),
        path: _csv_file.path().to_path_buf(),
        format: "csv".to_string(),
    };
    
    // Should be able to determine types without scanning all 10k rows
    let schema = DataReader::get_schema(&source).unwrap();
    
    assert_yaml_snapshot!("partial_scan_types", schema.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect::<Vec<_>>());
}

#[test]
fn test_type_range_tracking() {
    let csv_content = r#"id,data_type,value
1,email,alice@example.com
2,email,bob@example.com
3,phone,555-0123
4,phone,555-0199
5,email,charlie@example.com
6,mixed,not_email_or_phone
7,phone,+1-555-0187"#;
    
    let _csv_file = create_test_csv(csv_content);
    let source = f_ck::dsl::Source {
        id: "type_ranges".to_string(),
        path: _csv_file.path().to_path_buf(),
        format: "csv".to_string(),
    };
    
    let schema = DataReader::get_schema(&source).unwrap();
    
    // Should track that rows 1-2,5 are emails, rows 3-4,7 are phones, row 6 is text
    assert_yaml_snapshot!("type_range_tracking", schema.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect::<Vec<_>>());
}

#[test]
fn test_date_type_inference() {
    let csv_content = r#"id,created_at,birthday,timestamp
1,2023-01-15,1990-05-20,2023-01-15T10:30:00Z
2,2023-02-20,1985-12-03,2023-02-20T14:45:30Z
3,2023-03-10,1992-07-18,2023-03-10T09:15:45Z"#;
    
    let _csv_file = create_test_csv(csv_content);
    let source = f_ck::dsl::Source {
        id: "dates".to_string(),
        path: _csv_file.path().to_path_buf(),
        format: "csv".to_string(),
    };
    
    let schema = DataReader::get_schema(&source).unwrap();
    
    assert_yaml_snapshot!("date_type_inference", schema.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect::<Vec<_>>());
}

#[test]
fn test_numeric_type_inference() {
    let csv_content = r#"id,small_int,big_int,decimal,percentage
1,5,1000000,123.456,0.85
2,10,2000000,789.012,0.92
3,-3,3000000,-456.789,1.05"#;
    
    let _csv_file = create_test_csv(csv_content);
    let source = f_ck::dsl::Source {
        id: "numbers".to_string(),
        path: _csv_file.path().to_path_buf(),
        format: "csv".to_string(),
    };
    
    let schema = DataReader::get_schema(&source).unwrap();
    
    assert_yaml_snapshot!("numeric_type_inference", schema.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect::<Vec<_>>());
}

#[test]
fn test_empty_and_null_handling() {
    let csv_content = r#"id,optional_email,optional_number,mixed_nulls
1,alice@example.com,100,value1
2,,200,
3,bob@example.com,,value3
4,,,
5,charlie@example.com,300,value5"#;
    
    let _csv_file = create_test_csv(csv_content);
    let source = f_ck::dsl::Source {
        id: "nulls".to_string(),
        path: _csv_file.path().to_path_buf(),
        format: "csv".to_string(),
    };
    
    let schema = DataReader::get_schema(&source).unwrap();
    
    assert_yaml_snapshot!("null_handling", schema.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect::<Vec<_>>());
}

#[test]
fn test_country_and_postal_code_detection() {
    let csv_content = r#"id,country,postal_code,region
1,US,90210,CA
2,UK,SW1A 1AA,London
3,CA,M5V 3L9,ON
4,DE,10115,Berlin
5,FR,75001,Paris"#;
    
    let _csv_file = create_test_csv(csv_content);
    let source = f_ck::dsl::Source {
        id: "geography".to_string(),
        path: _csv_file.path().to_path_buf(),
        format: "csv".to_string(),
    };
    
    let schema = DataReader::get_schema(&source).unwrap();
    
    assert_yaml_snapshot!("geography_type_detection", schema.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect::<Vec<_>>());
}

#[test]
fn test_name_detection() {
    let csv_content = r#"id,first_name,last_name,full_name,username
1,Alice,Johnson,Alice Johnson,alice_j
2,Bob,Smith,Bob Smith,bob_s
3,Charlie,Brown,Charlie Brown,charlie_b"#;
    
    let _csv_file = create_test_csv(csv_content);
    let source = f_ck::dsl::Source {
        id: "names".to_string(),
        path: _csv_file.path().to_path_buf(),
        format: "csv".to_string(),
    };
    
    let schema = DataReader::get_schema(&source).unwrap();
    
    assert_yaml_snapshot!("name_type_detection", schema.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect::<Vec<_>>());
}