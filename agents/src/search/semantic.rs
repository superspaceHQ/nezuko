use thiserror::Error;
use tracing::log::debug;
// import hashset from collections
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env,
    path::Path,
    str,
};
// import anyhow from anyhow
use anyhow::Result;

use crate::parser::parser::Literal;
use crate::search::payload::{Embedding, Payload, SymbolPayload};
use std::sync::Arc;

pub(crate) const EMBEDDING_DIM: usize = 384;

use ndarray::Axis;
use ort::tensor::OrtOwnedTensor;
use ort::value::Value;
use ort::{Environment, ExecutionProvider, GraphOptimizationLevel, LoggingLevel, SessionBuilder};
use qdrant_client::{
    prelude::{QdrantClient, QdrantClientConfig},
    qdrant::{
        point_id::PointIdOptions, r#match::MatchValue, vectors::VectorsOptions, vectors_config,
        with_payload_selector, with_vectors_selector, CollectionOperationResponse, Condition,
        CreateCollection, Distance, FieldCondition, FieldType, Filter, Match, PointId,
        RetrievedPoint, ScoredPoint, SearchPoints, VectorParams, Vectors, VectorsConfig,
        WithPayloadSelector, WithVectorsSelector,
    },
};

use crate::Configuration;

pub struct Semantic {
    pub qdrant_collection_name: String,
    pub repo_name: String,
    pub qdrant: QdrantClient,
    pub tokenizer: tokenizers::Tokenizer,
    pub session: ort::Session,
}

#[derive(Error, Debug)]
pub enum SemanticError {
    /// Represents failure to initialize Qdrant client
    #[error("Qdrant initialization failed. Is Qdrant running on `qdrant-url`?")]
    QdrantInitializationError,

    #[error("ONNX runtime error")]
    OnnxRuntimeError {
        #[from]
        error: ort::OrtError,
    },

    #[error("semantic error")]
    Anyhow {
        #[from]
        error: anyhow::Error,
    },
}

impl Semantic {
    pub async fn initialize(config: Configuration) -> Result<Self, SemanticError> {
        // let qdrant = QdrantClient::new(Some(QdrantClientConfig::from_url(&config.semantic_url)))?;
        let qdrant_api_key = "yfxX63AauMGbXoGVSveAjq373wEOTASLLmHfTvMiOZKJtyYFKq9wHg";
        let qdrant_url = "https://81e9d930-b73c-4870-914b-2c8b6c5a3b9a.ap-southeast-1-0.aws.cloud.qdrant.io:6334";
        let qdrant = QdrantClient::from_url(qdrant_url)
            // using an env variable for the API KEY, for example
            .with_api_key(qdrant_api_key)
            .build()?;

        let environment = Arc::new(
            Environment::builder()
                .with_name("Encode")
                .with_log_level(LoggingLevel::Warning)
                .with_execution_providers([ExecutionProvider::CPU(Default::default())])
                .with_telemetry(false)
                .build()?,
        );

        let threads = if let Ok(v) = std::env::var("NUM_OMP_THREADS") {
            str::parse(&v).unwrap_or(1)
        } else {
            1
        };

        Ok(Self {
            qdrant: qdrant.into(),
            tokenizer: tokenizers::Tokenizer::from_file(config.tokenizer_path.as_str())
                .unwrap()
                .into(),
            session: SessionBuilder::new(&environment)?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(threads)?
                .with_model_from_file(config.model_path)?
                .into(),
            qdrant_collection_name: config.semantic_collection_name,
            repo_name: config.repo_name,
        })
    }

    pub fn embed(&self, sequence: &str) -> anyhow::Result<Embedding> {
        let tokenizer_output = self.tokenizer.encode(sequence, true).unwrap();

        let input_ids = tokenizer_output.get_ids();
        let attention_mask = tokenizer_output.get_attention_mask();
        let token_type_ids = tokenizer_output.get_type_ids();
        let length = input_ids.len();
        println!("embedding {} tokens {:?}", length, sequence);

        let inputs_ids_array = ndarray::Array::from_shape_vec(
            (1, length),
            input_ids.iter().map(|&x| x as i64).collect(),
        )?;

        let attention_mask_array = ndarray::Array::from_shape_vec(
            (1, length),
            attention_mask.iter().map(|&x| x as i64).collect(),
        )?;

        let token_type_ids_array = ndarray::Array::from_shape_vec(
            (1, length),
            token_type_ids.iter().map(|&x| x as i64).collect(),
        )?;

        let outputs = self.session.run(vec![
            Value::from_array(
                self.session.allocator(),
                &ndarray::CowArray::from(inputs_ids_array).into_dyn(),
            )
            .unwrap(),
            Value::from_array(
                self.session.allocator(),
                &ndarray::CowArray::from(attention_mask_array).into_dyn(),
            )
            .unwrap(),
            Value::from_array(
                self.session.allocator(),
                &ndarray::CowArray::from(token_type_ids_array).into_dyn(),
            )
            .unwrap(),
        ])?;

        let output_tensor: OrtOwnedTensor<f32, _> = outputs[0].try_extract().unwrap();
        let sequence_embedding = &*output_tensor.view();
        let pooled = sequence_embedding.mean_axis(Axis(1)).unwrap();
        Ok(pooled.to_owned().as_slice().unwrap().to_vec())
    }
}

// Exact match filter
pub(crate) fn make_kv_keyword_filter(key: &str, value: &str) -> FieldCondition {
    let key = key.to_owned();
    let value = value.to_owned();
    FieldCondition {
        key,
        r#match: Some(Match {
            match_value: MatchValue::Keyword(value).into(),
        }),
        ..Default::default()
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SemanticQuery<'a> {
    pub paths: HashSet<Literal<'a>>,
    pub langs: HashSet<Cow<'a, str>>,

    pub target: Option<Literal<'a>>,
}

