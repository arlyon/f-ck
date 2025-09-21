use f_ck::{JoinEngine, QueryPlan};
use insta::assert_yaml_snapshot;
use std::path::PathBuf;

#[test]
fn test_v1_query_plan_parsing() {
    let json = r#"{
        "sources": [
            {
                "id": "customers",
                "path": "test_data/customers.csv",
                "format": "csv"
            },
            {
                "id": "orders",
                "path": "test_data/orders.csv",
                "format": "csv"
            }
        ],
        "destination_schema": [
            {
                "name": "customer_id",
                "data_type": "Integer"
            },
            {
                "name": "customer_name",
                "data_type": "String"
            },
            {
                "name": "total_spent",
                "data_type": "Float"
            }
        ],
        "primary_keys": {
            "logic": "or",
            "keys": ["customer_id"]
        },
        "mappings": [
            {
                "destination_field": "customer_id",
                "policy": {
                    "type": "firstMatch",
                    "priority": ["customers", "orders"]
                },
                "source_fields": [
                    {
                        "id": "cust_id",
                        "source_file_id": "customers",
                        "column_name": "id"
                    }
                ]
            },
            {
                "destination_field": "customer_name",
                "policy": {
                    "type": "firstMatch",
                    "priority": ["customers"]
                },
                "source_fields": [
                    {
                        "id": "cust_name",
                        "source_file_id": "customers",
                        "column_name": "name"
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
            }
        ]
    }"#;

    let query_plan = QueryPlan::from_json(json).expect("Should parse valid JSON");
    assert_yaml_snapshot!(query_plan);
}

#[test]
fn test_v1_query_plan_validation() {
    let json = r#"{
        "sources": [],
        "destination_schema": [],
        "primary_keys": {
            "logic": "or",
            "keys": []
        },
        "mappings": []
    }"#;

    let query_plan = QueryPlan::from_json(json).expect("Should parse JSON");
    let validation_result = query_plan.validate();

    assert!(validation_result.is_err());
    assert_yaml_snapshot!(validation_result.unwrap_err().to_string());
}

#[test]
fn test_v1_json_schema_generation() {
    let schema = QueryPlan::json_schema().expect("Should generate schema");
    let parsed_schema: serde_json::Value =
        serde_json::from_str(&schema).expect("Should be valid JSON");

    // Test that the schema has the expected structure
    assert!(parsed_schema["$schema"].is_string());
    assert!(parsed_schema["title"].is_string());
    assert!(parsed_schema["properties"].is_object());
    assert!(parsed_schema["$defs"].is_object());

    assert_yaml_snapshot!(parsed_schema);
}

#[test]
fn test_v1_query_plan_serialization_roundtrip() {
    let original_json =
        std::fs::read_to_string("test_data/test_query.json").expect("Should read test query file");

    let query_plan = QueryPlan::from_json(&original_json).expect("Should parse original JSON");

    let serialized = query_plan.to_json().expect("Should serialize back to JSON");

    let reparsed = QueryPlan::from_json(&serialized).expect("Should parse serialized JSON");

    assert_yaml_snapshot!("original", query_plan);
    assert_yaml_snapshot!("reparsed", reparsed);
}

#[test]
fn test_v1_merge_policies() {
    use f_ck::MergePolicy;

    let policies = vec![
        MergePolicy::FirstMatch {
            priority: vec!["source1".to_string(), "source2".to_string()],
        },
        MergePolicy::Sum,
        MergePolicy::Count,
        MergePolicy::Average,
        MergePolicy::Min,
        MergePolicy::Max,
    ];

    for policy in policies {
        let json = serde_json::to_string(&policy).expect("Should serialize");
        let deserialized: MergePolicy = serde_json::from_str(&json).expect("Should deserialize");
        assert_yaml_snapshot!(format!("policy_{:?}", policy), deserialized);
    }
}
