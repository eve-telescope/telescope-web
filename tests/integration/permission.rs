//! tests/Feature/PermissionTest.php: what viewers, members, managers, the
//! owner and outsiders may do in a network.

use serde_json::json;
use time::Duration;

use crate::common::TestApp;
use crate::common::factories::{NewUser, USER};

struct Setup {
    app: TestApp,
    network: i64,
    owner: String,
    manager: String,
    member: String,
    viewer: String,
    outsider: String,
}

/// The PHP suite's `beforeEach`.
async fn setup() -> Setup {
    let app = TestApp::new().await;
    app.create_corporation(98_000_001, "Test Corp", "TSTC", None)
        .await;
    let owner = app
        .create_user(NewUser {
            corporation_id: Some(98_000_001),
            ..NewUser::default()
        })
        .await;
    let manager = app.user().await;
    let member = app.user().await;
    let viewer = app.user().await;
    let outsider = app.user().await;
    let network = app.create_network("Permission Test", owner.id).await;
    app.grant_owner(network, &owner).await;
    app.grant_user(network, &manager, "manager").await;
    app.grant_user(network, &member, "member").await;
    app.grant_user(network, &viewer, "viewer").await;
    Setup {
        network,
        owner: app.token(&owner).await,
        manager: app.token(&manager).await,
        member: app.token(&member).await,
        viewer: app.token(&viewer).await,
        outsider: app.token(&outsider).await,
        app,
    }
}

impl Setup {
    fn path(&self, rest: &str) -> String {
        format!("/api/networks/{}{rest}", self.network)
    }

    async fn create_entry(&self, token: &str) -> u16 {
        self.app
            .post_json(
                token,
                &self.path("/entries"),
                &json!({ "entity_type": "character", "entity_id": 1, "entity_name": "Test" }),
            )
            .await
            .status
            .as_u16()
    }

    async fn share_scan(&self, token: &str) -> u16 {
        self.app
            .post_json(
                token,
                &self.path("/scans"),
                &json!({ "scan_type": "local", "raw_text": "Pilot One" }),
            )
            .await
            .status
            .as_u16()
    }

    async fn grant(&self, token: &str) -> u16 {
        let target = self.app.user().await;
        self.app
            .post_json(
                token,
                &self.path("/access"),
                &json!({
                    "accessible_type": "character",
                    "accessible_id": target.character_id,
                    "accessible_name": target.character_name,
                    "permission": "viewer",
                }),
            )
            .await
            .status
            .as_u16()
    }

    async fn rename(&self, token: &str) -> u16 {
        self.app
            .put_json(token, &self.path(""), &json!({ "name": "Updated" }))
            .await
            .status
            .as_u16()
    }
}

#[tokio::test]
async fn viewer_can_view_network_detail() {
    let s = setup().await;
    s.app
        .get_json(&s.viewer, &s.path(""))
        .await
        .assert_status(200);
}

#[tokio::test]
async fn outsider_cannot_view_network_detail() {
    let s = setup().await;
    s.app
        .get_json(&s.outsider, &s.path(""))
        .await
        .assert_status(403);
}

#[tokio::test]
async fn member_can_create_entries() {
    let s = setup().await;
    assert_eq!(s.create_entry(&s.member).await, 201);
}

#[tokio::test]
async fn viewer_cannot_create_entries() {
    let s = setup().await;
    assert_eq!(s.create_entry(&s.viewer).await, 403);
}

#[tokio::test]
async fn outsider_cannot_create_entries() {
    let s = setup().await;
    assert_eq!(s.create_entry(&s.outsider).await, 403);
}

#[tokio::test]
async fn member_can_share_scans() {
    let s = setup().await;
    assert_eq!(s.share_scan(&s.member).await, 201);
}

#[tokio::test]
async fn viewer_cannot_share_scans() {
    let s = setup().await;
    assert_eq!(s.share_scan(&s.viewer).await, 403);
}

#[tokio::test]
async fn viewer_can_list_scans() {
    let s = setup().await;
    s.app
        .get_json(&s.viewer, &s.path("/scans"))
        .await
        .assert_status(200);
}

#[tokio::test]
async fn outsider_cannot_list_scans() {
    let s = setup().await;
    s.app
        .get_json(&s.outsider, &s.path("/scans"))
        .await
        .assert_status(403);
}

#[tokio::test]
async fn manager_can_grant_access() {
    let s = setup().await;
    assert_eq!(s.grant(&s.manager).await, 201);
}

#[tokio::test]
async fn member_cannot_grant_access() {
    let s = setup().await;
    assert_eq!(s.grant(&s.member).await, 403);
}

#[tokio::test]
async fn viewer_cannot_grant_access() {
    let s = setup().await;
    assert_eq!(s.grant(&s.viewer).await, 403);
}

#[tokio::test]
async fn manager_can_update_network() {
    let s = setup().await;
    assert_eq!(s.rename(&s.manager).await, 200);
}

#[tokio::test]
async fn member_cannot_update_network() {
    let s = setup().await;
    assert_eq!(s.rename(&s.member).await, 403);
}

#[tokio::test]
async fn only_owner_can_delete_network() {
    let s = setup().await;
    s.app
        .delete_json(&s.manager, &s.path(""))
        .await
        .assert_status(403);
    s.app
        .delete_json(&s.owner, &s.path(""))
        .await
        .assert_status(200);
}

#[tokio::test]
async fn expired_access_is_denied() {
    let s = setup().await;
    let expired = s.app.user().await;
    s.app
        .grant(
            s.network,
            USER,
            expired.character_id,
            "member",
            false,
            Some(s.app.now() - Duration::days(1)),
        )
        .await;
    let token = s.app.token(&expired).await;
    s.app.get_json(&token, &s.path("")).await.assert_status(403);
}

// ---------------------------------------------------------------------------
// Beyond the PHP suite.
// ---------------------------------------------------------------------------

/// The highest permission of the matching rows wins, and an access that
/// expires at this very second is already expired.
#[tokio::test]
async fn the_highest_unexpired_permission_wins() {
    let s = setup().await;
    s.app
        .create_corporation(98_000_002, "Second Corp", "SEC", None)
        .await;
    let pilot = s
        .app
        .create_user(NewUser {
            corporation_id: Some(98_000_002),
            ..NewUser::default()
        })
        .await;
    s.app.grant_user(s.network, &pilot, "viewer").await;
    s.app
        .grant(
            s.network,
            "App\\Models\\Corporation",
            98_000_002,
            "manager",
            false,
            Some(s.app.now()),
        )
        .await;
    s.app
        .grant(
            s.network,
            "App\\Models\\Alliance",
            98_000_002,
            "member",
            false,
            Some(s.app.now() + Duration::seconds(1)),
        )
        .await;
    let token = s.app.token(&pilot).await;
    // Member through the third row (ids match whatever their type), not
    // manager through the expired one.
    assert_eq!(s.create_entry(&token).await, 201);
    assert_eq!(s.grant(&token).await, 403);
}

/// Deleting needs an owner row with the caller's character; a manager
/// row is not enough.
#[tokio::test]
async fn deleting_needs_the_owner_row() {
    let s = setup().await;
    s.app
        .delete_json(&s.member, &s.path(""))
        .await
        .assert_status(403);
    s.app
        .delete_json(&s.outsider, &s.path(""))
        .await
        .assert_status(403);
    assert_eq!(s.app.count("intel_networks").await, 1);
}
