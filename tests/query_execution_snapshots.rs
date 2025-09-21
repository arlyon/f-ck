use f_ck::dsl::QueryPlan;
use f_ck::{MergePolicy, engine::*};
use insta::assert_yaml_snapshot;
use serde_json;
use std::path::PathBuf;

#[track_caller]
fn load_query_from_file(path: &str) -> QueryPlan {
    let content = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&content).unwrap()
}

#[test]
fn test_basic_query_execution() {
    let query = load_query_from_file("test_data/test_query.json");

    // Test the query structure
    assert_yaml_snapshot!("basic_query_structure", query);

    // Test source loading and schema inference
    for source in &query.sources {
        let schema = DataReader::get_schema(source).unwrap();
        let schema_snapshot = schema
            .iter()
            .map(|(k, v)| (k.clone(), format!("{:?}", v)))
            .collect::<Vec<_>>();
        assert_yaml_snapshot!(format!("schema_{}", source.id), schema_snapshot);
    }
}

#[test]
fn test_query_execution_with_small_data() {
    // Create a modified query using the small data files
    let mut query = load_query_from_file("test_data/test_query.json");

    // Update the mapping to match the small data column names
    // customer_id mapping - update source fields
    for mapping in &mut query.mappings {
        if mapping.destination_field == "customer_id" {
            // Update customers source field to match small_customers.csv
            for source_field in &mut mapping.source_fields {
                if source_field.source_file_id == "customers" {
                    source_field.column_name = "customer_id".to_string();
                }
            }
        }
        if mapping.destination_field == "customer_name" {
            // Update to use first_name from small_customers.csv
            for source_field in &mut mapping.source_fields {
                if source_field.source_file_id == "customers" {
                    source_field.column_name = "first_name".to_string();
                }
            }
        }
        if mapping.destination_field == "total_spent" {
            // Update to use 'total' from small_orders.csv
            for source_field in &mut mapping.source_fields {
                if source_field.source_file_id == "orders" {
                    source_field.column_name = "total".to_string();
                }
            }
        }
    }

    assert_yaml_snapshot!("small_data_query_structure", query);

    // Test individual source schemas
    for source in &query.sources {
        let schema = DataReader::get_schema(source).unwrap();
        let schema_snapshot = schema
            .iter()
            .map(|(k, v)| (k.clone(), format!("{:?}", v)))
            .collect::<Vec<_>>();
        assert_yaml_snapshot!(format!("small_data_schema_{}", source.id), schema_snapshot);
    }
}

#[test]
fn test_query_validation() {
    let query = load_query_from_file("test_data/test_query.json");

    // Test source file existence validation
    let mut validation_results = Vec::new();

    for source in &query.sources {
        let exists = source.path.exists();
        validation_results.push((
            source.id.clone(),
            exists,
            format!("{}", source.path.display()),
        ));
    }

    assert_yaml_snapshot!("source_validation_results", validation_results);
}

#[test]
fn test_mapping_policies() {
    let query = load_query_from_file("test_data/test_query.json");

    // Extract and test different mapping policies
    let policies: Vec<_> = query
        .mappings
        .iter()
        .map(|m| {
            (
                m.destination_field.clone(),
                format!("{:?}", m.policy),
                m.source_fields.len(),
            )
        })
        .collect();

    assert_yaml_snapshot!("mapping_policies", policies);
}

#[test]
fn test_destination_schema_structure() {
    let query = load_query_from_file("test_data/test_query.json");

    // Test the destination schema
    let dest_schema: Vec<_> = query
        .destination_schema
        .iter()
        .map(|field| (field.name.clone(), field.data_type.clone()))
        .collect();

    assert_yaml_snapshot!("destination_schema", dest_schema);
}

#[test]
fn test_primary_key_configuration() {
    let query = load_query_from_file("test_data/test_query.json");

    // Test primary key setup
    let pk_config = (
        query.primary_keys.logic.clone(),
        query.primary_keys.keys.clone(),
    );

    assert_yaml_snapshot!("primary_key_config", pk_config);
}

#[test]
fn test_complex_aggregation_mapping() {
    let query = load_query_from_file("test_data/test_query.json");

    // Focus on the sum aggregation mapping
    let sum_mapping = query
        .mappings
        .iter()
        .find(|m| matches!(m.policy, MergePolicy::Sum))
        .unwrap();

    let aggregation_details = (
        sum_mapping.destination_field.clone(),
        format!("{:?}", sum_mapping.policy),
        sum_mapping
            .source_fields
            .iter()
            .map(|sf| (sf.source_file_id.clone(), sf.column_name.clone()))
            .collect::<Vec<_>>(),
    );

    assert_yaml_snapshot!("sum_aggregation_mapping", aggregation_details);
}

