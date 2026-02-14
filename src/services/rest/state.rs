use std::sync::Arc;

use async_trait::async_trait;

use crate::core::types::Fid;
use crate::services::{mcp::WaypointMcpService, rest::error::RestError};

const DEFAULT_LIMIT: usize = 10;
const DEFAULT_LINK_TYPE: &str = "follow";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceReadOptions {
    pub limit: Option<usize>,
    pub recursive: Option<bool>,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestResource {
    UserByFid { fid: u64 },
    UserByUsername { username: String },
    VerificationsByFid { fid: u64 },
    VerificationByAddress { fid: u64, address: String },
    AllVerificationMessagesByFid { fid: u64, start_time: Option<u64>, end_time: Option<u64> },
    Cast { fid: u64, hash: String },
    Conversation { fid: u64, hash: String },
    CastsByFid { fid: u64 },
    CastsByMention { fid: u64 },
    CastsByParent { fid: u64, hash: String },
    CastsByParentUrl { url: String },
    ReactionsByFid { fid: u64 },
    ReactionsByTargetCast { fid: u64, hash: String },
    ReactionsByTargetUrl { url: String },
    LinksByFid { fid: u64 },
    LinksByTarget { fid: u64 },
    LinkCompactStateByFid { fid: u64 },
    UsernameProofByName { name: String },
    UsernameProofsByFid { fid: u64 },
}

#[derive(Debug, thiserror::Error)]
pub enum ResourceReadError {
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

fn normalize_limit(limit: Option<usize>) -> usize {
    match limit {
        Some(0) => DEFAULT_LIMIT,
        Some(value) => value,
        None => DEFAULT_LIMIT,
    }
}

fn empty_list_payload(resource: &RestResource) -> serde_json::Value {
    match resource {
        RestResource::VerificationsByFid { fid } => {
            serde_json::json!({ "fid": fid, "count": 0, "verifications": [] })
        },
        RestResource::AllVerificationMessagesByFid { fid, start_time, end_time } => {
            serde_json::json!({
                "fid": fid,
                "count": 0,
                "start_time": start_time,
                "end_time": end_time,
                "verifications": []
            })
        },
        RestResource::CastsByFid { fid } | RestResource::CastsByMention { fid } => {
            serde_json::json!({ "fid": fid, "count": 0, "casts": [] })
        },
        RestResource::CastsByParent { fid, hash } => serde_json::json!({
            "parent": { "fid": fid, "hash": hash },
            "count": 0,
            "replies": []
        }),
        RestResource::CastsByParentUrl { url } => {
            serde_json::json!({ "parent_url": url, "count": 0, "replies": [] })
        },
        RestResource::ReactionsByFid { fid } => {
            serde_json::json!({ "fid": fid, "count": 0, "reactions": [] })
        },
        RestResource::ReactionsByTargetCast { fid, hash } => serde_json::json!({
            "target_cast": { "fid": fid, "hash": hash },
            "count": 0,
            "reactions": []
        }),
        RestResource::ReactionsByTargetUrl { url } => {
            serde_json::json!({ "target_url": url, "count": 0, "reactions": [] })
        },
        RestResource::LinksByFid { fid } => {
            serde_json::json!({ "fid": fid, "count": 0, "links": [] })
        },
        RestResource::LinksByTarget { fid } => {
            serde_json::json!({ "target_fid": fid, "count": 0, "links": [] })
        },
        RestResource::LinkCompactStateByFid { fid } => {
            serde_json::json!({ "fid": fid, "count": 0, "compact_links": [] })
        },
        RestResource::UsernameProofsByFid { fid } => {
            serde_json::json!({ "fid": fid, "count": 0, "proofs": [] })
        },
        _ => serde_json::json!({}),
    }
}

fn classify_found_false_error(message: String) -> ResourceReadError {
    let lowered = message.trim().to_ascii_lowercase();

    if lowered.starts_with("error") {
        return ResourceReadError::Internal(message);
    }

    ResourceReadError::NotFound(message)
}

fn parse_resource_output(
    resource: &RestResource,
    output: String,
) -> Result<serde_json::Value, ResourceReadError> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&output) {
        if matches!(
            resource,
            RestResource::UserByUsername { .. }
                | RestResource::VerificationByAddress { .. }
                | RestResource::UsernameProofByName { .. }
        ) && value.get("found").and_then(serde_json::Value::as_bool) == Some(false)
        {
            let message = value
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Resource not found")
                .to_string();
            return Err(classify_found_false_error(message));
        }

        return Ok(value);
    }

    let lowered = output.to_lowercase();

    if lowered.starts_with("invalid") || lowered.contains("missing ") {
        return Err(ResourceReadError::InvalidParams(output));
    }

    if lowered.starts_with("error") {
        return Err(ResourceReadError::Internal(output));
    }

    if lowered.starts_with("no ") || lowered.contains(" not found") {
        return match resource {
            RestResource::UserByFid { .. }
            | RestResource::UserByUsername { .. }
            | RestResource::VerificationByAddress { .. }
            | RestResource::Cast { .. }
            | RestResource::Conversation { .. }
            | RestResource::UsernameProofByName { .. } => Err(ResourceReadError::NotFound(output)),
            _ => Ok(empty_list_payload(resource)),
        };
    }

    Err(ResourceReadError::Internal(output))
}

