//! `/v1/context` 端点：tokitai-context 0.2 facade
//!
//! 由于 `tokitai_context::Context` 内部包含非 Send 的 `Box<dyn FileContextService>`，
//! handler 不直接持锁；而是进入 `spawn_blocking` 中开启一个短生命周期的
//! `Context`、完成操作、返回结果。
//!
//! 因此本模块只缓存 `Path`（Send），handler 全部走 `spawn_blocking`。

use axum::extract::{Path as AxumPath, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use parking_lot::Mutex as PlMutex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokitai_context::{Context, Layer};

use crate::server::error::ApiError;
use crate::server::state::AppState;

/// 共享的 Context root 路径；handler 自行 `Context::open` 使用
#[derive(Default)]
pub struct ContextState {
    pub root: Option<PathBuf>,
}

impl ContextState {
    pub fn open(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }
}

/// 启动时构造的共享 root 路径
pub fn build_default_context() -> Arc<PlMutex<ContextState>> {
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    Arc::new(PlMutex::new(ContextState::open(
        root.join(".tokitai/context"),
    )))
}

fn root_path(state: &AppState) -> Result<PathBuf, ApiError> {
    state
        .context
        .lock()
        .root
        .clone()
        .ok_or_else(|| ApiError::Conflict("Context root 未配置".to_string()))
}

#[derive(Deserialize)]
pub struct StoreReq {
    pub session: String,
    pub content_b64: String,
    #[serde(default)]
    pub layer: Option<String>,
}

#[derive(Deserialize)]
pub struct RetrieveQuery {
    pub session: String,
    pub hash: String,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub session: String,
    pub q: String,
}

fn parse_layer(name: Option<&str>) -> Layer {
    match name.unwrap_or("short_term") {
        "transient" => Layer::Transient,
        "long_term" => Layer::LongTerm,
        _ => Layer::ShortTerm,
    }
}

async fn store(
    State(state): State<AppState>,
    Json(req): Json<StoreReq>,
) -> Result<Json<Value>, ApiError> {
    let root = root_path(&state)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(req.content_b64.as_bytes())
        .map_err(|e| ApiError::BadRequest(format!("base64 解码失败：{}", e)))?;
    let layer = parse_layer(req.layer.as_deref());
    let session = req.session.clone();
    let hash = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let mut ctx = Context::open(&root)?;
        Ok(ctx.store(&session, &bytes, layer)?)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(|e| ApiError::Internal(format!("{:?}", e)))?;
    Ok(Json(json!({"hash": hash})))
}

async fn retrieve(
    State(state): State<AppState>,
    Query(q): Query<RetrieveQuery>,
) -> Result<Json<Value>, ApiError> {
    let root = root_path(&state)?;
    let session = q.session.clone();
    let hash = q.hash.clone();
    let item = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let ctx = Context::open(&root)?;
        Ok(ctx.retrieve(&session, &hash)?)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(|e| ApiError::Internal(format!("{:?}", e)))?;
    let payload = base64::engine::general_purpose::STANDARD.encode(&item.content);
    Ok(Json(json!({
        "hash": item.hash,
        "size": item.content.len(),
        "summary": item.summary,
        "content_b64": payload,
    })))
}

async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let root = root_path(&state)?;
    let session = q.session.clone();
    let query = q.q.clone();
    let hits = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let ctx = Context::open(&root)?;
        Ok(ctx.search(&session, &query)?)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(|e| ApiError::Internal(format!("{:?}", e)))?;
    let payload: Vec<Value> = hits
        .into_iter()
        .map(|h| {
            json!({
                "hash": h.hash,
                "score": h.score,
                "summary": h.summary,
            })
        })
        .collect();
    Ok(Json(json!({"results": payload})))
}

async fn stats(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let root = root_path(&state)?;
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<Value> {
        let ctx = Context::open(&root)?;
        // ContextStats 不实现 Serialize，转为字段级 JSON
        let s = ctx.stats();
        Ok(json!({
            "sessions_count": s.sessions_count,
            "items_count": s.items_count,
            "total_size_bytes": s.total_size_bytes,
            "cache_hit_rate": s.cache_hit_rate,
            "filekv_memtable_size": s.filekv_memtable_size,
            "filekv_memtable_entries": s.filekv_memtable_entries,
            "filekv_segment_count": s.filekv_segment_count,
            "filekv_total_entries": s.filekv_total_entries,
            "filekv_write_count": s.filekv_write_count,
            "filekv_read_count": s.filekv_read_count,
            "filekv_flush_count": s.filekv_flush_count,
            "filekv_compaction_runs": s.filekv_compaction_runs,
            "filekv_bloom_filtered": s.filekv_bloom_filtered,
            "filekv_memory_usage_bytes": s.filekv_memory_usage_bytes,
            "filekv_stats_warning": s.filekv_stats_warning,
        }))
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(|e| ApiError::Internal(format!("{:?}", e)))?;
    Ok(Json(json!({"stats": value})))
}

async fn checkpoint(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    let root = root_path(&state)?;
    let id_str = id.clone();
    let restored = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<usize>> {
        let ctx = Context::open(&root)?;
        Ok(ctx.restore_checkpoint(&id_str)?)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(|e| ApiError::Internal(format!("{:?}", e)))?;
    Ok(Json(json!({"restored": restored})))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/context/store", post(store))
        .route("/context/retrieve", get(retrieve))
        .route("/context/search", get(search))
        .route("/context/stats", get(stats))
        .route("/context/checkpoints/:id/restore", post(checkpoint))
}
