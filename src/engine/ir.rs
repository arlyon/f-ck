//! Internal Representation (IR) for granular operations tracked by Salsa
//!
//! This module defines the low-level operations that can be individually tracked
//! and cached by Salsa. The high-level Query DSL is compiled into these IR operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A unique identifier for an IR operation result
pub type OperationId = String;

/// Low-level operations that can be tracked individually by Salsa
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum IROperation {
    /// Load raw file contents from disk
    LoadSource {
        id: OperationId,
        path: PathBuf,
    },
    
    /// Parse loaded content into a DataFrame with type inference
    ParseSource {
        id: OperationId,
        input: OperationId, // LoadSource result
        format: String,
        schema_hint: Option<HashMap<String, ColumnType>>,
    },
    
    /// Scan a portion of data to infer column types
    InferColumnTypes {
        id: OperationId,
        input: OperationId, // ParseSource result
        sample_size: usize,
        columns: Vec<String>, // Specific columns to analyze, or empty for all
    },
    
    /// Create an entity link between two data sources
    DefineLink {
        id: OperationId,
        left: OperationId,
        left_on: String,
        right: OperationId,
        right_on: String,
        link_type: LinkType,
    },
    
    /// Resolve entities using transitive closure across all links
    ResolveEntities {
        id: OperationId,
        links: Vec<OperationId>, // DefineLink results
        algorithm: EntityResolutionAlgorithm,
    },
    
    /// Apply column mapping with merge policy
    ApplyMapping {
        id: OperationId,
        primary_source: OperationId,
        entity_resolution: OperationId, // ResolveEntities result
        column_name: String,
        source_column: String,
        policy: MergePolicy,
    },
    
    /// Select specific columns from a result
    Select {
        id: OperationId,
        input: OperationId,
        columns: Vec<String>,
    },
    
    /// Filter rows based on a predicate
    Filter {
        id: OperationId,
        input: OperationId,
        predicate: FilterPredicate,
    },
    
    /// Combine multiple column mappings into final output
    CombineColumns {
        id: OperationId,
        column_mappings: Vec<OperationId>, // ApplyMapping results
    },
}

/// Enhanced column type information with range tracking
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ColumnType {
    /// Simple types
    Integer,
    Float,
    Text,
    Boolean,
    Date,
    DateTime,
    
    /// Semantic types with detection patterns
    Email,
    Phone,
    CountryCode,
    PostalCode,
    Currency,
    Name,
    
    /// Mixed type with row ranges for each type
    Mixed {
        type_ranges: Vec<TypeRange>,
    },
    
    /// Unknown type that needs further analysis
    Unknown,
}

/// Represents a range of rows that have a specific type
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TypeRange {
    pub start_row: usize,
    pub end_row: usize,
    pub column_type: Box<ColumnType>,
    pub confidence: f32, // 0.0 to 1.0
}

/// Different types of entity links
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LinkType {
    /// Exact match on key values
    Exact,
    /// Fuzzy matching with similarity threshold (stored as integer percentage for hashing)
    Fuzzy { threshold_percent: u8 }, // 0-100
    /// Semantic matching (e.g., email domains)
    Semantic,
}

/// Algorithms for entity resolution
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EntityResolutionAlgorithm {
    /// Simple transitive closure
    TransitiveClosure,
    /// Union-Find with ranking
    UnionFind,
    /// Graph-based clustering
    GraphClustering { max_cluster_size: usize },
}

/// Merge policies for combining column values
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MergePolicy {
    /// Take first non-null value with priority order
    FirstMatch { priority: Vec<OperationId> },
    /// Sum numeric values
    Sum,
    /// Count occurrences
    Count,
    /// Calculate average
    Average,
    /// Take minimum value
    Min,
    /// Take maximum value
    Max,
    /// Concatenate text values
    Concat { separator: String },
    /// Custom function
    Custom { function: String },
}

/// Filter predicates for row filtering
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FilterPredicate {
    /// Column equals value
    Equals { column: String, value: String },
    /// Column contains value
    Contains { column: String, value: String },
    /// Column matches regex
    Regex { column: String, pattern: String },
    /// Column is null/empty
    IsNull { column: String },
    /// Column is not null/empty
    IsNotNull { column: String },
    /// Numeric comparisons (stored as string for consistency)
    GreaterThan { column: String, value: String },
    LessThan { column: String, value: String },
    /// Logical operations
    And { left: Box<FilterPredicate>, right: Box<FilterPredicate> },
    Or { left: Box<FilterPredicate>, right: Box<FilterPredicate> },
    Not { predicate: Box<FilterPredicate> },
}

/// Represents the execution plan as a directed acyclic graph
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRExecutionPlan {
    /// All operations in topological order
    pub operations: Vec<IROperation>,
    /// Dependencies between operations
    pub dependencies: HashMap<OperationId, Vec<OperationId>>,
    /// Output operations (operations whose results are needed)
    pub outputs: Vec<OperationId>,
}

