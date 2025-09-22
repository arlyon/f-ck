use crate::dsl::{DslQuery, Mapping, MergePolicy, Query, QueryPlan};
use crate::engine::DataReader;
use anyhow::Result;
use polars::lazy::frame::LazyFrame;
use polars::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

pub struct JoinEngine;

impl JoinEngine {
    #[tracing::instrument(skip(query))]
    pub fn execute_query(query: &QueryPlan) -> Result<LazyFrame> {
        query.validate()?;

        #[cfg(not(feature = "wasm"))]
        let start = Instant::now();

        let mut dataframes: HashMap<String, LazyFrame> = HashMap::new();
        let indexes = query
            .indexes()
            .iter()
            .map(|(k, v, s)| (*k, (*v, *s)))
            .collect::<HashMap<_, _>>();

        tracing::debug!("Indexes: {:?}", indexes);

        for source in &query.sources {
            let df = DataReader::read_source(source)?;

            dataframes.insert(source.id().to_owned(), df);
        }

        #[cfg(not(feature = "wasm"))]
        let df_loaded = {
            let df_loaded = Instant::now();
            tracing::debug!("Dataframes loaded in {:?}", df_loaded - start);
            df_loaded
        };

        let out = match &query.query {
            Query::Dsl(query) => Self::apply(query, dataframes),
            Query::Sql { sql } => {
                let mut ctx = polars_sql::SQLContext::new();
                for (key, df) in dataframes {
                    ctx.register(&key, df);
                }
                ctx.execute(sql).map_err(|err| err.into())
            }
        }?;

        #[cfg(not(feature = "wasm"))]
        tracing::debug!("Dataframes loaded in {:?}", Instant::now() - df_loaded);

        Ok(out)
    }

    fn apply(query: &DslQuery, dataframes: HashMap<String, LazyFrame>) -> Result<LazyFrame> {
        if dataframes.is_empty() {
            return Err(anyhow::anyhow!("No dataframes to join"));
        }

        // Apply field mappings and create destination schema with aggregation
        Self::apply_mappings_with_aggregation(dataframes, query)
    }

    fn apply_mappings_with_aggregation(
        mut dfs: HashMap<String, LazyFrame>,
        query: &DslQuery,
    ) -> Result<LazyFrame> {
        let DslQuery {
            destination_schema,
            primary_keys,
            mappings,
        } = query;

        // Find the primary key columns for grouping
        let primary_key = &primary_keys.keys[0];
        let pk_mapping = mappings
            .iter()
            .find(|m| m.destination_field == *primary_key)
            .ok_or_else(|| anyhow::anyhow!("No mapping found for primary key: {}", primary_key))?;

        use polars_ops::frame::{JoinCoalesce, JoinType, JoinValidation, MaintainOrderJoin};

        // merge row by row using
        let (df, group_col) = pk_mapping
            .source_fields
            .iter()
            .map(|sf| (sf.column_name.as_str(), sf.source_file_id.as_str()))
            .fold(
                None,
                |acc: Option<(LazyFrame, _)>, (col_name, file_id)| match acc {
                    Some((df, join_col)) => Some((
                        df.join(
                            dfs.remove(file_id).unwrap(),
                            &join_col,
                            &[col(col_name)],
                            JoinArgs {
                                maintain_order: MaintainOrderJoin::LeftRight,
                                how: JoinType::Inner,
                                validation: JoinValidation::ManyToMany,
                                suffix: None,
                                slice: None,
                                nulls_equal: false,
                                coalesce: JoinCoalesce::JoinSpecific,
                            },
                        ),
                        join_col,
                    )),
                    None => Some((dfs.remove(file_id).unwrap(), [col(col_name)])),
                },
            )
            .ok_or_else(|| anyhow::anyhow!("No join column found for source: {}", primary_key))?;

        // Create aggregation expressions
        let mut agg_exprs = Vec::new();

        for dest_field in destination_schema
            .iter()
            .filter(|m| !primary_keys.keys.contains(&m.name))
        {
            if let Some(mapping) = mappings
                .iter()
                .find(|m| m.destination_field == dest_field.name)
            {
                let expr = Self::create_mapping_expression(mapping)?;
                tracing::trace!("created map expr {} for {:?}", expr, mapping);
                agg_exprs.push(expr.alias(&dest_field.name));
            } else {
                // For non-mapped fields, use first() to get one value per group
                agg_exprs.push(col(&dest_field.name).first().alias(&dest_field.name));
            }
        }

        tracing::debug!("agg exprs: {:?}", agg_exprs);

        // Group by and aggregate, then sort by the preserved primary order
        // aggregate
        let df = if agg_exprs.len() > 1 {
            df.group_by_stable(group_col).agg(agg_exprs)
        } else {
            df
        };

        let destination_columns: Vec<_> = destination_schema
            .iter()
            .map(|field| col(&field.name))
            .collect();

        Ok(df.select(destination_columns))
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
                Ok(col(first_col).first())
            }
            MergePolicy::Sum => {
                // For sum, we'll fold over the columns
                let mut expr = lit(0);
                for sf in &mapping.source_fields {
                    expr = expr + col(&sf.column_name);
                }
                Ok(expr.sum())
            }
            MergePolicy::Count => {
                // Count non-null values across source fields
                let mut expr = lit(0);
                for sf in &mapping.source_fields {
                    expr = expr + col(&sf.column_name).is_not_null().cast(DataType::Int32);
                }
                Ok(expr.count())
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
