use f_ck::dsl::{
    DestinationField, DslQuery, Mapping, MergePolicy, PrimaryKeyLogic, PrimaryKeySpec, Query,
    QueryPlan, Source, SourceFieldSpec,
};
use f_ck::engine::JoinEngine;
use std::fs;
use tempfile::tempdir;
use tracing_test::traced_test;

/// Test that records maintain the order from the primary CSV file
#[test]
#[traced_test]
fn test_primary_source_order_preservation() {
    let temp_dir = tempdir().unwrap();

    // Create CSV A (primary source) with specific order
    let csv_a_path = temp_dir.path().join("customers_a.csv");
    fs::write(
        &csv_a_path,
        "cid,name,age\n\
         100,Alice,25\n\
         200,Bob,30\n\
         300,Charlie,35\n\
         150,Frank,28\n\
         250,Eve,32",
    )
    .unwrap();

    // Create CSV B (secondary source) with different order
    let csv_b_path = temp_dir.path().join("customers_b.csv");
    fs::write(
        &csv_b_path,
        "customer_id,email,city\n\
         250,eve@example.com,Portland\n\
         100,alice@example.com,Seattle\n\
         150,frank@example.com,Denver\n\
         200,bob@example.com,Austin\n\
         300,charlie@example.com,Boston",
    )
    .unwrap();

    let query = QueryPlan {
        sources: vec![
            Source {
                id: "A".to_string(),
                path: csv_a_path,
                format: "csv".to_string(),
            },
            Source {
                id: "B".to_string(),
                path: csv_b_path,
                format: "csv".to_string(),
            },
        ],
        query: Query::Dsl(DslQuery {
            destination_schema: vec![
                DestinationField {
                    name: "customer_id".to_string(),
                    data_type: "integer".to_string(),
                },
                DestinationField {
                    name: "name".to_string(),
                    data_type: "string".to_string(),
                },
                DestinationField {
                    name: "email".to_string(),
                    data_type: "string".to_string(),
                },
            ],
            primary_keys: PrimaryKeySpec {
                keys: vec!["cid".to_string()],
            },
            mappings: vec![
                Mapping {
                    destination_field: "customer_id".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["A".to_string(), "B".to_string()],
                    },
                    source_fields: vec![
                        SourceFieldSpec {
                            source_file_id: "A".to_string(),
                            column_name: "cid".to_string(),
                        },
                        SourceFieldSpec {
                            source_file_id: "B".to_string(),
                            column_name: "customer_id".to_string(),
                        },
                    ],
                },
                Mapping {
                    destination_field: "name".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["A".to_string()],
                    },
                    source_fields: vec![SourceFieldSpec {
                        source_file_id: "A".to_string(),
                        column_name: "name".to_string(),
                    }],
                },
                Mapping {
                    destination_field: "email".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["B".to_string()],
                    },
                    source_fields: vec![SourceFieldSpec {
                        source_file_id: "B".to_string(),
                        column_name: "email".to_string(),
                    }],
                },
            ],
        }),
    };

    let result = JoinEngine::execute_query(&query).unwrap();
    let df = result.collect().unwrap();

    tracing::debug!("df: {:?}", df);

    // Check that the records appear in the same order as CSV A
    let customer_ids: Vec<i64> = df
        .column("customer_id")
        .unwrap()
        .i64()
        .unwrap()
        .into_no_null_iter()
        .collect();

    // Should be ordered as: [100, 200, 300, 150, 250] - same as CSV A
    assert_eq!(customer_ids, vec![100, 200, 300, 150, 250]);

    // Verify that matched records have correct data
    let names: Vec<&str> = df
        .column("name")
        .unwrap()
        .str()
        .unwrap()
        .into_no_null_iter()
        .collect();
    assert_eq!(names, vec!["Alice", "Bob", "Charlie", "Frank", "Eve"]);
}