impl<'a> SemanticQuery<'a> {
    pub fn paths(&'a self) -> impl Iterator<Item = Cow<'a, str>> {
        self.paths.iter().filter_map(|t| t.as_plain())
    }

    pub fn langs(&'a self) -> impl Iterator<Item = Cow<'a, str>> {
        self.langs.iter().cloned()
    }

    pub fn target(&self) -> Option<Cow<'a, str>> {
        self.target.as_ref().and_then(|t| t.as_plain())
    }

    pub fn into_owned(self) -> SemanticQuery<'static> {
        SemanticQuery {
            paths: self.paths.into_iter().map(Literal::into_owned).collect(),
            langs: self
                .langs
                .into_iter()
                .map(|c| c.into_owned().into())
                .collect(),

            target: self.target.map(Literal::into_owned),
        }
    }
}

pub fn deduplicate_snippets(
    mut all_snippets: Vec<Payload>,
    query_embedding: Embedding,
    output_count: u64,
) -> Vec<Payload> {
    all_snippets = filter_overlapping_snippets(all_snippets);

    let idxs = {
        let lambda = 0.5;
        let k = output_count; // number of snippets
        let embeddings = all_snippets
            .iter()
            .map(|s| s.embedding.as_deref().unwrap())
            .collect::<Vec<_>>();
        let languages = all_snippets
            .iter()
            .map(|s| s.lang.as_ref())
            .collect::<Vec<_>>();
        let paths = all_snippets
            .iter()
            .map(|s| s.relative_path.as_ref())
            .collect::<Vec<_>>();
        deduplicate_with_mmr(
            &query_embedding,
            &embeddings,
            &languages,
            &paths,
            lambda,
            k as usize,
        )
    };

    println!("preserved idxs after MMR are {:?}", idxs);

    all_snippets
        .drain(..)
        .enumerate()
        .filter_map(|(ref i, payload)| {
            if idxs.contains(i) {
                Some(payload)
            } else {
                None
            }
        })
        .collect()
}

fn filter_overlapping_snippets(mut snippets: Vec<Payload>) -> Vec<Payload> {
    snippets.sort_by(|a, b| {
        a.relative_path
            .cmp(&b.relative_path)
            .then(a.start_line.cmp(&b.start_line))
    });

    snippets = snippets
        .into_iter()
        .fold(Vec::<Payload>::new(), |mut deduped_snippets, snippet| {
            if let Some(prev) = deduped_snippets.last_mut() {
                if prev.relative_path == snippet.relative_path
                    && prev.end_line >= snippet.start_line
                {
                    debug!(
                        "Filtering overlapping snippets. End: {:?} - Start: {:?} from {:?}",
                        prev.end_line, snippet.start_line, prev.relative_path
                    );
                    return deduped_snippets;
                }
            }
            deduped_snippets.push(snippet);
            deduped_snippets
        });

    snippets.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    snippets
}

// returns a list of indices to preserve from `snippets`
//
// query_embedding: the embedding of the query terms
// embeddings: the list of embeddings to select from
// lambda: MMR is a weighted selection of two opposing factors:
//    - relevance to the query
//    - "novelty" or, the measure of how minimal the similarity is
//      to existing documents in the selection
//      The value of lambda skews the weightage in favor of either relevance or novelty.
//    - we add a language diversity factor to the score to encourage a range of langauges in the results
//    - we also add a path diversity factor to the score to encourage a range of paths in the results
//  k: the number of embeddings to select
pub fn deduplicate_with_mmr(
    query_embedding: &[f32],
    embeddings: &[&[f32]],
    languages: &[&str],
    paths: &[&str],
    lambda: f32,
    k: usize,
) -> Vec<usize> {
    let mut idxs = vec![];
    let mut lang_counts = HashMap::new();
    let mut path_counts = HashMap::new();

    if embeddings.len() < k {
        return (0..embeddings.len()).collect();
    }

    while idxs.len() < k {
        let mut best_score = f32::NEG_INFINITY;
        let mut idx_to_add = None;

        for (i, emb) in embeddings.iter().enumerate() {
            if idxs.contains(&i) {
                continue;
            }
            let first_part = cosine_similarity(query_embedding, emb);
            let mut second_part = 0.;
            for j in idxs.iter() {
                let cos_sim = cosine_similarity(emb, embeddings[*j]);
                if cos_sim > second_part {
                    second_part = cos_sim;
                }
            }
            let mut equation_score = lambda * first_part - (1. - lambda) * second_part;

            // MMR + (1/2)^n where n is the number of times a language has been selected
            let lang_count = lang_counts.get(languages[i]).unwrap_or(&0);
            equation_score += 0.5_f32.powi(*lang_count);

            // MMR + (3/4)^n where n is the number of times a path has been selected
            let path_count = path_counts.get(paths[i]).unwrap_or(&0);
            equation_score += 0.75_f32.powi(*path_count);

            if equation_score > best_score {
                best_score = equation_score;
                idx_to_add = Some(i);
            }
        }
        if let Some(i) = idx_to_add {
            idxs.push(i);
            *lang_counts.entry(languages[i]).or_insert(0) += 1;
            *path_counts.entry(paths[i]).or_insert(0) += 1;
        }
    }
    idxs
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(ai, bi)| ai * bi).sum()
}

fn norm(a: &[f32]) -> f32 {
    dot(a, a)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    dot(a, b) / (norm(a) * norm(b))
}
