//! `/v1/providers` 与 `/v1/models` 端点

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::llm::ProviderType;
use crate::server::error::ApiError;
use crate::server::state::AppState;

#[derive(Serialize)]
pub struct ProvidersResp {
    pub current: Option<String>,
    pub providers: Vec<ProviderSummary>,
}

#[derive(Serialize)]
pub struct ProviderSummary {
    pub name: String,
    pub provider_type: String,
    pub default_model: String,
}

#[derive(Deserialize)]
pub struct SwitchProviderReq {
    pub provider: String,
}

#[derive(Serialize)]
pub struct ModelsResp {
    pub current_provider: Option<String>,
    pub models: Vec<ModelSummary>,
}

#[derive(Serialize)]
pub struct ModelSummary {
    pub provider: String,
    pub model: String,
}

async fn list_providers(State(state): State<AppState>) -> Json<ProvidersResp> {
    let llm = state.llm.lock();
    let current = llm.current_provider_type().map(|p| p.as_str().to_string());
    let providers = llm
        .list_providers()
        .into_iter()
        .filter_map(|provider_type| {
            llm.get_provider(provider_type)
                .map(|provider| ProviderSummary {
                    name: provider.name().to_string(),
                    provider_type: provider.provider_type().as_str().to_string(),
                    default_model: provider.default_model().to_string(),
                })
        })
        .collect();

    Json(ProvidersResp { current, providers })
}

async fn current_provider(
    State(state): State<AppState>,
) -> Result<Json<ProviderSummary>, ApiError> {
    let llm = state.llm.lock();
    let provider = llm
        .current_provider()
        .ok_or_else(|| ApiError::NotFound("当前未设置 provider".to_string()))?;
    Ok(Json(ProviderSummary {
        name: provider.name().to_string(),
        provider_type: provider.provider_type().as_str().to_string(),
        default_model: provider.default_model().to_string(),
    }))
}

async fn switch_provider(
    State(state): State<AppState>,
    Json(req): Json<SwitchProviderReq>,
) -> Result<Json<ProviderSummary>, ApiError> {
    let provider_type = ProviderType::from_str(&req.provider);
    let mut llm = state.llm.lock();
    llm.set_current(provider_type.clone())
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let provider = llm
        .current_provider()
        .ok_or_else(|| ApiError::Internal("切换 provider 后读取失败".to_string()))?;
    Ok(Json(ProviderSummary {
        name: provider.name().to_string(),
        provider_type: provider.provider_type().as_str().to_string(),
        default_model: provider.default_model().to_string(),
    }))
}

async fn list_models(State(state): State<AppState>) -> Json<ModelsResp> {
    let llm = state.llm.lock();
    let current = llm.current_provider_type().map(|p| p.as_str().to_string());
    let models = llm
        .list_providers()
        .into_iter()
        .filter_map(|provider_type| {
            llm.get_provider(provider_type)
                .map(|provider| ModelSummary {
                    provider: provider.provider_type().as_str().to_string(),
                    model: provider.default_model().to_string(),
                })
        })
        .collect();

    Json(ModelsResp {
        current_provider: current,
        models,
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/providers", get(list_providers))
        .route(
            "/providers/current",
            get(current_provider).post(switch_provider),
        )
        .route("/models", get(list_models))
}