impl IRExecutionPlan {
    /// Create a new empty execution plan
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            dependencies: HashMap::new(),
            outputs: Vec::new(),
        }
    }
    
    /// Add an operation to the plan
    pub fn add_operation(&mut self, operation: IROperation) {
        let id = operation.id().clone();
        let deps = operation.dependencies();
        
        self.operations.push(operation);
        self.dependencies.insert(id, deps);
    }
    
    /// Mark an operation as an output
    pub fn add_output(&mut self, operation_id: OperationId) {
        if !self.outputs.contains(&operation_id) {
            self.outputs.push(operation_id);
        }
    }
    
    /// Validate the execution plan for correctness
    pub fn validate(&self) -> anyhow::Result<()> {
        // Check that all dependencies exist
        for (op_id, deps) in &self.dependencies {
            for dep in deps {
                if !self.dependencies.contains_key(dep) {
                    return Err(anyhow::anyhow!(
                        "Operation '{}' depends on unknown operation '{}'", 
                        op_id, dep
                    ));
                }
            }
        }
        
        // Check for cycles (simplified check)
        // In a real implementation, we'd use a proper cycle detection algorithm
        for output in &self.outputs {
            if !self.dependencies.contains_key(output) {
                return Err(anyhow::anyhow!(
                    "Output operation '{}' does not exist in plan", 
                    output
                ));
            }
        }
        
        Ok(())
    }
    
    /// Get operations in topological order for execution
    pub fn execution_order(&self) -> Vec<&IROperation> {
        // Simple topological sort
        // In a real implementation, we'd use Kahn's algorithm or DFS
        self.operations.iter().collect()
    }
}

impl IROperation {
    /// Get the ID of this operation
    pub fn id(&self) -> &OperationId {
        match self {
            IROperation::LoadSource { id, .. } => id,
            IROperation::ParseSource { id, .. } => id,
            IROperation::InferColumnTypes { id, .. } => id,
            IROperation::DefineLink { id, .. } => id,
            IROperation::ResolveEntities { id, .. } => id,
            IROperation::ApplyMapping { id, .. } => id,
            IROperation::Select { id, .. } => id,
            IROperation::Filter { id, .. } => id,
            IROperation::CombineColumns { id, .. } => id,
        }
    }
    
    /// Get the dependencies of this operation
    pub fn dependencies(&self) -> Vec<OperationId> {
        match self {
            IROperation::LoadSource { .. } => vec![],
            IROperation::ParseSource { input, .. } => vec![input.clone()],
            IROperation::InferColumnTypes { input, .. } => vec![input.clone()],
            IROperation::DefineLink { left, right, .. } => vec![left.clone(), right.clone()],
            IROperation::ResolveEntities { links, .. } => links.clone(),
            IROperation::ApplyMapping { primary_source, entity_resolution, .. } => {
                vec![primary_source.clone(), entity_resolution.clone()]
            },
            IROperation::Select { input, .. } => vec![input.clone()],
            IROperation::Filter { input, .. } => vec![input.clone()],
            IROperation::CombineColumns { column_mappings, .. } => column_mappings.clone(),
        }
    }
    
    /// Check if this operation can be parallelized
    pub fn is_parallelizable(&self) -> bool {
        match self {
            IROperation::LoadSource { .. } => true,
            IROperation::ParseSource { .. } => false, // Depends on file format
            IROperation::InferColumnTypes { .. } => true,
            IROperation::DefineLink { .. } => true,
            IROperation::ResolveEntities { .. } => false, // Global operation
            IROperation::ApplyMapping { .. } => true,
            IROperation::Select { .. } => true,
            IROperation::Filter { .. } => true,
            IROperation::CombineColumns { .. } => false, // Global operation
        }
    }
}

impl Default for IRExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_operation_dependencies() {
        let load_op = IROperation::LoadSource {
            id: "load1".to_string(),
            path: PathBuf::from("test.csv"),
        };
        
        let parse_op = IROperation::ParseSource {
            id: "parse1".to_string(),
            input: "load1".to_string(),
            format: "csv".to_string(),
            schema_hint: None,
        };
        
        assert_eq!(load_op.dependencies(), Vec::<String>::new());
        assert_eq!(parse_op.dependencies(), vec!["load1".to_string()]);
    }
    
    #[test]
    fn test_execution_plan_validation() {
        let mut plan = IRExecutionPlan::new();
        
        plan.add_operation(IROperation::LoadSource {
            id: "load1".to_string(),
            path: PathBuf::from("test.csv"),
        });
        
        plan.add_operation(IROperation::ParseSource {
            id: "parse1".to_string(),
            input: "load1".to_string(),
            format: "csv".to_string(),
            schema_hint: None,
        });
        
        plan.add_output("parse1".to_string());
        
        assert!(plan.validate().is_ok());
    }
    
    #[test]
    fn test_execution_plan_invalid_dependency() {
        let mut plan = IRExecutionPlan::new();
        
        plan.add_operation(IROperation::ParseSource {
            id: "parse1".to_string(),
            input: "nonexistent".to_string(),
            format: "csv".to_string(),
            schema_hint: None,
        });
        
        assert!(plan.validate().is_err());
    }
    
    #[test]
    fn test_mixed_column_type() {
        let mixed_type = ColumnType::Mixed {
            type_ranges: vec![
                TypeRange {
                    start_row: 0,
                    end_row: 10,
                    column_type: Box::new(ColumnType::Email),
                    confidence: 0.95,
                },
                TypeRange {
                    start_row: 11,
                    end_row: 20,
                    column_type: Box::new(ColumnType::Phone),
                    confidence: 0.90,
                },
            ],
        };
        
        // Test serialization
        let json = serde_json::to_string(&mixed_type).unwrap();
        let deserialized: ColumnType = serde_json::from_str(&json).unwrap();
        assert_eq!(mixed_type, deserialized);
    }
}