pub fn parse_hash_bytes(hash: &str) -> Result<Vec<u8>, String> {
    let trimmed = hash.trim_start_matches("0x");
    if trimmed.is_empty() {
        return Err("Missing hash value".to_string());
    }

    hex::decode(trimmed).map_err(|_| format!("Invalid hash format: {hash}"))
}

pub fn parse_address_bytes(address: &str) -> Result<Vec<u8>, String> {
    let trimmed = address.trim_start_matches("0x");
    if trimmed.is_empty() {
        return Err("Invalid address format: empty address".to_string());
    }

    hex::decode(trimmed).map_err(|_| format!("Invalid address format: {address}"))
}

#[async_trait]
pub trait ResourceReader: Send + Sync {
    async fn read_resource(
        &self,
        resource: RestResource,
        options: ResourceReadOptions,
    ) -> Result<serde_json::Value, RestError>;
}

#[derive(Clone)]
pub struct McpResourceReader<DB, HC> {
    service: WaypointMcpService<DB, HC>,
}

impl<DB, HC> McpResourceReader<DB, HC>
where
    DB: crate::core::data_context::Database + Clone + Send + Sync + 'static,
    HC: crate::core::data_context::HubClient + Clone + Send + Sync + 'static,
{
    pub fn new(service: WaypointMcpService<DB, HC>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl<DB, HC> ResourceReader for McpResourceReader<DB, HC>
where
    DB: crate::core::data_context::Database + Clone + Send + Sync + 'static,
    HC: crate::core::data_context::HubClient + Clone + Send + Sync + 'static,
{
    async fn read_resource(
        &self,
        resource: RestResource,
        options: ResourceReadOptions,
    ) -> Result<serde_json::Value, RestError> {
        let limit = normalize_limit(options.limit);

        let output = match &resource {
            RestResource::UserByFid { fid } => {
                self.service.do_get_user_by_fid(Fid::from(*fid)).await
            },
            RestResource::UserByUsername { username } => {
                self.service.do_get_user_by_username(username).await
            },
            RestResource::VerificationsByFid { fid } => {
                self.service.do_get_verifications_by_fid(Fid::from(*fid), limit).await
            },
            RestResource::VerificationByAddress { fid, address } => {
                self.service.do_get_verification(Fid::from(*fid), address).await
            },
            RestResource::AllVerificationMessagesByFid { fid, start_time, end_time } => {
                self.service
                    .do_get_all_verification_messages_by_fid(
                        Fid::from(*fid),
                        limit,
                        *start_time,
                        *end_time,
                    )
                    .await
            },
            RestResource::Cast { fid, hash } => {
                self.service.do_get_cast(Fid::from(*fid), hash).await
            },
            RestResource::Conversation { fid, hash } => {
                let recursive = options.recursive.unwrap_or(true);
                let max_depth = options.max_depth.unwrap_or(5);
                self.service
                    .do_get_conversation(Fid::from(*fid), hash, recursive, max_depth, limit)
                    .await
            },
            RestResource::CastsByFid { fid } => {
                self.service.do_get_casts_by_fid(Fid::from(*fid), limit).await
            },
            RestResource::CastsByMention { fid } => {
                self.service.do_get_casts_by_mention(Fid::from(*fid), limit).await
            },
            RestResource::CastsByParent { fid, hash } => {
                self.service.do_get_casts_by_parent(Fid::from(*fid), hash, limit).await
            },
            RestResource::CastsByParentUrl { url } => {
                self.service.do_get_casts_by_parent_url(url, limit).await
            },
            RestResource::ReactionsByFid { fid } => {
                self.service.do_get_reactions_by_fid(Fid::from(*fid), None, limit).await
            },
            RestResource::ReactionsByTargetCast { fid, hash } => {
                let target_cast_hash =
                    parse_hash_bytes(hash).map_err(ResourceReadError::InvalidParams)?;

                self.service
                    .do_get_reactions_by_target(
                        Some(Fid::from(*fid)),
                        Some(target_cast_hash.as_slice()),
                        None,
                        None,
                        limit,
                    )
                    .await
            },
            RestResource::ReactionsByTargetUrl { url } => {
                self.service.do_get_reactions_by_target(None, None, Some(url), None, limit).await
            },
            RestResource::LinksByFid { fid } => {
                self.service
                    .do_get_links_by_fid(Fid::from(*fid), Some(DEFAULT_LINK_TYPE), limit)
                    .await
            },
            RestResource::LinksByTarget { fid } => {
                self.service
                    .do_get_links_by_target(Fid::from(*fid), Some(DEFAULT_LINK_TYPE), limit)
                    .await
            },
            RestResource::LinkCompactStateByFid { fid } => {
                self.service.do_get_link_compact_state_by_fid(Fid::from(*fid)).await
            },
            RestResource::UsernameProofByName { name } => {
                self.service.do_get_username_proof(name).await
            },
            RestResource::UsernameProofsByFid { fid } => {
                self.service.do_get_username_proofs_by_fid(Fid::from(*fid)).await
            },
        };

        parse_resource_output(&resource, output).map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct RestState {
    pub reader: Arc<dyn ResourceReader>,
    pub max_limit: usize,
}

impl RestState {
    pub fn new(reader: Arc<dyn ResourceReader>, max_limit: usize) -> Self {
        Self { reader, max_limit }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singular_resources_map_not_found_to_error() {
        let resource = RestResource::UserByFid { fid: 1 };
        let result = parse_resource_output(&resource, "No user data found for FID 1".to_string());
        assert!(matches!(result, Err(ResourceReadError::NotFound(_))));
    }

    #[test]
    fn list_resources_map_not_found_to_empty_payload() {
        let resource = RestResource::CastsByFid { fid: 1 };
        let result =
            parse_resource_output(&resource, "No casts found for FID 1".to_string()).unwrap();

        assert_eq!(result["fid"], 1);
        assert_eq!(result["count"], 0);
        assert_eq!(result["casts"], serde_json::json!([]));
    }

    #[test]
    fn username_not_found_json_maps_to_404() {
        let resource = RestResource::UserByUsername { username: "alice".to_string() };
        let output = serde_json::json!({
            "username": "alice",
            "found": false,
            "error": "Username not found"
        })
        .to_string();

        let result = parse_resource_output(&resource, output);
        assert!(matches!(result, Err(ResourceReadError::NotFound(_))));
    }

    #[test]
    fn username_lookup_upstream_errors_map_to_500() {
        let resource = RestResource::UserByUsername { username: "alice".to_string() };
        let output = serde_json::json!({
            "username": "alice",
            "found": false,
            "error": "Error: upstream timeout"
        })
        .to_string();

        let result = parse_resource_output(&resource, output);
        assert!(matches!(result, Err(ResourceReadError::Internal(_))));
    }

    #[test]
    fn verification_lookup_found_false_json_maps_to_404() {
        let resource = RestResource::VerificationByAddress { fid: 1, address: "0xabc".to_string() };
        let output = serde_json::json!({
            "fid": 1,
            "address": "0xabc",
            "found": false,
            "error": "Verification not found"
        })
        .to_string();

        let result = parse_resource_output(&resource, output);
        assert!(matches!(result, Err(ResourceReadError::NotFound(_))));
    }

    #[test]
    fn username_proof_lookup_found_false_json_maps_to_404() {
        let resource = RestResource::UsernameProofByName { name: "alice".to_string() };
        let output = serde_json::json!({
            "name": "alice",
            "found": false,
            "error": "Username proof not found"
        })
        .to_string();

        let result = parse_resource_output(&resource, output);
        assert!(matches!(result, Err(ResourceReadError::NotFound(_))));
    }

    #[test]
    fn verification_messages_not_found_maps_to_empty_payload() {
        let resource = RestResource::AllVerificationMessagesByFid {
            fid: 1,
            start_time: Some(10),
            end_time: Some(20),
        };
        let result = parse_resource_output(
            &resource,
            "No verification messages found for FID 1 between timestamps 10 and 20".to_string(),
        )
        .unwrap();

        assert_eq!(result["fid"], 1);
        assert_eq!(result["count"], 0);
        assert_eq!(result["start_time"], 10);
        assert_eq!(result["end_time"], 20);
        assert_eq!(result["verifications"], serde_json::json!([]));
    }

    #[test]
    fn username_proofs_not_found_maps_to_empty_payload() {
        let resource = RestResource::UsernameProofsByFid { fid: 1 };
        let result =
            parse_resource_output(&resource, "No username proofs found for FID 1".to_string())
                .unwrap();

        assert_eq!(result["fid"], 1);
        assert_eq!(result["count"], 0);
        assert_eq!(result["proofs"], serde_json::json!([]));
    }

    #[test]
    fn mcp_json_payload_shape_is_preserved_for_rest_resources() {
        let cases = vec![
            (
                RestResource::UserByFid { fid: 1 },
                serde_json::json!({ "fid": 1, "username": "alice" }),
            ),
            (
                RestResource::UserByUsername { username: "alice".to_string() },
                serde_json::json!({ "fid": 1, "username": "alice", "display_name": "Alice" }),
            ),
            (
                RestResource::VerificationsByFid { fid: 2 },
                serde_json::json!({ "fid": 2, "count": 1, "verifications": [{ "address": "0xabc" }] }),
            ),
            (
                RestResource::VerificationByAddress { fid: 2, address: "0xabc".to_string() },
                serde_json::json!({
                    "fid": 2,
                    "address": "0xabc",
                    "found": true,
                    "verification": { "address": "0xabc" }
                }),
            ),
            (
                RestResource::AllVerificationMessagesByFid {
                    fid: 2,
                    start_time: Some(10),
                    end_time: Some(20),
                },
                serde_json::json!({
                    "fid": 2,
                    "count": 1,
                    "start_time": 10,
                    "end_time": 20,
                    "verifications": [{ "address": "0xabc", "action": "add" }]
                }),
            ),
            (
                RestResource::Cast { fid: 3, hash: "0abc".to_string() },
                serde_json::json!({ "fid": 3, "hash": "0abc", "text": "hello" }),
            ),
            (
                RestResource::Conversation { fid: 4, hash: "0abc".to_string() },
                serde_json::json!({ "fid": 4, "hash": "0abc", "conversation": { "root": { "fid": 4 } } }),
            ),
            (
                RestResource::CastsByFid { fid: 5 },
                serde_json::json!({ "fid": 5, "count": 1, "casts": [{ "hash": "0def" }] }),
            ),
            (
                RestResource::CastsByMention { fid: 6 },
                serde_json::json!({ "fid": 6, "count": 1, "casts": [{ "hash": "0fed" }] }),
            ),
            (
                RestResource::CastsByParent { fid: 7, hash: "0abc".to_string() },
                serde_json::json!({
                    "parent": { "fid": 7, "hash": "0abc" },
                    "count": 1,
                    "replies": [{ "hash": "0aaa" }]
                }),
            ),
            (
                RestResource::CastsByParentUrl { url: "https://example.com".to_string() },
                serde_json::json!({
                    "parent_url": "https://example.com",
                    "count": 1,
                    "replies": [{ "hash": "0bbb" }]
                }),
            ),
            (
                RestResource::ReactionsByFid { fid: 8 },
                serde_json::json!({ "fid": 8, "count": 1, "reactions": [{ "reaction_type": "like" }] }),
            ),
            (
                RestResource::ReactionsByTargetCast { fid: 9, hash: "0abc".to_string() },
                serde_json::json!({
                    "target_cast": { "fid": 9, "hash": "0abc" },
                    "count": 1,
                    "reactions": [{ "reaction_type": "like" }]
                }),
            ),
            (
                RestResource::ReactionsByTargetUrl { url: "https://example.com".to_string() },
                serde_json::json!({
                    "target_url": "https://example.com",
                    "count": 1,
                    "reactions": [{ "reaction_type": "like" }]
                }),
            ),
            (
                RestResource::LinksByFid { fid: 10 },
                serde_json::json!({ "fid": 10, "count": 1, "links": [{ "target_fid": 77 }] }),
            ),
            (
                RestResource::LinksByTarget { fid: 11 },
                serde_json::json!({ "target_fid": 11, "count": 1, "links": [{ "fid": 99 }] }),
            ),
            (
                RestResource::LinkCompactStateByFid { fid: 12 },
                serde_json::json!({
                    "fid": 12,
                    "count": 1,
                    "compact_links": [{ "target_fid": 42, "state": "follow" }]
                }),
            ),
            (
                RestResource::UsernameProofByName { name: "alice".to_string() },
                serde_json::json!({
                    "name": "alice",
                    "found": true,
                    "type": "fname",
                    "fid": 12,
                    "timestamp": 1710000000,
                    "owner": "0xabc"
                }),
            ),
            (
                RestResource::UsernameProofsByFid { fid: 12 },
                serde_json::json!({
                    "fid": 12,
                    "count": 1,
                    "proofs": [{
                        "name": "alice",
                        "type": "fname",
                        "fid": 12,
                        "timestamp": 1710000000,
                        "owner": "0xabc"
                    }]
                }),
            ),
        ];

        for (resource, mcp_payload) in cases {
            let parsed = parse_resource_output(&resource, mcp_payload.to_string()).unwrap();
            assert_eq!(parsed, mcp_payload);
        }
    }

    #[test]
    fn hash_parser_accepts_prefixed_and_unprefixed_hex() {
        let with_prefix = parse_hash_bytes("0x0abc").unwrap();
        let without_prefix = parse_hash_bytes("0abc").unwrap();
        assert_eq!(with_prefix, without_prefix);
        assert_eq!(with_prefix, vec![0x0a, 0xbc]);
    }

    #[test]
    fn address_parser_accepts_prefixed_and_unprefixed_hex() {
        let with_prefix = parse_address_bytes("0x0abc").unwrap();
        let without_prefix = parse_address_bytes("0abc").unwrap();
        assert_eq!(with_prefix, without_prefix);
        assert_eq!(with_prefix, vec![0x0a, 0xbc]);
    }

    #[test]
    fn address_parser_rejects_empty_address() {
        let err = parse_address_bytes("0x").unwrap_err();
        assert_eq!(err, "Invalid address format: empty address");
    }
}
