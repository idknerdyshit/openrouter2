#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteSpec {
    pub method: &'static str,
    pub path: &'static str,
    pub operation: &'static str,
}

pub const SPEC_SNAPSHOT_DATE: &str = "2026-06-28";

pub const NON_DEPRECATED_ROUTES: &[RouteSpec] = &[
    RouteSpec {
        method: "GET",
        path: "/activity",
        operation: "get_user_activity",
    },
    RouteSpec {
        method: "GET",
        path: "/analytics/meta",
        operation: "get_analytics_meta",
    },
    RouteSpec {
        method: "POST",
        path: "/analytics/query",
        operation: "query_analytics",
    },
    RouteSpec {
        method: "POST",
        path: "/audio/speech",
        operation: "create_audio_speech",
    },
    RouteSpec {
        method: "POST",
        path: "/audio/transcriptions",
        operation: "create_audio_transcription",
    },
    RouteSpec {
        method: "POST",
        path: "/auth/keys",
        operation: "exchange_auth_code_for_api_key",
    },
    RouteSpec {
        method: "POST",
        path: "/auth/keys/code",
        operation: "create_auth_key_code",
    },
    RouteSpec {
        method: "GET",
        path: "/benchmarks",
        operation: "list_benchmarks",
    },
    RouteSpec {
        method: "GET",
        path: "/byok",
        operation: "list_byok_keys",
    },
    RouteSpec {
        method: "POST",
        path: "/byok",
        operation: "create_byok_key",
    },
    RouteSpec {
        method: "DELETE",
        path: "/byok/{id}",
        operation: "delete_byok_key",
    },
    RouteSpec {
        method: "GET",
        path: "/byok/{id}",
        operation: "get_byok_key",
    },
    RouteSpec {
        method: "PATCH",
        path: "/byok/{id}",
        operation: "update_byok_key",
    },
    RouteSpec {
        method: "POST",
        path: "/chat/completions",
        operation: "create_chat_completion",
    },
    RouteSpec {
        method: "GET",
        path: "/classifications/task",
        operation: "get_task_classifications",
    },
    RouteSpec {
        method: "GET",
        path: "/credits",
        operation: "get_credits",
    },
    RouteSpec {
        method: "GET",
        path: "/datasets/app-rankings",
        operation: "get_app_rankings",
    },
    RouteSpec {
        method: "GET",
        path: "/datasets/rankings-daily",
        operation: "get_rankings_daily",
    },
    RouteSpec {
        method: "POST",
        path: "/embeddings",
        operation: "create_embeddings",
    },
    RouteSpec {
        method: "GET",
        path: "/embeddings/models",
        operation: "list_embedding_models",
    },
    RouteSpec {
        method: "GET",
        path: "/endpoints/zdr",
        operation: "list_zdr_endpoints",
    },
    RouteSpec {
        method: "GET",
        path: "/files",
        operation: "list_files",
    },
    RouteSpec {
        method: "POST",
        path: "/files",
        operation: "upload_file",
    },
    RouteSpec {
        method: "DELETE",
        path: "/files/{file_id}",
        operation: "delete_file",
    },
    RouteSpec {
        method: "GET",
        path: "/files/{file_id}",
        operation: "get_file_metadata",
    },
    RouteSpec {
        method: "GET",
        path: "/files/{file_id}/content",
        operation: "download_file_content",
    },
    RouteSpec {
        method: "GET",
        path: "/generation",
        operation: "get_generation",
    },
    RouteSpec {
        method: "GET",
        path: "/generation/content",
        operation: "get_generation_content",
    },
    RouteSpec {
        method: "GET",
        path: "/guardrails",
        operation: "list_guardrails",
    },
    RouteSpec {
        method: "POST",
        path: "/guardrails",
        operation: "create_guardrail",
    },
    RouteSpec {
        method: "DELETE",
        path: "/guardrails/{id}",
        operation: "delete_guardrail",
    },
    RouteSpec {
        method: "GET",
        path: "/guardrails/{id}",
        operation: "get_guardrail",
    },
    RouteSpec {
        method: "PATCH",
        path: "/guardrails/{id}",
        operation: "update_guardrail",
    },
    RouteSpec {
        method: "GET",
        path: "/guardrails/{id}/assignments/keys",
        operation: "list_guardrail_key_assignments",
    },
    RouteSpec {
        method: "POST",
        path: "/guardrails/{id}/assignments/keys",
        operation: "bulk_assign_keys_to_guardrail",
    },
    RouteSpec {
        method: "POST",
        path: "/guardrails/{id}/assignments/keys/remove",
        operation: "bulk_unassign_keys_from_guardrail",
    },
    RouteSpec {
        method: "GET",
        path: "/guardrails/{id}/assignments/members",
        operation: "list_guardrail_member_assignments",
    },
    RouteSpec {
        method: "POST",
        path: "/guardrails/{id}/assignments/members",
        operation: "bulk_assign_members_to_guardrail",
    },
    RouteSpec {
        method: "POST",
        path: "/guardrails/{id}/assignments/members/remove",
        operation: "bulk_unassign_members_from_guardrail",
    },
    RouteSpec {
        method: "GET",
        path: "/guardrails/assignments/keys",
        operation: "list_key_assignments",
    },
    RouteSpec {
        method: "GET",
        path: "/guardrails/assignments/members",
        operation: "list_member_assignments",
    },
    RouteSpec {
        method: "POST",
        path: "/images",
        operation: "create_image",
    },
    RouteSpec {
        method: "GET",
        path: "/images/models",
        operation: "list_image_models",
    },
    RouteSpec {
        method: "GET",
        path: "/images/models/{author}/{slug}/endpoints",
        operation: "list_image_model_endpoints",
    },
    RouteSpec {
        method: "GET",
        path: "/key",
        operation: "get_current_key",
    },
    RouteSpec {
        method: "GET",
        path: "/keys",
        operation: "list_keys",
    },
    RouteSpec {
        method: "POST",
        path: "/keys",
        operation: "create_key",
    },
    RouteSpec {
        method: "DELETE",
        path: "/keys/{hash}",
        operation: "delete_key",
    },
    RouteSpec {
        method: "GET",
        path: "/keys/{hash}",
        operation: "get_key",
    },
    RouteSpec {
        method: "PATCH",
        path: "/keys/{hash}",
        operation: "update_key",
    },
    RouteSpec {
        method: "POST",
        path: "/messages",
        operation: "create_message",
    },
    RouteSpec {
        method: "GET",
        path: "/model/{author}/{slug}",
        operation: "get_model",
    },
    RouteSpec {
        method: "GET",
        path: "/models",
        operation: "list_models",
    },
    RouteSpec {
        method: "GET",
        path: "/models/{author}/{slug}/endpoints",
        operation: "list_model_endpoints",
    },
    RouteSpec {
        method: "GET",
        path: "/models/count",
        operation: "get_models_count",
    },
    RouteSpec {
        method: "GET",
        path: "/models/user",
        operation: "list_user_models",
    },
    RouteSpec {
        method: "GET",
        path: "/observability/destinations",
        operation: "list_observability_destinations",
    },
    RouteSpec {
        method: "POST",
        path: "/observability/destinations",
        operation: "create_observability_destination",
    },
    RouteSpec {
        method: "DELETE",
        path: "/observability/destinations/{id}",
        operation: "delete_observability_destination",
    },
    RouteSpec {
        method: "GET",
        path: "/observability/destinations/{id}",
        operation: "get_observability_destination",
    },
    RouteSpec {
        method: "PATCH",
        path: "/observability/destinations/{id}",
        operation: "update_observability_destination",
    },
    RouteSpec {
        method: "GET",
        path: "/organization/members",
        operation: "list_organization_members",
    },
    RouteSpec {
        method: "GET",
        path: "/presets",
        operation: "list_presets",
    },
    RouteSpec {
        method: "GET",
        path: "/presets/{slug}",
        operation: "get_preset",
    },
    RouteSpec {
        method: "POST",
        path: "/presets/{slug}/chat/completions",
        operation: "create_preset_from_chat_completion",
    },
    RouteSpec {
        method: "POST",
        path: "/presets/{slug}/messages",
        operation: "create_preset_from_message",
    },
    RouteSpec {
        method: "POST",
        path: "/presets/{slug}/responses",
        operation: "create_preset_from_response",
    },
    RouteSpec {
        method: "GET",
        path: "/presets/{slug}/versions",
        operation: "list_preset_versions",
    },
    RouteSpec {
        method: "GET",
        path: "/presets/{slug}/versions/{version}",
        operation: "get_preset_version",
    },
    RouteSpec {
        method: "GET",
        path: "/providers",
        operation: "list_providers",
    },
    RouteSpec {
        method: "POST",
        path: "/rerank",
        operation: "create_rerank",
    },
    RouteSpec {
        method: "POST",
        path: "/responses",
        operation: "create_response",
    },
    RouteSpec {
        method: "POST",
        path: "/videos",
        operation: "create_video",
    },
    RouteSpec {
        method: "GET",
        path: "/videos/{job_id}",
        operation: "get_video",
    },
    RouteSpec {
        method: "GET",
        path: "/videos/{job_id}/content",
        operation: "download_video_content",
    },
    RouteSpec {
        method: "GET",
        path: "/videos/models",
        operation: "list_video_models",
    },
    RouteSpec {
        method: "GET",
        path: "/workspaces",
        operation: "list_workspaces",
    },
    RouteSpec {
        method: "POST",
        path: "/workspaces",
        operation: "create_workspace",
    },
    RouteSpec {
        method: "DELETE",
        path: "/workspaces/{id}",
        operation: "delete_workspace",
    },
    RouteSpec {
        method: "GET",
        path: "/workspaces/{id}",
        operation: "get_workspace",
    },
    RouteSpec {
        method: "PATCH",
        path: "/workspaces/{id}",
        operation: "update_workspace",
    },
    RouteSpec {
        method: "GET",
        path: "/workspaces/{id}/budgets",
        operation: "list_workspace_budgets",
    },
    RouteSpec {
        method: "DELETE",
        path: "/workspaces/{id}/budgets/{interval}",
        operation: "delete_workspace_budget",
    },
    RouteSpec {
        method: "PUT",
        path: "/workspaces/{id}/budgets/{interval}",
        operation: "upsert_workspace_budget",
    },
    RouteSpec {
        method: "POST",
        path: "/workspaces/{id}/members/add",
        operation: "bulk_add_workspace_members",
    },
    RouteSpec {
        method: "POST",
        path: "/workspaces/{id}/members/remove",
        operation: "bulk_remove_workspace_members",
    },
];

#[cfg(test)]
mod tests {
    use super::NON_DEPRECATED_ROUTES;

    #[test]
    fn route_snapshot_covers_all_current_non_deprecated_routes() {
        assert_eq!(NON_DEPRECATED_ROUTES.len(), 86);
        assert!(
            !NON_DEPRECATED_ROUTES
                .iter()
                .any(|route| route.path == "/credits/coinbase")
        );
    }
}
