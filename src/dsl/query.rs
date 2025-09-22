use polars::series::IsSorted;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct QueryPlan {
    pub sources: Vec<Source>,
    pub query: Query,
}

impl QueryPlan {
    // get the sort order of the sources from the primary keys
    // - source id
    // - column name in that source
    // - sort order
    pub fn indexes(&self) -> Vec<(&str, &str, IsSorted)> {
        match &self.query {
            Query::Dsl(query) => {
                let DslQuery {
                    primary_keys,
                    mappings,
                    ..
                } = query;

                // get the pk then all the source fields
                let pk = primary_keys.keys.first().unwrap();

                mappings
                    .iter()
                    .find(|m| m.destination_field == *pk)
                    .iter()
                    .flat_map(|f| f.source_fields.iter())
                    .map(|sf| {
                        (
                            sf.source_file_id.as_str(),
                            sf.column_name.as_str(),
                            IsSorted::Ascending,
                        )
                    })
                    .collect()
            }
            Query::Sql { .. } => Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Query {
    Dsl(DslQuery),
    Sql { sql: String },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct DslQuery {
    pub destination_schema: Vec<DestinationField>,
    pub primary_keys: PrimaryKeySpec,
    pub mappings: Vec<Mapping>,
}

impl FromStr for Query {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try to parse as JSON first (DSL variant)
        if let Ok(dsl_query) = serde_json::from_str::<DslQuery>(s) {
            return Ok(Query::Dsl(dsl_query));
        }

        // If not valid JSON or doesn't parse as DSL, treat as SQL
        Ok(Query::Sql { sql: s.to_string() })
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Source {
    pub id: String,
    pub path: PathBuf,
    pub format: String,
}

impl FromStr for Source {
    type Err = anyhow::Error;

    // id:path. format is derived from the path ext
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (id, path) = s
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("Source format must be 'id:path'"))?;
        let path: PathBuf = path.into();
        let ext = path
            .extension()
            .ok_or_else(|| anyhow::anyhow!("Path must have an extension"))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid extension"))?;
        Ok(Self {
            id: id.to_owned(),
            format: ext.to_owned(),
            path,
        })
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct DestinationField {
    pub name: String,
    pub data_type: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PrimaryKeyLogic {
    Or,
    And,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct PrimaryKeySpec {
    pub keys: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Mapping {
    pub destination_field: String,
    pub policy: MergePolicy,
    pub source_fields: Vec<SourceFieldSpec>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct SourceFieldSpec {
    pub source_file_id: String,
    pub column_name: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum MergePolicy {
    FirstMatch {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        priority: Vec<String>,
    },
    Sum,
    Count,
    Average,
    Min,
    Max,
}

impl QueryPlan {
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Generate JSON schema for QueryPlan validation
    pub fn json_schema() -> anyhow::Result<String> {
        let schema = schemars::schema_for!(QueryPlan);
        Ok(serde_json::to_string_pretty(&schema)?)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.sources.is_empty() {
            return Err(anyhow::anyhow!(
                "QueryPlan must have at least one data source"
            ));
        }

        match &self.query {
            Query::Dsl(dsl_query) => {
                let DslQuery {
                    destination_schema,
                    primary_keys,
                    mappings,
                } = dsl_query;
                if mappings.is_empty() {
                    return Err(anyhow::anyhow!(
                        "DSL QueryPlan must have at least one field mapping"
                    ));
                }

                if primary_keys.keys.is_empty() {
                    return Err(anyhow::anyhow!(
                        "DSL QueryPlan must have at least one primary key"
                    ));
                }

                // Validate that source IDs referenced in mappings exist
                let source_ids: std::collections::HashSet<_> =
                    self.sources.iter().map(|s| &s.id).collect();

                for mapping in mappings {
                    for source_field in &mapping.source_fields {
                        if !source_ids.contains(&source_field.source_file_id) {
                            return Err(anyhow::anyhow!(
                                "Mapping references unknown source: {}",
                                source_field.source_file_id
                            ));
                        }
                    }
                }

                // Validate that destination fields referenced in primary keys exist
                let dest_field_names: std::collections::HashSet<_> =
                    destination_schema.iter().map(|f| &f.name).collect();

                for key in &primary_keys.keys {
                    if !dest_field_names.contains(key) {
                        return Err(anyhow::anyhow!(
                            "Primary key references unknown destination field: {}",
                            key
                        ));
                    }
                }
            }
            Query::Sql { sql } => {
                if sql.trim().is_empty() {
                    return Err(anyhow::anyhow!("SQL query cannot be empty"));
                }
            }
        }

        Ok(())
    }
}