#[test]
fn test_first_match_priority_mapping() {
    let query = load_query_from_file("test_data/test_query.json");

    // Focus on first match mappings with priorities
    let first_match_mappings: Vec<_> = query
        .mappings
        .iter()
        .filter_map(|m| {
            if let MergePolicy::FirstMatch { priority } = &m.policy {
                Some((
                    m.destination_field.clone(),
                    priority.clone(),
                    m.source_fields
                        .iter()
                        .map(|sf| (sf.source_file_id.clone(), sf.column_name.clone()))
                        .collect::<Vec<_>>(),
                ))
            } else {
                None
            }
        })
        .collect();

    assert_yaml_snapshot!("first_match_mappings", first_match_mappings);
}

#[test]
fn test_source_field_coverage() {
    let query = load_query_from_file("test_data/test_query.json");

    // Analyze which source columns are used in mappings
    let mut source_coverage = std::collections::HashMap::new();

    for mapping in &query.mappings {
        for source_field in &mapping.source_fields {
            let key = format!(
                "{}:{}",
                source_field.source_file_id, source_field.column_name
            );
            source_coverage
                .entry(key)
                .or_insert(Vec::new())
                .push(mapping.destination_field.clone());
        }
    }

    let mut coverage_report: Vec<_> = source_coverage.into_iter().collect();
    coverage_report.sort_by(|a, b| a.0.cmp(&b.0));
    assert_yaml_snapshot!("source_field_coverage", coverage_report);
}

#[test]
fn test_error_handling_missing_file() {
    let mut query = load_query_from_file("test_data/test_query.json");

    // Change path to non-existent file
    query.sources[0].path = PathBuf::from("test_data/nonexistent.csv");

    // Test validation failure
    let validation_results: Vec<_> = query
        .sources
        .iter()
        .map(|source| {
            let exists = source.path.exists();
            let schema_result = if exists {
                DataReader::get_schema(source)
                    .map(|_| "success".to_string())
                    .unwrap_or_else(|e| format!("error: {}", e))
            } else {
                "file_not_found".to_string()
            };
            (source.id.clone(), exists, schema_result)
        })
        .collect();

    assert_yaml_snapshot!("error_missing_file_validation", validation_results);
}

#[test]
fn test_query_with_all_merge_policies() {
    // Create a comprehensive query demonstrating all merge policy types
    let query_json = r#"{
  "sources": [
    {
      "id": "source1",
      "path": "test_data/small_customers.csv",
      "format": "csv"
    },
    {
      "id": "source2",
      "path": "test_data/small_orders.csv",
      "format": "csv"
    }
  ],
  "destination_schema": [
    {
      "name": "id",
      "data_type": "Int64"
    },
    {
      "name": "name",
      "data_type": "String"
    },
    {
      "name": "total_value",
      "data_type": "Float64"
    },
    {
      "name": "order_count",
      "data_type": "Int64"
    },
    {
      "name": "avg_order",
      "data_type": "Float64"
    },
    {
      "name": "min_order",
      "data_type": "Float64"
    },
    {
      "name": "max_order",
      "data_type": "Float64"
    }
  ],
  "primary_keys": {
    "logic": "and",
    "keys": ["id"]
  },
  "mappings": [
    {
      "destination_field": "id",
      "policy": {
        "type": "firstMatch",
        "priority": ["source1", "source2"]
      },
      "source_fields": [
        {
          "id": "customer_id_1",
          "source_file_id": "source1",
          "column_name": "customer_id"
        },
        {
          "id": "customer_id_2",
          "source_file_id": "source2",
          "column_name": "customer_id"
        }
      ]
    },
    {
      "destination_field": "name",
      "policy": {
        "type": "firstMatch",
        "priority": ["source1"]
      },
      "source_fields": [
        {
          "id": "first_name",
          "source_file_id": "source1",
          "column_name": "first_name"
        }
      ]
    },
    {
      "destination_field": "total_value",
      "policy": {
        "type": "sum"
      },
      "source_fields": [
        {
          "id": "order_total",
          "source_file_id": "source2",
          "column_name": "total"
        }
      ]
    },
    {
      "destination_field": "order_count",
      "policy": {
        "type": "count"
      },
      "source_fields": [
        {
          "id": "order_id",
          "source_file_id": "source2",
          "column_name": "order_id"
        }
      ]
    },
    {
      "destination_field": "avg_order",
      "policy": {
        "type": "average"
      },
      "source_fields": [
        {
          "id": "order_avg",
          "source_file_id": "source2",
          "column_name": "total"
        }
      ]
    },
    {
      "destination_field": "min_order",
      "policy": {
        "type": "min"
      },
      "source_fields": [
        {
          "id": "order_min",
          "source_file_id": "source2",
          "column_name": "total"
        }
      ]
    },
    {
      "destination_field": "max_order",
      "policy": {
        "type": "max"
      },
      "source_fields": [
        {
          "id": "order_max",
          "source_file_id": "source2",
          "column_name": "total"
        }
      ]
    }
  ]
}"#;

    let query: QueryPlan = serde_json::from_str(query_json).unwrap();

    // Test the comprehensive query structure
    assert_yaml_snapshot!("comprehensive_merge_policies_query", query);

    // Test all merge policy types
    let policy_types: Vec<_> = query
        .mappings
        .iter()
        .map(|m| (m.destination_field.clone(), format!("{:?}", m.policy)))
        .collect();

    assert_yaml_snapshot!("all_merge_policy_types", policy_types);
}

