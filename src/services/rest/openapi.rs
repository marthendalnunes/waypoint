use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({ "error": { "code": "invalid_params", "message": "Invalid parameters: Invalid fid: abc" } }))]
pub struct ErrorEnvelopeDoc {
    pub error: ErrorBodyDoc,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorBodyDoc {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({ "fid": 3, "username": "dwr", "display_name": "Dan Romero" }))]
pub struct UserProfileResponseDoc {
    pub fid: u64,
    pub display_name: Option<String>,
    pub username: Option<String>,
    pub bio: Option<String>,
    pub pfp: Option<String>,
    pub url: Option<String>,
    pub location: Option<String>,
    pub twitter: Option<String>,
    pub github: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VerificationItemDoc {
    pub fid: u64,
    pub address: String,
    pub protocol: String,
    #[serde(rename = "type")]
    pub verification_type: String,
    pub chain_id: Option<u64>,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({ "fid": 3, "count": 1, "verifications": [{ "fid": 3, "address": "0x1234", "protocol": "ethereum", "type": "eoa", "timestamp": 1710000000 }] }))]
pub struct VerificationsResponseDoc {
    pub fid: u64,
    pub count: usize,
    pub verifications: Vec<VerificationItemDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CastSummaryDoc {
    pub fid: u64,
    pub hash: String,
    pub timestamp: Option<u64>,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({ "fid": 3, "count": 1, "casts": [{ "fid": 3, "hash": "0xabc", "text": "hello" }] }))]
pub struct CastListResponseDoc {
    pub fid: u64,
    pub count: usize,
    pub casts: Vec<CastSummaryDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CastRepliesByParentResponseDoc {
    pub parent: ParentCastDoc,
    pub count: usize,
    pub replies: Vec<CastSummaryDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CastRepliesByUrlResponseDoc {
    pub parent_url: String,
    pub count: usize,
    pub replies: Vec<CastSummaryDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ParentCastDoc {
    pub fid: u64,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReactionSummaryDoc {
    pub fid: u64,
    pub hash: String,
    pub timestamp: Option<u64>,
    pub reaction_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({ "fid": 3, "count": 1, "reactions": [{ "fid": 3, "hash": "0xabc", "reaction_type": "like" }] }))]
pub struct ReactionsByFidResponseDoc {
    pub fid: u64,
    pub count: usize,
    pub reactions: Vec<ReactionSummaryDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReactionsByTargetCastResponseDoc {
    pub target_cast: ParentCastDoc,
    pub count: usize,
    pub reactions: Vec<ReactionSummaryDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReactionsByTargetUrlResponseDoc {
    pub target_url: String,
    pub count: usize,
    pub reactions: Vec<ReactionSummaryDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LinkSummaryDoc {
    pub fid: u64,
    pub target_fid: Option<u64>,
    pub link_type: Option<String>,
    pub hash: Option<String>,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({ "fid": 3, "count": 1, "links": [{ "fid": 3, "target_fid": 5, "link_type": "follow" }] }))]
pub struct LinksByFidResponseDoc {
    pub fid: u64,
    pub count: usize,
    pub links: Vec<LinkSummaryDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LinksByTargetResponseDoc {
    pub target_fid: u64,
    pub count: usize,
    pub links: Vec<LinkSummaryDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LinkCompactStateResponseDoc {
    pub fid: u64,
    pub count: usize,
    pub compact_links: Vec<LinkSummaryDoc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({ "root_cast": { "fid": 3, "hash": "0xabc", "text": "hello" }, "conversation": { "replies": [], "has_more": false } }))]
pub struct ConversationResponseDoc {
    pub root_cast: serde_json::Value,
    pub parent_casts: Option<Vec<serde_json::Value>>,
    pub quoted_casts: Option<Vec<serde_json::Value>>,
    pub participants: Option<serde_json::Value>,
    pub topic: Option<String>,
    pub summary: Option<String>,
    pub conversation: serde_json::Value,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::services::rest::handlers::get_openapi,
        crate::services::rest::handlers::get_user_by_fid,
        crate::services::rest::handlers::get_user_by_username,
        crate::services::rest::handlers::get_verifications_by_fid,
        crate::services::rest::handlers::get_cast,
        crate::services::rest::handlers::get_casts_by_fid,
        crate::services::rest::handlers::get_casts_by_mention,
        crate::services::rest::handlers::get_casts_by_parent,
        crate::services::rest::handlers::get_casts_by_parent_url,
        crate::services::rest::handlers::get_conversation,
        crate::services::rest::handlers::get_reactions_by_fid,
        crate::services::rest::handlers::get_reactions_by_target_cast,
        crate::services::rest::handlers::get_reactions_by_target_url,
        crate::services::rest::handlers::get_links_by_fid,
        crate::services::rest::handlers::get_links_by_target,
        crate::services::rest::handlers::get_link_compact_state
    ),
    components(
        schemas(
            ErrorEnvelopeDoc,
            ErrorBodyDoc,
            UserProfileResponseDoc,
            VerificationItemDoc,
            VerificationsResponseDoc,
            CastSummaryDoc,
            CastListResponseDoc,
            CastRepliesByParentResponseDoc,
            CastRepliesByUrlResponseDoc,
            ParentCastDoc,
            ConversationResponseDoc,
            ReactionSummaryDoc,
            ReactionsByFidResponseDoc,
            ReactionsByTargetCastResponseDoc,
            ReactionsByTargetUrlResponseDoc,
            LinkSummaryDoc,
            LinksByFidResponseDoc,
            LinksByTargetResponseDoc,
            LinkCompactStateResponseDoc
        )
    ),
    tags(
        (name = "users", description = "Farcaster user resources"),
        (name = "verifications", description = "Verified wallet resources"),
        (name = "casts", description = "Cast resources"),
        (name = "conversations", description = "Conversation thread resources"),
        (name = "reactions", description = "Reaction resources"),
        (name = "links", description = "Social graph link resources"),
        (name = "meta", description = "Service metadata endpoints")
    )
)]
pub struct RestApiDoc;

static OPENAPI: Lazy<utoipa::openapi::OpenApi> = Lazy::new(RestApiDoc::openapi);

pub fn openapi() -> utoipa::openapi::OpenApi {
    OPENAPI.clone()
}
