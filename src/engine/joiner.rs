use crate::dsl::{QueryPlan, Mapping, MergePolicy};
use crate::engine::DataReader;
use anyhow::Result;
use polars::lazy::frame::LazyFrame;
use polars::prelude::*;
use std::collections::HashMap;

pub struct JoinEngine;

impl JoinEngine {
    pub fn execute_query(query: &QueryPlan) -> Result<LazyFrame> {
        query.validate()?;

        // Load all source data
        let mut dataframes: HashMap<String, LazyFrame> = HashMap::new();
        for source in &query.sources {
            let df = DataReader::read_source(source)?;
            dataframes.insert(source.id.clone(), df);
        }

        // For now, implement a simple join strategy
        // This is a basic implementation - will be enhanced with proper transitive closure later
        Self::simple_join(query, dataframes)
    }

    fn simple_join(query: &QueryPlan, mut dataframes: HashMap<String, LazyFrame>) -> Result<LazyFrame> {
        if dataframes.is_empty() {
            return Err(anyhow::anyhow!("No dataframes to join"));
        }

        // Start with the first dataframe
        let first_source_id = query.sources[0].id.clone();
        let mut result = dataframes.remove(&first_source_id)
            .ok_or_else(|| anyhow::anyhow!("First source not found"))?;

        // For each additional source, perform joins based on mappings
        for source in query.sources.iter().skip(1) {
            if let Some(right_df) = dataframes.remove(&source.id) {
                result = Self::join_dataframes(result, right_df, query)?;
            }
        }

        // Apply field mappings and create destination schema with aggregation
        Self::apply_mappings_with_aggregation(result, query)
    }

    fn join_dataframes(left: LazyFrame, right: LazyFrame, query: &QueryPlan) -> Result<LazyFrame> {
        // Find the primary key mapping to determine join columns
        let primary_key = &query.primary_keys.keys[0]; // Using first primary key
        
        // Find the mapping for this primary key
        let pk_mapping = query.mappings.iter()
            .find(|m| m.destination_field == *primary_key)
            .ok_or_else(|| anyhow::anyhow!("No mapping found for primary key: {}", primary_key))?;
        
        // Extract the source columns for the join
        // Assume first source field is from left table, second from right table
        if pk_mapping.source_fields.len() < 2 {
            return Err(anyhow::anyhow!("Primary key mapping needs at least 2 source fields for join"));
        }
        
        let left_col = &pk_mapping.source_fields[0].column_name;
        let right_col = &pk_mapping.source_fields[1].column_name;

        let result = left.join(
            right,
            [col(left_col)],
            [col(right_col)],
            JoinArgs::new(JoinType::Left),
        );

        Ok(result)
    }

    fn apply_mappings_with_aggregation(df: LazyFrame, query: &QueryPlan) -> Result<LazyFrame> {
        // First, determine if we need aggregation by checking if any mapping uses Sum, Count, Average
        let needs_aggregation = query.mappings.iter().any(|mapping| {
            matches!(mapping.policy, MergePolicy::Sum | MergePolicy::Count | MergePolicy::Average)
        });

        if needs_aggregation {
            // Find the primary key columns for grouping
            let primary_key = &query.primary_keys.keys[0];
            let pk_mapping = query.mappings.iter()
                .find(|m| m.destination_field == *primary_key)
                .ok_or_else(|| anyhow::anyhow!("No mapping found for primary key: {}", primary_key))?;
            
            // Use the left table's column for grouping (from the join)
            let group_col = &pk_mapping.source_fields[0].column_name;
            
            // Create aggregation expressions
            let mut agg_exprs = Vec::new();
            
            for dest_field in &query.destination_schema {
                if let Some(mapping) = query.mappings.iter()
                    .find(|m| m.destination_field == dest_field.name) {
                    
                    let expr = Self::create_aggregation_expression(mapping)?;
                    agg_exprs.push(expr.alias(&dest_field.name));
                } else {
                    // For non-mapped fields, use first() to get one value per group
                    agg_exprs.push(col(&dest_field.name).first().alias(&dest_field.name));
                }
            }
            
            Ok(df.group_by([col(group_col)]).agg(agg_exprs))
        } else {
            // No aggregation needed, just apply regular mappings
            Self::apply_simple_mappings(df, query)
        }
    }

