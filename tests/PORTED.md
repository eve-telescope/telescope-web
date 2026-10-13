# Ported tests

Every test of the Laravel app (`main`: `tests/Unit`, `tests/Feature`,
Pest) and the test that replaces it. Feature tests run against the real
router and database in `tests/integration` (one module per PHP file, same
names). The PHP tests signed in with `actingAs`; the ports use real
Sanctum tokens, as the desktop clients do. Where a PHP test pinned less
than the whole answer (`assertJsonStructure`, `toHaveKeys`), the port pins
the exact JSON, keys in order. No PHP test pinned a Laravel bug.

| PHP test | New test |
| --- | --- |
| Unit/ExampleTest "that true is true" | dropped: asserts nothing about the app |
| Feature/ExampleTest "returns a successful response" | `frontend/e2e/routes.spec.ts` "returns a successful response", `pages::the_home_page_props` |
| Feature/EntitySearchTest "searches characters by name" | `entity_search::searches_characters_by_name` |
| Feature/EntitySearchTest "searches corporations by name" | `entity_search::searches_corporations_by_name` |
| Feature/EntitySearchTest "searches corporations by ticker" | `entity_search::searches_corporations_by_ticker` |
| Feature/EntitySearchTest "searches alliances by name" | `entity_search::searches_alliances_by_name` |
| Feature/EntitySearchTest "searches all categories when no category specified" | `entity_search::searches_all_categories_when_no_category_specified` |
| Feature/EntitySearchTest "returns results from multiple categories" | `entity_search::returns_results_from_multiple_categories` |
| Feature/EntitySearchTest "requires query parameter" | `entity_search::requires_query_parameter` |
| Feature/EntitySearchTest "requires query to be at least 2 characters" | `entity_search::requires_query_to_be_at_least_2_characters` |
| Feature/EntitySearchTest "rejects invalid category" | `entity_search::rejects_invalid_category` |
| Feature/EntitySearchTest "never returns fields that would break Rust deserialization" | `entity_search::never_returns_fields_that_would_break_rust_deserialization` |
| Feature/IntelEntryTest "allows member to create an entry" | `intel_entry::allows_member_to_create_an_entry` |
| Feature/IntelEntryTest "allows note-only entries without tags" | `intel_entry::allows_note_only_entries_without_tags` |
| Feature/IntelEntryTest "denies viewer from creating entries" | `intel_entry::denies_viewer_from_creating_entries` |
| Feature/IntelEntryTest "upserts entry for same entity" | `intel_entry::upserts_entry_for_same_entity` |
| Feature/IntelEntryTest "allows member to update an entry" | `intel_entry::allows_member_to_update_an_entry` |
| Feature/IntelEntryTest "allows member to delete an entry" | `intel_entry::allows_member_to_delete_an_entry` |
| Feature/IntelEntryTest "denies viewer from deleting entries" | `intel_entry::denies_viewer_from_deleting_entries` |
| Feature/IntelEntryTest "allows viewer to list entries" | `intel_entry::allows_viewer_to_list_entries` |
| Feature/IntelEntryTest "returns entries matching entity IDs across accessible networks" | `intel_entry::returns_entries_matching_entity_ids_across_accessible_networks` |
| Feature/IntelEntryTest "excludes entries from inaccessible networks" | `intel_entry::excludes_entries_from_inaccessible_networks` |
| Feature/IntelNetworkTest "lists networks the user has access to" | `intel_network::lists_networks_the_user_has_access_to` |
| Feature/IntelNetworkTest "excludes networks the user has no access to" | `intel_network::excludes_networks_the_user_has_no_access_to` |
| Feature/IntelNetworkTest "includes networks via corporation access" | `intel_network::includes_networks_via_corporation_access` |
| Feature/IntelNetworkTest "creates a network with owner access" | `intel_network::creates_a_network_with_owner_access` |
| Feature/IntelNetworkTest "rejects empty network name" | `intel_network::rejects_empty_network_name` |
| Feature/IntelNetworkTest "shows network detail with accesses and entries" | `intel_network::shows_network_detail_with_accesses_and_entries` |
| Feature/IntelNetworkTest "denies show to non-members" | `intel_network::denies_show_to_non_members` |
| Feature/IntelNetworkTest "allows manager to update network name" | `intel_network::allows_manager_to_update_network_name` |
| Feature/IntelNetworkTest "denies viewer from updating network" | `intel_network::denies_viewer_from_updating_network` |
| Feature/IntelNetworkTest "allows owner to delete network" | `intel_network::allows_owner_to_delete_network` |
| Feature/IntelNetworkTest "denies non-owner from deleting network" | `intel_network::denies_non_owner_from_deleting_network` |
| Feature/NetworkAccessTest "can grant character access and returns correct JSON shape" | `network_access::can_grant_character_access_and_returns_correct_json_shape` |
| Feature/NetworkAccessTest "can grant corporation access and returns correct JSON shape" | `network_access::can_grant_corporation_access_and_returns_correct_json_shape` |
| Feature/NetworkAccessTest "can grant alliance access and returns correct JSON shape" | `network_access::can_grant_alliance_access_and_returns_correct_json_shape` |
| Feature/NetworkAccessTest "never returns PHP class names in accessible_type" | `network_access::never_returns_php_class_names_in_accessible_type` |
| Feature/NetworkAccessTest "returns accesses with entity data in network detail" | `network_access::returns_accesses_with_entity_data_in_network_detail` |
| Feature/NetworkAccessTest "returns formatted accesses in index" | `network_access::returns_formatted_accesses_in_index` |
| Feature/NetworkAccessTest "returns formatted access after update" | `network_access::returns_formatted_access_after_update` |
| Feature/NetworkAccessTest "prevents modifying owner access" | `network_access::prevents_modifying_owner_access` |
| Feature/NetworkAccessTest "prevents revoking owner access" | `network_access::prevents_revoking_owner_access` |
| Feature/NetworkAccessTest "can revoke non-owner access" | `network_access::can_revoke_non_owner_access` |
| Feature/NetworkAccessTest "denies access management to non-managers" | `network_access::denies_access_management_to_non_managers` |
| Feature/NetworkScanTest "allows member to share a local scan" | `network_scan::allows_member_to_share_a_local_scan` |
| Feature/NetworkScanTest "allows member to share a dscan" | `network_scan::allows_member_to_share_a_dscan` |
| Feature/NetworkScanTest "denies viewer from sharing scans" | `network_scan::denies_viewer_from_sharing_scans` |
| Feature/NetworkScanTest "rejects invalid scan type" | `network_scan::rejects_invalid_scan_type` |
| Feature/NetworkScanTest "rejects missing raw text" | `network_scan::rejects_missing_raw_text` |
| Feature/NetworkScanTest "allows viewer to list scan history" | `network_scan::allows_viewer_to_list_scan_history` |
| Feature/NetworkScanTest "returns scans in newest-first order" | `network_scan::returns_scans_in_newest_first_order` |
| Feature/NetworkScanTest "paginates scan history" | `network_scan::paginates_scan_history` |
| Feature/PermissionTest "viewer can view network detail" | `permission::viewer_can_view_network_detail` |
| Feature/PermissionTest "outsider cannot view network detail" | `permission::outsider_cannot_view_network_detail` |
| Feature/PermissionTest "member can create entries" | `permission::member_can_create_entries` |
| Feature/PermissionTest "viewer cannot create entries" | `permission::viewer_cannot_create_entries` |
| Feature/PermissionTest "outsider cannot create entries" | `permission::outsider_cannot_create_entries` |
| Feature/PermissionTest "member can share scans" | `permission::member_can_share_scans` |
| Feature/PermissionTest "viewer cannot share scans" | `permission::viewer_cannot_share_scans` |
| Feature/PermissionTest "viewer can list scans" | `permission::viewer_can_list_scans` |
| Feature/PermissionTest "outsider cannot list scans" | `permission::outsider_cannot_list_scans` |
| Feature/PermissionTest "manager can grant access" | `permission::manager_can_grant_access` |
| Feature/PermissionTest "member cannot grant access" | `permission::member_cannot_grant_access` |
| Feature/PermissionTest "viewer cannot grant access" | `permission::viewer_cannot_grant_access` |
| Feature/PermissionTest "manager can update network" | `permission::manager_can_update_network` |
| Feature/PermissionTest "member cannot update network" | `permission::member_cannot_update_network` |
| Feature/PermissionTest "only owner can delete network" | `permission::only_owner_can_delete_network` |
| Feature/PermissionTest "expired access is denied" | `permission::expired_access_is_denied` |
| Feature/PurgeOldScansTest "deletes scans older than configured retention" | `purge_old_scans::deletes_scans_older_than_configured_retention` |
| Feature/PurgeOldScansTest "respects --days override" | `purge_old_scans::respects_days_override` |
| Feature/PurgeOldScansTest "keeps all scans when none are expired" | `purge_old_scans::keeps_all_scans_when_none_are_expired` |
| Feature/PurgeOldScansTest "respects config value" | `purge_old_scans::respects_config_value` |

