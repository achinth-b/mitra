mod helpers;

use helpers::*;
use mitra_backend::models::*;
use mitra_backend::repositories::*;
use mitra_backend::services::*;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

/// End-to-end test: Complete flow from group creation to settlement
#[sqlx::test]
async fn test_complete_e2e_flow(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    // Step 1: Create users
    let admin = create_test_user(&db, "admin_wallet_123").await;
    let user1 = create_test_user(&db, "user1_wallet_456").await;
    let user2 = create_test_user(&db, "user2_wallet_789").await;

    assert_eq!(admin.wallet_address, "admin_wallet_123");
    assert_eq!(user1.wallet_address, "user1_wallet_456");
    assert_eq!(user2.wallet_address, "user2_wallet_789");

    // Step 2: Create friend group
    let group = db.friend_group_repo
        .create("group_pubkey_e2e", "E2E Test Group", "admin_wallet_123")
        .await
        .expect("Failed to create group");

    assert_eq!(group.name, "E2E Test Group");
    assert_eq!(group.admin_wallet, "admin_wallet_123");

    // Step 3: Add members
    db.group_member_repo
        .add_member(group.id, admin.id, MemberRole::Admin)
        .await
        .expect("Failed to add admin");

    db.group_member_repo
        .add_member(group.id, user1.id, MemberRole::Member)
        .await
        .expect("Failed to add user1");

    db.group_member_repo
        .add_member(group.id, user2.id, MemberRole::Member)
        .await
        .expect("Failed to add user2");

    // Verify members
    let members = db.group_member_repo
        .find_by_group(group.id)
        .await
        .expect("Failed to find members");

    assert_eq!(members.len(), 3);

    // Step 4: Create event
    let outcomes = serde_json::json!(["YES", "NO"]);
    let event = db.event_repo
        .create(
            group.id,
            "E2E Test Event: Will it rain?",
            Some("A test event for E2E testing"),
            &outcomes,
            "manual",
            None,
        )
        .await
        .expect("Failed to create event");

    assert_eq!(event.title, "E2E Test Event: Will it rain?");
    assert_eq!(event.status, "active");
    assert_eq!(event.settlement_type, "manual");

    // Step 5: Place bets
    let bet1 = create_test_bet(
        &db,
        event.id,
        user1.id,
        "YES",
        Decimal::new(100, 0), // 100 shares
        Decimal::new(50, 2),  // 0.50 price
        Decimal::new(50, 0), // 50 USDC
    )
    .await;

    let bet2 = create_test_bet(
        &db,
        event.id,
        user2.id,
        "NO",
        Decimal::new(100, 0), // 100 shares
        Decimal::new(50, 2),  // 0.50 price
        Decimal::new(50, 0), // 50 USDC
    )
    .await;

    assert_eq!(bet1.outcome, "YES");
    assert_eq!(bet2.outcome, "NO");

    // Step 6: Verify bets
    let bets = db.bet_repo
        .find_by_event(event.id)
        .await
        .expect("Failed to find bets");

    assert_eq!(bets.len(), 2);

    // Step 7: Get volume
    let total_volume = db.bet_repo
        .get_total_volume_for_event(event.id)
        .await
        .expect("Failed to get volume")
        .expect("Volume should exist");

    assert_eq!(total_volume, Decimal::new(100, 0)); // 50 + 50 = 100

    // Step 8: Get volume by outcome
    let yes_volume = db.bet_repo
        .get_volume_by_outcome(event.id, "YES")
        .await
        .expect("Failed to get YES volume")
        .unwrap_or(Decimal::ZERO);

    let no_volume = db.bet_repo
        .get_volume_by_outcome(event.id, "NO")
        .await
        .expect("Failed to get NO volume")
        .unwrap_or(Decimal::ZERO);

    assert_eq!(yes_volume, Decimal::new(50, 0));
    assert_eq!(no_volume, Decimal::new(50, 0));

    // Step 9: Settle event
    let settled_event = db.event_repo
        .update_status(event.id, EventStatus::Resolved)
        .await
        .expect("Failed to settle event");

    assert_eq!(settled_event.status, "resolved");

    // Step 10: Verify event is settled
    let final_event = db.event_repo
        .find_by_id(event.id)
        .await
        .expect("Failed to find event")
        .expect("Event should exist");

    assert!(final_event.is_resolved());
}

/// E2E test: Multiple events in same group
#[sqlx::test]
async fn test_multiple_events_in_group(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    // Create second event
    let outcomes2 = serde_json::json!(["WIN", "LOSE"]);
    let event2 = create_test_event(
        &db,
        fixtures.friend_group.id,
        "Second Event",
        None,
        &outcomes2,
        SettlementType::Manual,
    )
    .await;

    // Verify both events exist
    let events = db.event_repo
        .find_by_group(fixtures.friend_group.id)
        .await
        .expect("Failed to find events");

    assert_eq!(events.len(), 2);
}

/// E2E test: User places multiple bets
#[sqlx::test]
async fn test_user_multiple_bets(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    // User1 places multiple bets
    let bet1 = create_test_bet(
        &db,
        fixtures.event.id,
        fixtures.user1.id,
        "Yes",
        Decimal::new(50, 0),
        Decimal::new(50, 2),
        Decimal::new(25, 0),
    )
    .await;

    let bet2 = create_test_bet(
        &db,
        fixtures.event.id,
        fixtures.user1.id,
        "Yes",
        Decimal::new(30, 0),
        Decimal::new(55, 2),
        Decimal::new(16, 5),
    )
    .await;

    // Get user's bets
    let user_bets = db.bet_repo
        .find_by_user(fixtures.user1.id)
        .await
        .expect("Failed to find user bets");

    assert_eq!(user_bets.len(), 2);

    // Get user's bets for this specific event
    let user_event_bets = db.bet_repo
        .find_by_user_and_event(fixtures.user1.id, fixtures.event.id)
        .await
        .expect("Failed to find user event bets");

    assert_eq!(user_event_bets.len(), 2);
}

/// E2E test: Group member removal
#[sqlx::test]
async fn test_member_removal(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    // Remove user2
    db.group_member_repo
        .remove_member(fixtures.friend_group.id, fixtures.user2.id)
        .await
        .expect("Failed to remove member");

    // Verify user2 is no longer a member
    let is_member = db.group_member_repo
        .is_member(fixtures.friend_group.id, fixtures.user2.id)
        .await
        .expect("Failed to check membership");

    assert!(!is_member);

    // Verify other members still exist
    let members = db.group_member_repo
        .find_by_group(fixtures.friend_group.id)
        .await
        .expect("Failed to find members");

    assert_eq!(members.len(), 2); // admin + user1
}

/// E2E test: Event cancellation
#[sqlx::test]
async fn test_event_cancellation(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    // Cancel event
    let cancelled = db.event_repo
        .update_status(fixtures.event.id, EventStatus::Cancelled)
        .await
        .expect("Failed to cancel event");

    assert_eq!(cancelled.status, "cancelled");

    // Verify active events don't include cancelled event
    let active_events = db.event_repo
        .find_active_events()
        .await
        .expect("Failed to find active events");

    assert!(!active_events.iter().any(|e| e.id == fixtures.event.id));
}