    fn apply_simple_mappings(df: LazyFrame, query: &QueryPlan) -> Result<LazyFrame> {
        let mut exprs = Vec::new();

        // Create expressions for each destination field based on mappings
        for dest_field in &query.destination_schema {
            if let Some(mapping) = query.mappings.iter()
                .find(|m| m.destination_field == dest_field.name) {
                
                let expr = Self::create_mapping_expression(mapping)?;
                exprs.push(expr.alias(&dest_field.name));
            } else {
                // If no mapping found, try to find a column with the same name
                exprs.push(col(&dest_field.name));
            }
        }

        Ok(df.select(exprs))
    }

    fn create_aggregation_expression(mapping: &Mapping) -> Result<Expr> {
        if mapping.source_fields.is_empty() {
            return Err(anyhow::anyhow!("Mapping has no source fields"));
        }

        match &mapping.policy {
            MergePolicy::FirstMatch { priority: _ } => {
                // For aggregation, use first() to get one value per group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).first())
            },
            MergePolicy::Sum => {
                // Sum the values across the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).sum())
            },
            MergePolicy::Count => {
                // Count non-null values in the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).count())
            },
            MergePolicy::Average => {
                // Average the values across the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).mean())
            },
            MergePolicy::Min => {
                // Minimum value in the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).min())
            },
            MergePolicy::Max => {
                // Maximum value in the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).max())
            },
        }
    }

    fn create_mapping_expression(mapping: &Mapping) -> Result<Expr> {
        if mapping.source_fields.is_empty() {
            return Err(anyhow::anyhow!("Mapping has no source fields"));
        }

        match &mapping.policy {
            MergePolicy::FirstMatch { priority: _ } => {
                // For FirstMatch, use the first available column
                // This handles the case where join keys might not be available after join
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col))
            },
            MergePolicy::Sum => {
                // For sum, we'll fold over the columns
                let mut expr = lit(0);
                for sf in &mapping.source_fields {
                    expr = expr + col(&sf.column_name);
                }
                Ok(expr)
            },
            MergePolicy::Count => {
                // Count non-null values across source fields
                let mut expr = lit(0);
                for sf in &mapping.source_fields {
                    expr = expr + col(&sf.column_name).is_not_null().cast(DataType::Int32);
                }
                Ok(expr)
            },
            MergePolicy::Average => {
                // Calculate average manually
                let mut sum_expr = lit(0.0);
                let count = mapping.source_fields.len() as f64;
                for sf in &mapping.source_fields {
                    sum_expr = sum_expr + col(&sf.column_name).cast(DataType::Float64);
                }
                Ok(sum_expr / lit(count))
            },
            MergePolicy::Min => {
                // Use coalesce-like approach for minimum across columns
                let cols: Vec<Expr> = mapping.source_fields.iter()
                    .map(|sf| col(&sf.column_name))
                    .collect();
                
                if cols.len() == 1 {
                    Ok(cols[0].clone())
                } else {
                    // For now, use the first non-null value as a placeholder
                    // TODO: Implement proper minimum across columns
                    Ok(coalesce(&cols))
                }
            },
            MergePolicy::Max => {
                // Use coalesce-like approach for maximum across columns
                let cols: Vec<Expr> = mapping.source_fields.iter()
                    .map(|sf| col(&sf.column_name))
                    .collect();
                
                if cols.len() == 1 {
                    Ok(cols[0].clone())
                } else {
                    // For now, use the first non-null value as a placeholder
                    // TODO: Implement proper maximum across columns
                    Ok(coalesce(&cols))
                }
            },
        }
    }
}