#[test]
fn test_complex_primary_key_configurations() {
    // Test different primary key logic configurations
    let configs = vec![
        ("and_logic", r#"{"logic": "and", "keys": ["id", "email"]}"#),
        (
            "or_logic",
            r#"{"logic": "or", "keys": ["id", "phone", "email"]}"#,
        ),
        ("single_key", r#"{"logic": "and", "keys": ["customer_id"]}"#),
        (
            "multiple_keys",
            r#"{"logic": "and", "keys": ["first_name", "last_name", "birth_date"]}"#,
        ),
    ];

    let pk_configurations: Vec<_> = configs
        .into_iter()
        .map(|(name, json)| {
            let pk: serde_json::Value = serde_json::from_str(json).unwrap();
            (name, pk)
        })
        .collect();

    assert_yaml_snapshot!("primary_key_configurations", pk_configurations);
}

#[test]
fn test_schema_mismatch_scenarios() {
    // Test various schema scenarios that might cause validation issues
    let query = load_query_from_file("test_data/test_query.json");

    // Collect actual vs expected schema information
    let schema_analysis: Vec<_> = query
        .sources
        .iter()
        .map(|source| {
            let actual_schema = if source.path.exists() {
                DataReader::get_schema(source)
                    .map(|schema| {
                        schema
                            .iter()
                            .map(|(k, v)| (k.to_string(), format!("{:?}", v)))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|_| vec![("error".to_string(), "failed_to_read".to_string())])
            } else {
                vec![("error".to_string(), "file_not_found".to_string())]
            };

            // Find mappings that reference this source
            let referenced_columns: Vec<_> = query
                .mappings
                .iter()
                .flat_map(|mapping| &mapping.source_fields)
                .filter(|sf| sf.source_file_id == source.id)
                .map(|sf| sf.column_name.clone())
                .collect();

            (source.id.clone(), actual_schema, referenced_columns)
        })
        .collect();

    assert_yaml_snapshot!("schema_analysis", schema_analysis);
}

#[test]
fn test_basic_query_output() {
    let query = load_query_from_file("test_data/test_query.json");

    // Execute the query and get the result
    let result = JoinEngine::execute_query(&query);

    match result {
        Ok(lazy_df) => {
            // Collect the result to get actual data
            match lazy_df.collect() {
                Ok(df) => {
                    // Convert to a structure suitable for snapshot testing
                    let output_data = capture_dataframe_output(&df);
                    assert_yaml_snapshot!("basic_query_output", output_data);
                }
                Err(e) => {
                    // Capture execution errors
                    let error_info = format!("Execution error: {}", e);
                    assert_yaml_snapshot!("basic_query_execution_error", error_info);
                }
            }
        }
        Err(e) => {
            // Capture validation or setup errors
            let error_info = format!("Query error: {}", e);
            assert_yaml_snapshot!("basic_query_validation_error", error_info);
        }
    }
}

#[test]
fn test_small_data_query_output() {
    // Create a modified query using the small data files
    let mut query = load_query_from_file("test_data/test_query.json");

    // Update the mapping to match the small data column names
    for mapping in &mut query.mappings {
        if mapping.destination_field == "customer_id" {
            // Update customers source field to match small_customers.csv
            for source_field in &mut mapping.source_fields {
                if source_field.source_file_id == "customers" {
                    source_field.column_name = "customer_id".to_string();
                }
            }
        }
        if mapping.destination_field == "customer_name" {
            // Update to use first_name from small_customers.csv
            for source_field in &mut mapping.source_fields {
                if source_field.source_file_id == "customers" {
                    source_field.column_name = "first_name".to_string();
                }
            }
        }
        if mapping.destination_field == "total_spent" {
            // Update to use 'total' from small_orders.csv
            for source_field in &mut mapping.source_fields {
                if source_field.source_file_id == "orders" {
                    source_field.column_name = "total".to_string();
                }
            }
        }
    }

    // Execute the query
    let result = JoinEngine::execute_query(&query);

    match result {
        Ok(lazy_df) => match lazy_df.collect() {
            Ok(df) => {
                let output_data = capture_dataframe_output(&df);
                assert_yaml_snapshot!("small_data_query_output", output_data);
            }
            Err(e) => {
                let error_info = format!("Small data execution error: {}", e);
                assert_yaml_snapshot!("small_data_execution_error", error_info);
            }
        },
        Err(e) => {
            let error_info = format!("Small data query error: {}", e);
            assert_yaml_snapshot!("small_data_query_error", error_info);
        }
    }
}

#[test]
fn test_comprehensive_merge_policies_output() {
    // Use the comprehensive query that tests all merge policy types
    let query_json = r#"{
  "sources": [
    {
      "id": "source1",
      "path": "test_data/small_customers.csv",
      "format": "csv"
    },
    {
      "id": "source2",
      "path": "test_data/small_orders.csv",
      "format": "csv"
    }
  ],
  "destination_schema": [
    {
      "name": "id",
      "data_type": "Int64"
    },
    {
      "name": "name",
      "data_type": "String"
    },
    {
      "name": "total_value",
      "data_type": "Float64"
    },
    {
      "name": "order_count",
      "data_type": "Int64"
    }
  ],
  "primary_keys": {
    "logic": "and",
    "keys": ["id"]
  },
  "mappings": [
    {
      "destination_field": "id",
      "policy": {
        "type": "firstMatch",
        "priority": ["source1", "source2"]
      },
      "source_fields": [
        {
          "id": "customer_id_1",
          "source_file_id": "source1",
          "column_name": "customer_id"
        },
        {
          "id": "customer_id_2",
          "source_file_id": "source2",
          "column_name": "customer_id"
        }
      ]
    },
    {
      "destination_field": "name",
      "policy": {
        "type": "firstMatch",
        "priority": ["source1"]
      },
      "source_fields": [
        {
          "id": "first_name",
          "source_file_id": "source1",
          "column_name": "first_name"
        }
      ]
    },
    {
      "destination_field": "total_value",
      "policy": {
        "type": "sum"
      },
      "source_fields": [
        {
          "id": "order_total",
          "source_file_id": "source2",
          "column_name": "total"
        }
      ]
    },
    {
      "destination_field": "order_count",
      "policy": {
        "type": "count"
      },
      "source_fields": [
        {
          "id": "order_id",
          "source_file_id": "source2",
          "column_name": "order_id"
        }
      ]
    }
  ]
}"#;

    let query: QueryPlan = serde_json::from_str(query_json).unwrap();

    // Execute the comprehensive query
    let result = JoinEngine::execute_query(&query);

    match result {
        Ok(lazy_df) => match lazy_df.collect() {
            Ok(df) => {
                let output_data = capture_dataframe_output(&df);
                assert_yaml_snapshot!("comprehensive_merge_policies_output", output_data);
            }
            Err(e) => {
                let error_info = format!("Comprehensive query execution error: {}", e);
                assert_yaml_snapshot!("comprehensive_query_execution_error", error_info);
            }
        },
        Err(e) => {
            let error_info = format!("Comprehensive query error: {}", e);
            assert_yaml_snapshot!("comprehensive_query_error", error_info);
        }
    }
}

#[test]
fn test_aggregation_only_query_output() {
    // Create a query that focuses on aggregation operations
    let query_json = r#"{
  "sources": [
    {
      "id": "orders",
      "path": "test_data/small_orders.csv",
      "format": "csv"
    }
  ],
  "destination_schema": [
    {
      "name": "customer_id",
      "data_type": "Int64"
    },
    {
      "name": "total_spent",
      "data_type": "Float64"
    },
    {
      "name": "order_count",
      "data_type": "Int64"
    },
    {
      "name": "avg_order",
      "data_type": "Float64"
    },
    {
      "name": "min_order",
      "data_type": "Float64"
    },
    {
      "name": "max_order",
      "data_type": "Float64"
    }
  ],
  "primary_keys": {
    "logic": "and",
    "keys": ["customer_id"]
  },
  "mappings": [
    {
      "destination_field": "customer_id",
      "policy": {
        "type": "firstMatch",
        "priority": ["orders"]
      },
      "source_fields": [
        {
          "id": "cust_id",
          "source_file_id": "orders",
          "column_name": "customer_id"
        }
      ]
    },
    {
      "destination_field": "total_spent",
      "policy": {
        "type": "sum"
      },
      "source_fields": [
        {
          "id": "order_total",
          "source_file_id": "orders",
          "column_name": "total"
        }
      ]
    },
    {
      "destination_field": "order_count",
      "policy": {
        "type": "count"
      },
      "source_fields": [
        {
          "id": "order_id",
          "source_file_id": "orders",
          "column_name": "order_id"
        }
      ]
    },
    {
      "destination_field": "avg_order",
      "policy": {
        "type": "average"
      },
      "source_fields": [
        {
          "id": "order_avg",
          "source_file_id": "orders",
          "column_name": "total"
        }
      ]
    },
    {
      "destination_field": "min_order",
      "policy": {
        "type": "min"
      },
      "source_fields": [
        {
          "id": "order_min",
          "source_file_id": "orders",
          "column_name": "total"
        }
      ]
    },
    {
      "destination_field": "max_order",
      "policy": {
        "type": "max"
      },
      "source_fields": [
        {
          "id": "order_max",
          "source_file_id": "orders",
          "column_name": "total"
        }
      ]
    }
  ]
}"#;

    let query: QueryPlan = serde_json::from_str(query_json).unwrap();

    // Execute the aggregation query
    let result = JoinEngine::execute_query(&query);

    match result {
        Ok(lazy_df) => match lazy_df.collect() {
            Ok(df) => {
                let output_data = capture_dataframe_output(&df);
                assert_yaml_snapshot!("aggregation_only_query_output", output_data);
            }
            Err(e) => {
                let error_info = format!("Aggregation query execution error: {}", e);
                assert_yaml_snapshot!("aggregation_query_execution_error", error_info);
            }
        },
        Err(e) => {
            let error_info = format!("Aggregation query error: {}", e);
            assert_yaml_snapshot!("aggregation_query_error", error_info);
        }
    }
}

// Helper function to convert a polars DataFrame to a snapshot-friendly format
fn capture_dataframe_output(df: &polars::prelude::DataFrame) -> serde_json::Value {
    use serde_json::{Map, Value};

    let mut output = Map::new();

    // Add basic metadata
    output.insert(
        "shape".to_string(),
        Value::Array(vec![
            Value::Number(df.height().into()),
            Value::Number(df.width().into()),
        ]),
    );

    // Add column information
    let columns: Vec<Value> = df
        .get_column_names()
        .iter()
        .map(|name| Value::String(name.to_string()))
        .collect();
    output.insert("columns".to_string(), Value::Array(columns));

    // Add data rows (limited to first 20 rows for snapshot brevity)
    let max_rows = std::cmp::min(df.height(), 20);
    let mut rows = Vec::new();

    for i in 0..max_rows {
        let mut row = Map::new();
        for (col_idx, col_name) in df.get_column_names().iter().enumerate() {
            if let Ok(col) = df.get_columns()[col_idx].get(i) {
                let value_str = format!("{}", col);
                row.insert(col_name.to_string(), Value::String(value_str));
            }
        }
        rows.push(Value::Object(row));
    }

    output.insert("data".to_string(), Value::Array(rows));

    // Add truncation info if we limited rows
    if df.height() > max_rows {
        output.insert("truncated".to_string(), Value::Bool(true));
        output.insert("total_rows".to_string(), Value::Number(df.height().into()));
    } else {
        output.insert("truncated".to_string(), Value::Bool(false));
    }

    Value::Object(output)
}
