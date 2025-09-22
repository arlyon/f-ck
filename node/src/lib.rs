use f_ck::{DataReader, DataWriter, JoinEngine, Query, QueryPlan, Source};
use wasm_bindgen::prelude::*;

// Import the `console.log` function from the browser's Web API
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// store the sources within the wasm boundary so they do not need to be re-loaded
#[wasm_bindgen]
pub struct SourceHandle {
    source: Source,
}

#[wasm_bindgen]
impl SourceHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(source: Source) -> Self {
        Self { source }
    }
    pub fn id(&self) -> String {
        self.source.id().to_owned()
    }
    pub fn format(&self) -> String {
        self.source.format().to_owned()
    }

    pub fn schema(&mut self) -> JsValue {
        let mut df = DataReader::read_source(&self.source).unwrap();

        serde_wasm_bindgen::to_value(
            &polars_jsonschema_bridge::polars_schema_to_json_schema(
                df.collect_schema().unwrap().as_ref(),
                &polars_jsonschema_bridge::JsonSchemaOptions::new(),
            )
            .unwrap(),
        )
        .unwrap()
    }
}

// Define a macro to provide a `println!`-style syntax for logging
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    tracing_wasm::set_as_global_default();

    Ok(())
}

#[wasm_bindgen]
pub fn execute_query_json(source: Vec<SourceHandle>, query: Query) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    let query_plan = QueryPlan {
        sources: source.into_iter().map(|s| s.source.clone()).collect(),
        query,
    };

    let result = JoinEngine::execute_query(&query_plan)
        .map_err(|e| JsValue::from_str(&format!("Query execution failed: {}", e)))?;

    console_log!("Query executed successfully");

    let csv_output = DataWriter::preview_data(result, None)
        .map_err(|e| JsValue::from_str(&format!("Failed to format result: {}", e)))?;

    Ok(csv_output)
}
