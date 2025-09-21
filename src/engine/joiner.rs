use crate::dsl::{Mapping, MergePolicy, QueryPlan};
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

    fn simple_join(query: &QueryPlan, dataframes: HashMap<String, LazyFrame>) -> Result<LazyFrame> {
        if dataframes.is_empty() {
            return Err(anyhow::anyhow!("No dataframes to join"));
        }

        // Get the primary key mapping to determine join columns
        let primary_key = &query.primary_keys.keys[0];
        let pk_mapping = query
            .mappings
            .iter()
            .find(|m| m.destination_field == *primary_key)
            .ok_or_else(|| anyhow::anyhow!("No mapping found for primary key: {}", primary_key))?;

        // Create ordered join preserving input CSV order and source priority
        let result = Self::ordered_priority_join(query, dataframes, pk_mapping)?;

        // Apply field mappings and create destination schema with aggregation
        Self::apply_mappings_with_aggregation(result, query)
    }

    fn ordered_priority_join(
        query: &QueryPlan,
        mut dataframes: HashMap<String, LazyFrame>,
        pk_mapping: &Mapping,
    ) -> Result<LazyFrame> {
        // Step 1: Start with the primary source (first in sources list)
        let primary_source_id = &query.sources[0].id;
        let mut result = dataframes
            .remove(primary_source_id)
            .ok_or_else(|| anyhow::anyhow!("Primary source not found: {}", primary_source_id))?;

        // Add row index to preserve original order for primary source
        result = result.with_row_index("__primary_order", None);

        // Step 2: Process remaining sources in order with simple left joins for now
        // This preserves the primary source order and priority
        for source in query.sources.iter().skip(1) {
            if let Some(right_df) = dataframes.remove(&source.id) {
                result = Self::simple_priority_join(
                    result,
                    right_df,
                    pk_mapping,
                    primary_source_id,
                    &source.id,
                )?;
            }
        }

        // Step 3: Sort by original order (keep all columns for now)
        result = result.sort(["__primary_order"], SortMultipleOptions::default());

        Ok(result)
    }

    fn get_join_column_for_source<'a>(pk_mapping: &'a Mapping, source_id: &str) -> Result<&'a str> {
        pk_mapping
            .source_fields
            .iter()
            .find(|sf| sf.source_file_id == source_id)
            .map(|sf| sf.column_name.as_str())
            .ok_or_else(|| anyhow::anyhow!("No join column found for source: {}", source_id))
    }

    fn simple_priority_join(
        left: LazyFrame,
        right: LazyFrame,
        pk_mapping: &Mapping,
        primary_source_id: &str,
        right_source_id: &str,
    ) -> Result<LazyFrame> {
        // Get join columns
        let left_join_col = Self::get_join_column_for_source(pk_mapping, primary_source_id)?;
        let right_join_col = Self::get_join_column_for_source(pk_mapping, right_source_id)?;

        // Perform left join to preserve primary source order
        let result = left.join(
            right,
            [col(left_join_col)],
            [col(right_join_col)],
            JoinArgs::new(JoinType::Left),
        );

        Ok(result)
    }

    fn apply_mappings_with_aggregation(df: LazyFrame, query: &QueryPlan) -> Result<LazyFrame> {
        // First, determine if we need aggregation by checking if any mapping uses Sum, Count, Average
        let needs_aggregation = query.mappings.iter().any(|mapping| {
            matches!(
                mapping.policy,
                MergePolicy::Sum | MergePolicy::Count | MergePolicy::Average
            )
        });

        if needs_aggregation {
            // Find the primary key columns for grouping
            let primary_key = &query.primary_keys.keys[0];
            let pk_mapping = query
                .mappings
                .iter()
                .find(|m| m.destination_field == *primary_key)
                .ok_or_else(|| {
                    anyhow::anyhow!("No mapping found for primary key: {}", primary_key)
                })?;

            // Use the left table's column for grouping (from the join)
            let group_col = &pk_mapping.source_fields[0].column_name;

            // Create aggregation expressions
            let mut agg_exprs = Vec::new();

            // Include the primary order to maintain ordering after aggregation
            agg_exprs.push(col("__primary_order").first().alias("__primary_order"));

            for dest_field in &query.destination_schema {
                if let Some(mapping) = query
                    .mappings
                    .iter()
                    .find(|m| m.destination_field == dest_field.name)
                {
                    let expr = Self::create_aggregation_expression(mapping)?;
                    agg_exprs.push(expr.alias(&dest_field.name));
                } else {
                    // For non-mapped fields, use first() to get one value per group
                    agg_exprs.push(col(&dest_field.name).first().alias(&dest_field.name));
                }
            }

            // Group by and aggregate, then sort by the preserved primary order
            let result = df.group_by_stable([col(group_col)]).agg(agg_exprs);

            let destination_columns: Vec<_> = query
                .destination_schema
                .iter()
                .map(|field| col(&field.name))
                .collect();

            Ok(result.select(destination_columns))
        } else {
            // No aggregation needed, just apply regular mappings
            Self::apply_simple_mappings(df, query)
        }
    }

    fn apply_simple_mappings(df: LazyFrame, query: &QueryPlan) -> Result<LazyFrame> {
        let mut exprs = Vec::new();

        // Create expressions for each destination field based on mappings
        for dest_field in &query.destination_schema {
            if let Some(mapping) = query
                .mappings
                .iter()
                .find(|m| m.destination_field == dest_field.name)
            {
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
            }
            MergePolicy::Sum => {
                // Sum the values across the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).sum())
            }
            MergePolicy::Count => {
                // Count non-null values in the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).count())
            }
            MergePolicy::Average => {
                // Average the values across the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).mean())
            }
            MergePolicy::Min => {
                // Minimum value in the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).min())
            }
            MergePolicy::Max => {
                // Maximum value in the group
                let first_col = &mapping.source_fields[0].column_name;
                Ok(col(first_col).max())
            }
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
            }
            MergePolicy::Sum => {
                // For sum, we'll fold over the columns
                let mut expr = lit(0);
                for sf in &mapping.source_fields {
                    expr = expr + col(&sf.column_name);
                }
                Ok(expr)
            }
            MergePolicy::Count => {
                // Count non-null values across source fields
                let mut expr = lit(0);
                for sf in &mapping.source_fields {
                    expr = expr + col(&sf.column_name).is_not_null().cast(DataType::Int32);
                }
                Ok(expr)
            }
            MergePolicy::Average => {
                // Calculate average manually
                let mut sum_expr = lit(0.0);
                let count = mapping.source_fields.len() as f64;
                for sf in &mapping.source_fields {
                    sum_expr = sum_expr + col(&sf.column_name).cast(DataType::Float64);
                }
                Ok(sum_expr / lit(count))
            }
            MergePolicy::Min => {
                // Use coalesce-like approach for minimum across columns
                let cols: Vec<Expr> = mapping
                    .source_fields
                    .iter()
                    .map(|sf| col(&sf.column_name))
                    .collect();

                if cols.len() == 1 {
                    Ok(cols[0].clone())
                } else {
                    // For now, use the first non-null value as a placeholder
                    // TODO: Implement proper minimum across columns
                    Ok(coalesce(&cols))
                }
            }
            MergePolicy::Max => {
                // Use coalesce-like approach for maximum across columns
                let cols: Vec<Expr> = mapping
                    .source_fields
                    .iter()
                    .map(|sf| col(&sf.column_name))
                    .collect();

                if cols.len() == 1 {
                    Ok(cols[0].clone())
                } else {
                    // For now, use the first non-null value as a placeholder
                    // TODO: Implement proper maximum across columns
                    Ok(coalesce(&cols))
                }
            }
        }
    }
}
