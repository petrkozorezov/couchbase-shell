use crate::cli::error::{client_error_to_shell_error, unexpected_status_code_error};
use crate::cli::generic_error;
use crate::cli::util::namespace_from_args;
use crate::cli::util::{cluster_identifiers_from, get_active_cluster, NuValueMap};
use crate::client::VectorSearchQueryRequest;
use crate::state::State;
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use serde_derive::Deserialize;
use serde_json::json;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct VectorSearch {
    state: Arc<Mutex<State>>,
}

impl VectorSearch {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for VectorSearch {
    fn name(&self) -> &str {
        "vector search"
    }

    fn signature(&self) -> Signature {
        Signature::build("vector search")
            .optional(
                "vector",
                SyntaxShape::List(Box::new(SyntaxShape::Float)),
                "the vector used for searching",
            )
            .required("index", SyntaxShape::String, "the index name")
            .required(
                "field",
                SyntaxShape::String,
                "name of the vector field the index was built on",
            )
            .named(
                "query",
                SyntaxShape::String,
                "the text to query for using a query string query",
                None,
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .named(
                "neighbors",
                SyntaxShape::Int,
                "number of neighbors returned by vector search (default = 3)",
                None,
            )
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Performs a vector search query"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(self.state.clone(), engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
            description: "Source vector fetched using 'doc get'",
            example: "doc get 10019 | flatten | select contentVector  | vector search landmark-content-index contentVector",
            result: None,
        },
        Example {
            description: "Source vector fetched using 'subdoc get'",
            example: "subdoc get contentVector 10019 | select content | vector search landmark-content-index contentVector",
            result: None,
        },
        Example{
             description: "Plain source vector as positional parameter",
             example: "vector search vector-index fieldName [0.1 0.2 0.3 0.4]",
             result: None,
         },
        ]
    }
}

fn run(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let mut vector: Vec<f32> = vec![];
    match input.into_value(span)? {
        Value::List { vals, .. } => {
            let rec = match vals[0].as_record() {
                Ok(r) => r,
                Err(_) => {
                    return Err(failed_to_parse_input_vector_error());
                }
            };

            if rec.contains("id") && rec.contains("content") {
                // Input is from vector enrich-text
                let id = rec.get("id").unwrap().as_str().unwrap();
                if id.len() > 6 && id[..6] == *"vector" {
                    let content = rec.get("content").unwrap().as_record().unwrap();
                    vector = content
                        .get("vector")
                        .unwrap()
                        .as_list()
                        .unwrap()
                        .iter()
                        .map(|e| e.as_float().unwrap() as f32)
                        .collect();
                }
            } else {
                // Input is vector from doc get or query
                let list = match rec.get_index(0).unwrap().1.as_list() {
                    Ok(l) => l,
                    Err(_) => {
                        return Err(failed_to_parse_input_vector_error());
                    }
                };
                vector = list.iter().map(|e| e.as_float().unwrap() as f32).collect();
            }
        }
        Value::Nothing { internal_span: _ } => {
            let vec: Option<Value> = call.opt(engine_state, stack, 2)?;
            if let Some(v) = vec {
                vector = v
                    .as_list()
                    .unwrap()
                    .iter()
                    .map(|e| e.as_float().unwrap() as f32)
                    .collect();
            } else {
                return Err(failed_to_parse_input_vector_error());
            }
        }
        _ => {
            return Err(failed_to_parse_input_vector_error());
        }
    }

    let index: String = call.req(engine_state, stack, 0)?;
    let field: String = call.req(engine_state, stack, 1)?;

    let query: serde_json::Value = match call.get_flag::<String>(engine_state, stack, "query")? {
        Some(q) => json!({ "query": q }),
        None => {
            json!({"match_none": {}})
        }
    };

    let neighbors = call
        .get_flag(engine_state, stack, "neighbors")?
        .unwrap_or(3);

    let bucket_flag: Option<String> = call.get_flag(engine_state, stack, "bucket")?;
    let scope_flag: Option<String> = call.get_flag(engine_state, stack, "scope")?;

    debug!("Running vector search query {} against {}", &query, &index);

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let namespace = namespace_from_args(
            bucket_flag.clone(),
            scope_flag.clone(),
            None,
            active_cluster,
            span,
        )?;

        let qualified_index = index_name_from_namespace(index.clone(), namespace);
        let response = active_cluster
            .cluster()
            .http_client()
            .search_query_request(
                VectorSearchQueryRequest::Execute {
                    query: query.clone(),
                    index: qualified_index.clone(),
                    vector: vector.clone(),
                    field: field.clone(),
                    neighbors,
                    timeout: active_cluster.timeouts().search_timeout().as_millis(),
                },
                Instant::now().add(active_cluster.timeouts().search_timeout()),
                ctrl_c.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        let rows: SearchResultData = match response.status() {
            200 => serde_json::from_str(response.content()).map_err(|_e| {
                unexpected_status_code_error(response.status(), response.content(), span)
            })?,
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
                ));
            }
        };

        for row in rows.hits {
            let mut collected = NuValueMap::default();
            collected.add_string("id", row.id, span);
            collected.add_string("score", format!("{}", row.score), span);
            collected.add_string("cluster", identifier.clone(), span);

            results.push(collected.into_value(span));
        }
    }

    Ok(Value::List {
        vals: results,
        internal_span: call.head,
    }
    .into_pipeline_data())
}

fn index_name_from_namespace(index: String, namespace: (String, String, String)) -> String {
    let scope = if namespace.1.is_empty() {
        "_default".to_string()
    } else {
        namespace.1
    };
    format!("{}.{}.{}", namespace.0, scope, index)
}

#[derive(Debug, Deserialize)]
struct SearchResultHit {
    score: f32,
    id: String,
}

#[derive(Debug, Deserialize)]
struct SearchResultData {
    hits: Vec<SearchResultHit>,
}

fn failed_to_parse_input_vector_error() -> ShellError {
    generic_error(
        "Could not parse input vector",
        "Piped input must be correctly formatted, run 'vector search --help' for examples"
            .to_string(),
        None,
    )
}