/// Test that primary source takes priority in deduplication
#[test]
fn test_primary_source_priority_deduplication() {
    let temp_dir = tempdir().unwrap();

    // CSV A has specific customers
    let csv_a_path = temp_dir.path().join("orders_a.csv");
    fs::write(
        &csv_a_path,
        "order_id,amount\n\
         1001,100.50\n\
         1002,200.75\n\
         1003,150.00",
    )
    .unwrap();

    // CSV B has overlapping and additional customers
    let csv_b_path = temp_dir.path().join("orders_b.csv");
    fs::write(
        &csv_b_path,
        "order_number,price\n\
         1002,999.99\n\
         1004,300.25\n\
         1005,450.00",
    )
    .unwrap();

    let query = QueryPlan {
        sources: vec![
            Source {
                id: "A".to_string(),
                path: csv_a_path,
                format: "csv".to_string(),
            },
            Source {
                id: "B".to_string(),
                path: csv_b_path,
                format: "csv".to_string(),
            },
        ],
        query: Query::Dsl(DslQuery {
            destination_schema: vec![
                DestinationField {
                    name: "order_id".to_string(),
                    data_type: "integer".to_string(),
                },
                DestinationField {
                    name: "amount".to_string(),
                    data_type: "float".to_string(),
                },
            ],
            primary_keys: PrimaryKeySpec {
                keys: vec!["order_id".to_string()],
            },
            mappings: vec![
                Mapping {
                    destination_field: "order_id".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["A".to_string(), "B".to_string()],
                    },
                    source_fields: vec![
                        SourceFieldSpec {
                            source_file_id: "A".to_string(),
                            column_name: "order_id".to_string(),
                        },
                        SourceFieldSpec {
                            source_file_id: "B".to_string(),
                            column_name: "order_number".to_string(),
                        },
                    ],
                },
                Mapping {
                    destination_field: "amount".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["A".to_string(), "B".to_string()],
                    },
                    source_fields: vec![
                        SourceFieldSpec {
                            source_file_id: "A".to_string(),
                            column_name: "amount".to_string(),
                        },
                        SourceFieldSpec {
                            source_file_id: "B".to_string(),
                            column_name: "price".to_string(),
                        },
                    ],
                },
            ],
        }),
    };

    let result = JoinEngine::execute_query(&query).unwrap();
    let df = result.collect().unwrap();

    // Should only contain records from A (since A takes priority)
    // Order should be preserved from A
    let order_ids: Vec<i64> = df
        .column("order_id")
        .unwrap()
        .i64()
        .unwrap()
        .into_no_null_iter()
        .collect();

    // Only orders from A should appear, in A's order
    assert_eq!(order_ids, vec![1002]);

    // For order 1002, should use value from A (200.75), not B (999.99)
    let amounts: Vec<f64> = df
        .column("amount")
        .unwrap()
        .f64()
        .unwrap()
        .into_no_null_iter()
        .collect();
    assert_eq!(amounts, vec![200.75]);
}

/// Test that records not in primary source but in secondary sources are added after
#[test]
fn test_secondary_source_records_appended() {
    let temp_dir = tempdir().unwrap();

    // CSV A (primary) - smaller set
    let csv_a_path = temp_dir.path().join("products_a.csv");
    fs::write(
        &csv_a_path,
        "pid,name\n\
         101,Widget A\n\
         103,Widget C",
    )
    .unwrap();

    // CSV B (secondary) - has overlapping and additional products
    let csv_b_path = temp_dir.path().join("products_b.csv");
    fs::write(
        &csv_b_path,
        "product_id,description\n\
         102,Widget B\n\
         103,Widget C Updated\n\
         104,Widget D",
    )
    .unwrap();

    let query = QueryPlan {
        sources: vec![
            Source {
                id: "A".to_string(),
                path: csv_a_path,
                format: "csv".to_string(),
            },
            Source {
                id: "B".to_string(),
                path: csv_b_path,
                format: "csv".to_string(),
            },
        ],
        query: Query::Dsl(DslQuery {
            destination_schema: vec![
                DestinationField {
                    name: "product_id".to_string(),
                    data_type: "integer".to_string(),
                },
                DestinationField {
                    name: "name".to_string(),
                    data_type: "string".to_string(),
                },
            ],
            primary_keys: PrimaryKeySpec {
                keys: vec!["product_id".to_string()],
            },
            mappings: vec![
                Mapping {
                    destination_field: "product_id".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["A".to_string(), "B".to_string()],
                    },
                    source_fields: vec![
                        SourceFieldSpec {
                            source_file_id: "A".to_string(),
                            column_name: "pid".to_string(),
                        },
                        SourceFieldSpec {
                            source_file_id: "B".to_string(),
                            column_name: "product_id".to_string(),
                        },
                    ],
                },
                Mapping {
                    destination_field: "name".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["A".to_string(), "B".to_string()],
                    },
                    source_fields: vec![
                        SourceFieldSpec {
                            source_file_id: "A".to_string(),
                            column_name: "name".to_string(),
                        },
                        SourceFieldSpec {
                            source_file_id: "B".to_string(),
                            column_name: "description".to_string(),
                        },
                    ],
                },
            ],
        }),
    };

    let result = JoinEngine::execute_query(&query).unwrap();
    let df = result.collect().unwrap();

    // Should have all products: A's products first (in A's order), then B's unique products
    let product_ids: Vec<Option<i64>> = df
        .column("product_id")
        .unwrap()
        .i64()
        .unwrap()
        .into_iter()
        .collect();

    // Products from A should come first in A's order: [101, 103]
    // For 103, A's data should take priority over B's
    assert_eq!(product_ids[0], Some(101));
    assert_eq!(product_ids[1], Some(103));

    let names: Vec<Option<&str>> = df
        .column("name")
        .unwrap()
        .str()
        .unwrap()
        .into_iter()
        .collect();

    // For overlapping product 103, should use A's name, not B's description
    assert_eq!(names[0], Some("Widget A"));
    assert_eq!(names[1], Some("Widget C")); // A's name, not "Widget C Updated"
}

