use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;
use utoipa_swagger_ui::SwaggerUi;

use crate::services::rest::{
    RestError, RestState,
    state::{ResourceReadOptions, RestResource, parse_address_bytes, parse_hash_bytes},
};

const DEFAULT_LIMIT: usize = 10;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct LimitQuery {
    limit: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct UrlQuery {
    url: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ConversationQuery {
    recursive: Option<bool>,
    max_depth: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct VerificationMessagesQuery {
    limit: Option<usize>,
    start_time: Option<u64>,
    end_time: Option<u64>,
}

fn parse_fid(input: &str) -> Result<u64, RestError> {
    input.parse::<u64>().map_err(|_| RestError::invalid_params(format!("Invalid fid: {}", input)))
}

fn validate_hash(input: &str) -> Result<(), RestError> {
    parse_hash_bytes(input).map(|_| ()).map_err(RestError::invalid_params)
}

fn normalize_limit(limit: Option<usize>, max_limit: usize) -> Result<usize, RestError> {
    let value = limit.unwrap_or(DEFAULT_LIMIT);
    if value == 0 {
        return Err(RestError::invalid_params("limit must be greater than 0"));
    }

    let effective_max = max_limit.max(1);
    Ok(value.min(effective_max))
}

fn required_url(url: Option<String>) -> Result<String, RestError> {
    match url {
        Some(url) if !url.trim().is_empty() => Ok(url),
        _ => Err(RestError::invalid_params("Missing required query parameter: url")),
    }
}

fn validate_time_range(start_time: Option<u64>, end_time: Option<u64>) -> Result<(), RestError> {
    if let (Some(start), Some(end)) = (start_time, end_time)
        && start > end
    {
        return Err(RestError::invalid_params("start_time must be less than or equal to end_time"));
    }

    Ok(())
}

async fn fetch_resource(
    state: &RestState,
    resource: RestResource,
    options: ResourceReadOptions,
) -> Result<Json<serde_json::Value>, RestError> {
    let value = state.reader.read_resource(resource, options).await?;
    Ok(Json(value))
}

pub fn router(swagger_ui_enabled: bool) -> Router<RestState> {
    let router = Router::new()
        .route("/api/v1/openapi.json", get(get_openapi))
        .route("/api/v1/users/by-username/{username}", get(get_user_by_username))
        .route("/api/v1/users/{fid}", get(get_user_by_fid))
        .route("/api/v1/verifications/all-by-fid/{fid}", get(get_all_verification_messages_by_fid))
        .route("/api/v1/verifications/{fid}/{address}", get(get_verification_by_address))
        .route("/api/v1/verifications/{fid}", get(get_verifications_by_fid))
        .route("/api/v1/casts/by-fid/{fid}", get(get_casts_by_fid))
        .route("/api/v1/casts/by-mention/{fid}", get(get_casts_by_mention))
        .route("/api/v1/casts/by-parent/{fid}/{hash}", get(get_casts_by_parent))
        .route("/api/v1/casts/by-parent-url", get(get_casts_by_parent_url))
        .route("/api/v1/casts/{fid}/{hash}", get(get_cast))
        .route("/api/v1/conversations/{fid}/{hash}", get(get_conversation))
        .route("/api/v1/reactions/by-fid/{fid}", get(get_reactions_by_fid))
        .route("/api/v1/reactions/by-target-cast/{fid}/{hash}", get(get_reactions_by_target_cast))
        .route("/api/v1/reactions/by-target-url", get(get_reactions_by_target_url))
        .route("/api/v1/links/by-fid/{fid}", get(get_links_by_fid))
        .route("/api/v1/links/by-target/{fid}", get(get_links_by_target))
        .route("/api/v1/links/compact-state/{fid}", get(get_link_compact_state))
        .route("/api/v1/username-proofs/by-name/{name}", get(get_username_proof_by_name))
        .route("/api/v1/username-proofs/{fid}", get(get_username_proofs_by_fid));

    if swagger_ui_enabled {
        router.merge(
            SwaggerUi::new("/swagger-ui")
                .url("/swagger-ui/openapi.json", crate::services::rest::openapi::openapi()),
        )
    } else {
        router
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/openapi.json",
    tag = "meta",
    responses(
        (status = 200, description = "Generated OpenAPI specification document", body = serde_json::Value)
    )
)]
pub(crate) async fn get_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(crate::services::rest::openapi::openapi())
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{fid}",
    tag = "users",
    params(
        ("fid" = u64, Path, description = "Farcaster ID")
    ),
    responses(
        (
            status = 200,
            description = "User profile by FID",
            body = crate::services::rest::openapi::UserProfileResponseDoc
        ),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 404, description = "User not found", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_user_by_fid(
    State(state): State<RestState>,
    Path(fid): Path<String>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    fetch_resource(&state, RestResource::UserByFid { fid }, ResourceReadOptions::default()).await
}

#[utoipa::path(
    get,
    path = "/api/v1/users/by-username/{username}",
    tag = "users",
    params(
        ("username" = String, Path, description = "Farcaster username")
    ),
    responses(
        (
            status = 200,
            description = "User profile by username",
            body = crate::services::rest::openapi::UserProfileResponseDoc
        ),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 404, description = "User not found", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_user_by_username(
    State(state): State<RestState>,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, RestError> {
    fetch_resource(
        &state,
        RestResource::UserByUsername { username },
        ResourceReadOptions::default(),
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/verifications/{fid}",
    tag = "verifications",
    params(
        ("fid" = u64, Path, description = "Farcaster ID"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (
            status = 200,
            description = "Verifications by FID",
            body = crate::services::rest::openapi::VerificationsResponseDoc
        ),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_verifications_by_fid(
    State(state): State<RestState>,
    Path(fid): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;
    fetch_resource(
        &state,
        RestResource::VerificationsByFid { fid },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/verifications/{fid}/{address}",
    tag = "verifications",
    params(
        ("fid" = u64, Path, description = "Farcaster ID"),
        ("address" = String, Path, description = "Address in hex format (with or without 0x prefix)")
    ),
    responses(
        (
            status = 200,
            description = "Verification by FID and address",
            body = crate::services::rest::openapi::VerificationByAddressResponseDoc
        ),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 404, description = "Verification not found", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_verification_by_address(
    State(state): State<RestState>,
    Path((fid, address)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    parse_address_bytes(&address).map_err(RestError::invalid_params)?;

    fetch_resource(
        &state,
        RestResource::VerificationByAddress { fid, address },
        ResourceReadOptions::default(),
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/verifications/all-by-fid/{fid}",
    tag = "verifications",
    params(
        ("fid" = u64, Path, description = "Farcaster ID"),
        ("limit" = Option<usize>, Query, description = "Max number of records"),
        ("start_time" = Option<u64>, Query, description = "Filter records at or after this timestamp"),
        ("end_time" = Option<u64>, Query, description = "Filter records at or before this timestamp")
    ),
    responses(
        (
            status = 200,
            description = "All verification messages by FID",
            body = crate::services::rest::openapi::AllVerificationMessagesByFidResponseDoc
        ),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_all_verification_messages_by_fid(
    State(state): State<RestState>,
    Path(fid): Path<String>,
    Query(query): Query<VerificationMessagesQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;
    validate_time_range(query.start_time, query.end_time)?;

    fetch_resource(
        &state,
        RestResource::AllVerificationMessagesByFid {
            fid,
            start_time: query.start_time,
            end_time: query.end_time,
        },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/casts/{fid}/{hash}",
    tag = "casts",
    params(
        ("fid" = u64, Path, description = "Author Farcaster ID"),
        ("hash" = String, Path, description = "Cast hash (hex, with or without 0x)")
    ),
    responses(
        (
            status = 200,
            description = "Cast by FID and hash",
            body = crate::services::rest::openapi::CastSummaryDoc
        ),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 404, description = "Cast not found", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_cast(
    State(state): State<RestState>,
    Path((fid, hash)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    validate_hash(&hash)?;
    fetch_resource(&state, RestResource::Cast { fid, hash }, ResourceReadOptions::default()).await
}

#[utoipa::path(
    get,
    path = "/api/v1/casts/by-fid/{fid}",
    tag = "casts",
    params(
        ("fid" = u64, Path, description = "Author Farcaster ID"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (
            status = 200,
            description = "Recent casts by FID",
            body = crate::services::rest::openapi::CastListResponseDoc
        ),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_casts_by_fid(
    State(state): State<RestState>,
    Path(fid): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;
    fetch_resource(
        &state,
        RestResource::CastsByFid { fid },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/casts/by-mention/{fid}",
    tag = "casts",
    params(
        ("fid" = u64, Path, description = "Mentioned Farcaster ID"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (status = 200, description = "Casts mentioning a FID", body = crate::services::rest::openapi::CastListResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_casts_by_mention(
    State(state): State<RestState>,
    Path(fid): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;
    fetch_resource(
        &state,
        RestResource::CastsByMention { fid },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/casts/by-parent/{fid}/{hash}",
    tag = "casts",
    params(
        ("fid" = u64, Path, description = "Parent cast author FID"),
        ("hash" = String, Path, description = "Parent cast hash"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (status = 200, description = "Replies to a parent cast", body = crate::services::rest::openapi::CastRepliesByParentResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_casts_by_parent(
    State(state): State<RestState>,
    Path((fid, hash)): Path<(String, String)>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    validate_hash(&hash)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;

    fetch_resource(
        &state,
        RestResource::CastsByParent { fid, hash },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/casts/by-parent-url",
    tag = "casts",
    params(
        ("url" = String, Query, description = "Parent URL to match"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (status = 200, description = "Replies to a parent URL", body = crate::services::rest::openapi::CastRepliesByUrlResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_casts_by_parent_url(
    State(state): State<RestState>,
    Query(query): Query<UrlQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let url = required_url(query.url)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;

    fetch_resource(
        &state,
        RestResource::CastsByParentUrl { url },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/conversations/{fid}/{hash}",
    tag = "conversations",
    params(
        ("fid" = u64, Path, description = "Root cast author FID"),
        ("hash" = String, Path, description = "Root cast hash"),
        ("recursive" = Option<bool>, Query, description = "Include nested replies"),
        ("max_depth" = Option<usize>, Query, description = "Maximum nested reply depth"),
        ("limit" = Option<usize>, Query, description = "Max replies per level")
    ),
    responses(
        (status = 200, description = "Conversation thread", body = crate::services::rest::openapi::ConversationResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 404, description = "Conversation root cast not found", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_conversation(
    State(state): State<RestState>,
    Path((fid, hash)): Path<(String, String)>,
    Query(query): Query<ConversationQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    validate_hash(&hash)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;

    if query.max_depth == Some(0) {
        return Err(RestError::invalid_params("max_depth must be greater than 0"));
    }

    let options = ResourceReadOptions {
        limit: Some(limit),
        recursive: query.recursive,
        max_depth: query.max_depth,
    };

    fetch_resource(&state, RestResource::Conversation { fid, hash }, options).await
}

#[utoipa::path(
    get,
    path = "/api/v1/reactions/by-fid/{fid}",
    tag = "reactions",
    params(
        ("fid" = u64, Path, description = "Author Farcaster ID"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (status = 200, description = "Reactions by FID", body = crate::services::rest::openapi::ReactionsByFidResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_reactions_by_fid(
    State(state): State<RestState>,
    Path(fid): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;

    fetch_resource(
        &state,
        RestResource::ReactionsByFid { fid },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/reactions/by-target-cast/{fid}/{hash}",
    tag = "reactions",
    params(
        ("fid" = u64, Path, description = "Target cast author FID"),
        ("hash" = String, Path, description = "Target cast hash"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (status = 200, description = "Reactions for a target cast", body = crate::services::rest::openapi::ReactionsByTargetCastResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_reactions_by_target_cast(
    State(state): State<RestState>,
    Path((fid, hash)): Path<(String, String)>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    validate_hash(&hash)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;

    fetch_resource(
        &state,
        RestResource::ReactionsByTargetCast { fid, hash },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/reactions/by-target-url",
    tag = "reactions",
    params(
        ("url" = String, Query, description = "Target URL"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (status = 200, description = "Reactions for a target URL", body = crate::services::rest::openapi::ReactionsByTargetUrlResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_reactions_by_target_url(
    State(state): State<RestState>,
    Query(query): Query<UrlQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let url = required_url(query.url)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;

    fetch_resource(
        &state,
        RestResource::ReactionsByTargetUrl { url },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/links/by-fid/{fid}",
    tag = "links",
    params(
        ("fid" = u64, Path, description = "Source Farcaster ID"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (status = 200, description = "Links by FID", body = crate::services::rest::openapi::LinksByFidResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_links_by_fid(
    State(state): State<RestState>,
    Path(fid): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;

    fetch_resource(
        &state,
        RestResource::LinksByFid { fid },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/links/by-target/{fid}",
    tag = "links",
    params(
        ("fid" = u64, Path, description = "Target Farcaster ID"),
        ("limit" = Option<usize>, Query, description = "Max number of records")
    ),
    responses(
        (status = 200, description = "Links by target FID", body = crate::services::rest::openapi::LinksByTargetResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_links_by_target(
    State(state): State<RestState>,
    Path(fid): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    let limit = normalize_limit(query.limit, state.max_limit)?;

    fetch_resource(
        &state,
        RestResource::LinksByTarget { fid },
        ResourceReadOptions { limit: Some(limit), ..Default::default() },
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/links/compact-state/{fid}",
    tag = "links",
    params(
        ("fid" = u64, Path, description = "Farcaster ID")
    ),
    responses(
        (status = 200, description = "Compact link state by FID", body = crate::services::rest::openapi::LinkCompactStateResponseDoc),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_link_compact_state(
    State(state): State<RestState>,
    Path(fid): Path<String>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    fetch_resource(
        &state,
        RestResource::LinkCompactStateByFid { fid },
        ResourceReadOptions::default(),
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/username-proofs/by-name/{name}",
    tag = "username-proofs",
    params(
        ("name" = String, Path, description = "Username to lookup (e.g. alice, vitalik.eth)")
    ),
    responses(
        (
            status = 200,
            description = "Username proof by name",
            body = crate::services::rest::openapi::UsernameProofByNameResponseDoc
        ),
        (status = 404, description = "Username proof not found", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_username_proof_by_name(
    State(state): State<RestState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, RestError> {
    fetch_resource(
        &state,
        RestResource::UsernameProofByName { name },
        ResourceReadOptions::default(),
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/username-proofs/{fid}",
    tag = "username-proofs",
    params(
        ("fid" = u64, Path, description = "Farcaster ID")
    ),
    responses(
        (
            status = 200,
            description = "Username proofs by FID",
            body = crate::services::rest::openapi::UsernameProofsByFidResponseDoc
        ),
        (status = 400, description = "Invalid request parameters", body = crate::services::rest::openapi::ErrorEnvelopeDoc),
        (status = 500, description = "Internal server error", body = crate::services::rest::openapi::ErrorEnvelopeDoc)
    )
)]
pub(crate) async fn get_username_proofs_by_fid(
    State(state): State<RestState>,
    Path(fid): Path<String>,
) -> Result<Json<serde_json::Value>, RestError> {
    let fid = parse_fid(&fid)?;
    fetch_resource(
        &state,
        RestResource::UsernameProofsByFid { fid },
        ResourceReadOptions::default(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    use crate::services::rest::ResourceReader;

    #[derive(Clone, Default)]
    struct MockReader {
        calls: Arc<Mutex<Vec<(RestResource, ResourceReadOptions)>>>,
    }

    impl MockReader {
        async fn calls(&self) -> Vec<(RestResource, ResourceReadOptions)> {
            self.calls.lock().await.clone()
        }
    }

    #[async_trait]
    impl ResourceReader for MockReader {
        async fn read_resource(
            &self,
            resource: RestResource,
            options: ResourceReadOptions,
        ) -> Result<serde_json::Value, RestError> {
            self.calls.lock().await.push((resource.clone(), options));

            Ok(serde_json::json!({
                "resource": format!("{:?}", resource),
                "limit": options.limit,
                "recursive": options.recursive,
                "max_depth": options.max_depth,
            }))
        }
    }

    #[derive(Clone, Default)]
    struct NotFoundReader;

    #[async_trait]
    impl ResourceReader for NotFoundReader {
        async fn read_resource(
            &self,
            _resource: RestResource,
            _options: ResourceReadOptions,
        ) -> Result<serde_json::Value, RestError> {
            Err(RestError::NotFound("resource missing".to_string()))
        }
    }

    #[derive(Clone, Default)]
    struct EmptyListReader;

    #[async_trait]
    impl ResourceReader for EmptyListReader {
        async fn read_resource(
            &self,
            resource: RestResource,
            _options: ResourceReadOptions,
        ) -> Result<serde_json::Value, RestError> {
            let payload = match resource {
                RestResource::CastsByFid { fid } => {
                    serde_json::json!({ "fid": fid, "count": 0, "casts": [] })
                },
                _ => serde_json::json!({ "count": 0 }),
            };

            Ok(payload)
        }
    }

    fn app_with_reader(reader: Arc<dyn ResourceReader>, swagger_ui_enabled: bool) -> Router {
        router(swagger_ui_enabled).with_state(RestState::new(reader, 50))
    }

    fn app(reader: MockReader) -> Router {
        app_with_reader(Arc::new(reader), false)
    }

    fn app_with_swagger(reader: MockReader, swagger_ui_enabled: bool) -> Router {
        app_with_reader(Arc::new(reader), swagger_ui_enabled)
    }

    #[tokio::test]
    async fn all_routes_are_registered_and_return_ok() {
        let reader = MockReader::default();
        let app = app(reader);

        let uris = [
            "/api/v1/openapi.json",
            "/api/v1/users/123",
            "/api/v1/users/by-username/alice",
            "/api/v1/verifications/123",
            "/api/v1/verifications/123/0xabc123",
            "/api/v1/verifications/all-by-fid/123",
            "/api/v1/casts/123/0abc",
            "/api/v1/casts/by-fid/123",
            "/api/v1/casts/by-mention/123",
            "/api/v1/casts/by-parent/123/0abc",
            "/api/v1/casts/by-parent-url?url=https%3A%2F%2Fexample.com",
            "/api/v1/conversations/123/0abc",
            "/api/v1/reactions/by-fid/123",
            "/api/v1/reactions/by-target-cast/123/0abc",
            "/api/v1/reactions/by-target-url?url=https%3A%2F%2Fexample.com",
            "/api/v1/links/by-fid/123",
            "/api/v1/links/by-target/123",
            "/api/v1/links/compact-state/123",
            "/api/v1/username-proofs/123",
            "/api/v1/username-proofs/by-name/alice",
        ];

        for uri in uris {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK, "route failed: {}", uri);
        }
    }

    #[tokio::test]
    async fn invalid_fid_params_return_json_400_across_routes() {
        let app = app(MockReader::default());
        let uris = [
            "/api/v1/users/not-a-fid",
            "/api/v1/verifications/not-a-fid",
            "/api/v1/verifications/not-a-fid/0xabc123",
            "/api/v1/verifications/all-by-fid/not-a-fid",
            "/api/v1/casts/not-a-fid/0abc",
            "/api/v1/casts/by-fid/not-a-fid",
            "/api/v1/casts/by-mention/not-a-fid",
            "/api/v1/casts/by-parent/not-a-fid/0abc",
            "/api/v1/conversations/not-a-fid/0abc",
            "/api/v1/reactions/by-fid/not-a-fid",
            "/api/v1/reactions/by-target-cast/not-a-fid/0abc",
            "/api/v1/links/by-fid/not-a-fid",
            "/api/v1/links/by-target/not-a-fid",
            "/api/v1/links/compact-state/not-a-fid",
            "/api/v1/username-proofs/not-a-fid",
        ];

        for uri in uris {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "route failed: {}", uri);

            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(value["error"]["code"], "invalid_params", "route failed: {}", uri);
        }
    }

    #[tokio::test]
    async fn invalid_hash_returns_json_400_for_all_hash_routes() {
        let app = app(MockReader::default());
        let uris = [
            "/api/v1/casts/123/not-hex",
            "/api/v1/casts/by-parent/123/not-hex",
            "/api/v1/conversations/123/not-hex",
            "/api/v1/reactions/by-target-cast/123/not-hex",
        ];

        for uri in uris {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "route failed: {}", uri);
        }
    }

    #[tokio::test]
    async fn invalid_verification_address_returns_json_400() {
        let app = app(MockReader::default());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/verifications/123/not-hex")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn missing_url_query_returns_json_400() {
        let app = app(MockReader::default());
        let uris = ["/api/v1/casts/by-parent-url", "/api/v1/reactions/by-target-url"];

        for uri in uris {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "route failed: {}", uri);
        }
    }

    #[tokio::test]
    async fn singular_not_found_from_reader_returns_json_404() {
        let app = app_with_reader(Arc::new(NotFoundReader), false);

        let response = app
            .oneshot(Request::builder().uri("/api/v1/users/123").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["error"]["code"], "not_found");
    }

    #[tokio::test]
    async fn list_empty_payload_from_reader_returns_json_200() {
        let app = app_with_reader(Arc::new(EmptyListReader), false);

        let response = app
            .oneshot(
                Request::builder().uri("/api/v1/casts/by-fid/123").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["fid"], 123);
        assert_eq!(value["count"], 0);
        assert_eq!(value["casts"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn limit_validation_and_clamping_work() {
        let reader = MockReader::default();
        let app = app(reader.clone());

        let bad_limit = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/casts/by-fid/123?limit=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad_limit.status(), StatusCode::BAD_REQUEST);

        let clamped = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/casts/by-fid/123?limit=999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(clamped.status(), StatusCode::OK);

        let body = to_bytes(clamped.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["limit"], 50);

        let calls = reader.calls().await;
        assert!(calls.iter().any(|(_, opts)| opts.limit == Some(50)));
    }

    #[tokio::test]
    async fn conversation_defaults_are_applied_and_validation_works() {
        let reader = MockReader::default();
        let app = app(reader.clone());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/conversations/123/0abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let calls = reader.calls().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1.limit, Some(10));
        assert_eq!(calls[0].1.recursive, None);
        assert_eq!(calls[0].1.max_depth, None);

        let invalid = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/conversations/123/0abc?max_depth=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn conversation_custom_query_is_passed_to_reader() {
        let reader = MockReader::default();
        let app = app(reader.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/conversations/7/0abc?recursive=false&max_depth=7&limit=9")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let calls = reader.calls().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1.limit, Some(9));
        assert_eq!(calls[0].1.recursive, Some(false));
        assert_eq!(calls[0].1.max_depth, Some(7));
    }

    #[tokio::test]
    async fn verification_messages_query_validation_works() {
        let app = app(MockReader::default());

        let invalid_limit = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/verifications/all-by-fid/123?limit=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid_limit.status(), StatusCode::BAD_REQUEST);

        let invalid_time_range = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/verifications/all-by-fid/123?start_time=100&end_time=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid_time_range.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn verification_messages_query_is_passed_to_reader() {
        let reader = MockReader::default();
        let app = app(reader.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/verifications/all-by-fid/7?start_time=10&end_time=20&limit=9")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let calls = reader.calls().await;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1.limit, Some(9));
        assert_eq!(
            calls[0].0,
            RestResource::AllVerificationMessagesByFid {
                fid: 7,
                start_time: Some(10),
                end_time: Some(20),
            }
        );
    }

    #[tokio::test]
    async fn openapi_endpoint_is_available() {
        let app = app(MockReader::default());
        let response = app
            .oneshot(Request::builder().uri("/api/v1/openapi.json").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(value.get("openapi").is_some());
        assert!(value.get("paths").is_some());

        let expected_paths = [
            "/api/v1/openapi.json",
            "/api/v1/users/{fid}",
            "/api/v1/users/by-username/{username}",
            "/api/v1/verifications/{fid}",
            "/api/v1/verifications/{fid}/{address}",
            "/api/v1/verifications/all-by-fid/{fid}",
            "/api/v1/casts/{fid}/{hash}",
            "/api/v1/casts/by-fid/{fid}",
            "/api/v1/casts/by-mention/{fid}",
            "/api/v1/casts/by-parent/{fid}/{hash}",
            "/api/v1/casts/by-parent-url",
            "/api/v1/conversations/{fid}/{hash}",
            "/api/v1/reactions/by-fid/{fid}",
            "/api/v1/reactions/by-target-cast/{fid}/{hash}",
            "/api/v1/reactions/by-target-url",
            "/api/v1/links/by-fid/{fid}",
            "/api/v1/links/by-target/{fid}",
            "/api/v1/links/compact-state/{fid}",
            "/api/v1/username-proofs/by-name/{name}",
            "/api/v1/username-proofs/{fid}",
        ];

        for path in expected_paths {
            assert!(value["paths"].get(path).is_some(), "missing OpenAPI path: {}", path);
        }

        let expected_schemas = [
            "ErrorEnvelopeDoc",
            "UserProfileResponseDoc",
            "VerificationsResponseDoc",
            "VerificationByAddressResponseDoc",
            "AllVerificationMessagesByFidResponseDoc",
            "CastSummaryDoc",
            "CastListResponseDoc",
            "ConversationResponseDoc",
            "ReactionsByFidResponseDoc",
            "LinksByFidResponseDoc",
            "UsernameProofDoc",
            "UsernameProofByNameResponseDoc",
            "UsernameProofsByFidResponseDoc",
        ];

        for schema in expected_schemas {
            assert!(
                value["components"]["schemas"].get(schema).is_some(),
                "missing OpenAPI schema: {}",
                schema
            );
        }
    }

    #[tokio::test]
    async fn swagger_ui_endpoint_is_available() {
        let app = app_with_swagger(MockReader::default(), true);
        let response = app
            .oneshot(Request::builder().uri("/swagger-ui/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(response.status().is_success() || response.status().is_redirection());
    }

    #[tokio::test]
    async fn swagger_ui_endpoint_is_disabled_by_default() {
        let app = app(MockReader::default());
        let response = app
            .oneshot(Request::builder().uri("/swagger-ui/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn routes_map_to_expected_waypoint_resources() {
        let cases = vec![
            ("/api/v1/users/1", RestResource::UserByFid { fid: 1 }),
            (
                "/api/v1/users/by-username/alice",
                RestResource::UserByUsername { username: "alice".to_string() },
            ),
            ("/api/v1/verifications/2", RestResource::VerificationsByFid { fid: 2 }),
            (
                "/api/v1/verifications/2/0xabc123",
                RestResource::VerificationByAddress { fid: 2, address: "0xabc123".to_string() },
            ),
            (
                "/api/v1/verifications/all-by-fid/2",
                RestResource::AllVerificationMessagesByFid {
                    fid: 2,
                    start_time: None,
                    end_time: None,
                },
            ),
            ("/api/v1/casts/3/0abc", RestResource::Cast { fid: 3, hash: "0abc".to_string() }),
            ("/api/v1/casts/by-fid/4", RestResource::CastsByFid { fid: 4 }),
            ("/api/v1/casts/by-mention/5", RestResource::CastsByMention { fid: 5 }),
            (
                "/api/v1/casts/by-parent/6/0abc",
                RestResource::CastsByParent { fid: 6, hash: "0abc".to_string() },
            ),
            (
                "/api/v1/casts/by-parent-url?url=https%3A%2F%2Fexample.com",
                RestResource::CastsByParentUrl { url: "https://example.com".to_string() },
            ),
            (
                "/api/v1/conversations/7/0abc",
                RestResource::Conversation { fid: 7, hash: "0abc".to_string() },
            ),
            ("/api/v1/reactions/by-fid/8", RestResource::ReactionsByFid { fid: 8 }),
            (
                "/api/v1/reactions/by-target-cast/9/0abc",
                RestResource::ReactionsByTargetCast { fid: 9, hash: "0abc".to_string() },
            ),
            (
                "/api/v1/reactions/by-target-url?url=https%3A%2F%2Fexample.com",
                RestResource::ReactionsByTargetUrl { url: "https://example.com".to_string() },
            ),
            ("/api/v1/links/by-fid/10", RestResource::LinksByFid { fid: 10 }),
            ("/api/v1/links/by-target/11", RestResource::LinksByTarget { fid: 11 }),
            ("/api/v1/links/compact-state/12", RestResource::LinkCompactStateByFid { fid: 12 }),
            (
                "/api/v1/username-proofs/by-name/vitalik.eth",
                RestResource::UsernameProofByName { name: "vitalik.eth".to_string() },
            ),
            ("/api/v1/username-proofs/13", RestResource::UsernameProofsByFid { fid: 13 }),
        ];

        for (uri, expected) in cases {
            let reader = MockReader::default();
            let app = app(reader.clone());

            let response = app
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);

            let calls = reader.calls().await;
            assert_eq!(calls.len(), 1, "expected exactly one resource call for {}", uri);
            assert_eq!(calls[0].0, expected, "resource mismatch for {}", uri);
        }
    }
}
