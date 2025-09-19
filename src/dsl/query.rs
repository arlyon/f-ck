use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryPlan {
    pub sources: Vec<Source>,
    pub destination_schema: Vec<DestinationField>,
    pub primary_keys: PrimaryKeySpec,
    pub mappings: Vec<Mapping>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Source {
    pub id: String,
    pub path: PathBuf,
    pub format: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DestinationField {
    pub name: String,
    pub data_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PrimaryKeyLogic { 
    Or, 
    And 
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrimaryKeySpec {
    pub logic: PrimaryKeyLogic,
    pub keys: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Mapping {
    pub destination_field: String,
    pub policy: MergePolicy,
    pub source_fields: Vec<SourceFieldSpec>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SourceFieldSpec {
    pub id: String,
    pub source_file_id: String,
    pub column_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum MergePolicy {
    FirstMatch { priority: Vec<String> },
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

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.sources.is_empty() {
            return Err(anyhow::anyhow!("QueryPlan must have at least one data source"));
        }

        if self.mappings.is_empty() {
            return Err(anyhow::anyhow!("QueryPlan must have at least one field mapping"));
        }

        if self.primary_keys.keys.is_empty() {
            return Err(anyhow::anyhow!("QueryPlan must have at least one primary key"));
        }

        // Validate that source IDs referenced in mappings exist
        let source_ids: std::collections::HashSet<_> = 
            self.sources.iter().map(|s| &s.id).collect();
        
        for mapping in &self.mappings {
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
            self.destination_schema.iter().map(|f| &f.name).collect();
        
        for key in &self.primary_keys.keys {
            if !dest_field_names.contains(key) {
                return Err(anyhow::anyhow!(
                    "Primary key references unknown destination field: {}", 
                    key
                ));
            }
        }

        Ok(())
    }
}