/// Test complex three-way join with ordering
#[test]
fn test_three_way_join_ordering() {
    let temp_dir = tempdir().unwrap();

    // CSV A (primary)
    let csv_a_path = temp_dir.path().join("users_a.csv");
    fs::write(
        &csv_a_path,
        "uid,username\n\
         5,eve\n\
         1,alice\n\
         3,charlie",
    )
    .unwrap();

    // CSV B (secondary)
    let csv_b_path = temp_dir.path().join("users_b.csv");
    fs::write(
        &csv_b_path,
        "user_id,email\n\
         2,bob@example.com\n\
         1,alice@example.com\n\
         4,diana@example.com",
    )
    .unwrap();

    // CSV C (tertiary)
    let csv_c_path = temp_dir.path().join("users_c.csv");
    fs::write(
        &csv_c_path,
        "id,phone\n\
         1,555-0001\n\
         6,555-0006\n\
         3,555-0003",
    )
    .unwrap();

    let query = QueryPlan {
        sources: vec![
            Source {
                id: "A".to_string(),
                path: csv_a_path,
                format: "csv".to_string(),
            },
            Source {
                id: "B".to_string(),
                path: csv_b_path,
                format: "csv".to_string(),
            },
            Source {
                id: "C".to_string(),
                path: csv_c_path,
                format: "csv".to_string(),
            },
        ],
        query: Query::Dsl(DslQuery {
            destination_schema: vec![
                DestinationField {
                    name: "user_id".to_string(),
                    data_type: "integer".to_string(),
                },
                DestinationField {
                    name: "username".to_string(),
                    data_type: "string".to_string(),
                },
                DestinationField {
                    name: "email".to_string(),
                    data_type: "string".to_string(),
                },
                DestinationField {
                    name: "phone".to_string(),
                    data_type: "string".to_string(),
                },
            ],
            primary_keys: PrimaryKeySpec {
                keys: vec!["user_id".to_string()],
            },
            mappings: vec![
                Mapping {
                    destination_field: "user_id".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["A".to_string(), "B".to_string(), "C".to_string()],
                    },
                    source_fields: vec![
                        SourceFieldSpec {
                            source_file_id: "A".to_string(),
                            column_name: "uid".to_string(),
                        },
                        SourceFieldSpec {
                            source_file_id: "B".to_string(),
                            column_name: "user_id".to_string(),
                        },
                        SourceFieldSpec {
                            source_file_id: "C".to_string(),
                            column_name: "id".to_string(),
                        },
                    ],
                },
                Mapping {
                    destination_field: "username".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["A".to_string()],
                    },
                    source_fields: vec![SourceFieldSpec {
                        source_file_id: "A".to_string(),
                        column_name: "username".to_string(),
                    }],
                },
                Mapping {
                    destination_field: "email".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["B".to_string()],
                    },
                    source_fields: vec![SourceFieldSpec {
                        source_file_id: "B".to_string(),
                        column_name: "email".to_string(),
                    }],
                },
                Mapping {
                    destination_field: "phone".to_string(),
                    policy: MergePolicy::FirstMatch {
                        priority: vec!["C".to_string()],
                    },
                    source_fields: vec![SourceFieldSpec {
                        source_file_id: "C".to_string(),
                        column_name: "phone".to_string(),
                    }],
                },
            ],
        }),
    };

    let result = JoinEngine::execute_query(&query).unwrap();
    let df = result.collect().unwrap();

    // Results should be ordered by A first: [5, 1, 3] (A's order)
    let user_ids: Vec<i64> = df
        .column("user_id")
        .unwrap()
        .i64()
        .unwrap()
        .into_no_null_iter()
        .collect();

    // Should start with A's records in A's order
    assert_eq!(user_ids[0], 5); // eve (first in A)
    assert_eq!(user_ids[1], 1); // alice (second in A)
    assert_eq!(user_ids[2], 3); // charlie (third in A)

    // Verify usernames come from A
    let usernames: Vec<Option<&str>> = df
        .column("username")
        .unwrap()
        .str()
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(usernames[0], Some("eve"));
    assert_eq!(usernames[1], Some("alice"));
    assert_eq!(usernames[2], Some("charlie"));
}