## New test areas

The PHP suite covered the intel network API. Everything else is new:

- `golden_api`: every request recorded from the Laravel reference
  (221 scenarios: every route, guests and each permission, validation,
  errors, CORS, pagination), replayed with the documented normalizations
  and the FIXES.md overrides.
- `golden_pusher`, `pusher_protocol`, `events`: Reverb's frames replayed
  byte for byte against the websocket server, the server's ping and
  prune, the v0.5 client's flow, and the events of every mutation.
- `broadcasting_auth`, `cors`: channel signatures and the Tauri
  webview's preflight.
- `api_auth`, `api_tokens`, `web_session`, `eve_login`, `demo`: Sanctum
  tokens and `last_used_at`, the token endpoints, the website session,
  the EVE SSO login against mock SSO, JWKS and ESI servers, the demo
  login.
- `share`, `pages`, `github`: share links and their view counter, the
  page props, the latest release and its cache.
- `affiliations`, `background`: the hourly tasks against a mock ESI.
- `golden_php`: `Str::slug` as Laravel answered it.
- `api_errors`, `api_misc`, `harness`: error answers, the routes Laravel
  added, the route contract.
- frontend vitest suites: the head tags, the share and opener logic, the
  proxy contract, cookies and the Inertia reload.
- Playwright (`frontend/e2e`): the route contract through Caddy, the
  pages in a browser, the head of every page, laravel-echo and pusher-js
  against the websocket, and screenshots against the Laravel pages.
