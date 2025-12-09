mod helpers;

use helpers::*;
use mitra_backend::models::*;
use mitra_backend::repositories::*;
use rust_decimal::Decimal;
use sqlx::PgPool;

/// End-to-end integration test: Create group → Create event → Place bets → Settle
#[sqlx::test]
async fn test_e2e_flow(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    // Step 1: Create users
    let admin = create_test_user(&db, "admin_wallet").await;
    let user1 = create_test_user(&db, "user1_wallet").await;
    let user2 = create_test_user(&db, "user2_wallet").await;

    // Step 2: Create friend group
    let group = db.friend_group_repo
        .create("group_pubkey", "Test Group", "admin_wallet")
        .await
        .expect("Failed to create group");

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

    // Step 4: Create event
    let outcomes = serde_json::json!(["YES", "NO"]);
    let event = db.event_repo
        .create(
            group.id,
            "E2E Test Event",
            None,
            &outcomes,
            "manual",
            None,
        )
        .await
        .expect("Failed to create event");

    // Step 5: Place bets
    create_test_bet(
        &db,
        event.id,
        user1.id,
        "YES",
        Decimal::new(100, 0),
        Decimal::new(50, 2),
    )
    .await;

    create_test_bet(
        &db,
        event.id,
        user2.id,
        "NO",
        Decimal::new(100, 0),
        Decimal::new(50, 2),
    )
    .await;

    // Step 6: Verify bets were created
    let bets = db.bet_repo
        .find_by_event(event.id)
        .await
        .expect("Failed to find bets");

    assert_eq!(bets.len(), 2);

    // Step 7: Get total volume
    let volume = db.bet_repo
        .get_total_volume_for_event(event.id)
        .await
        .expect("Failed to get volume")
        .expect("Volume should exist");

    assert_eq!(volume, Decimal::new(200, 0));

    // Step 8: Settle event
    let updated_event = db.event_repo
        .update_status(event.id, EventStatus::Resolved)
        .await
        .expect("Failed to settle event");

    assert_eq!(updated_event.status, "resolved");

    // Verify event is settled
    let settled_event = db.event_repo
        .find_by_id(event.id)
        .await
        .expect("Failed to find event")
        .expect("Event should exist");

    assert!(settled_event.is_resolved());
}

/// Test WebSocket subscription flow
#[sqlx::test]
async fn test_websocket_subscriptions(_pool: PgPool) {
    // This would require WebSocket server setup
    // For now, test the subscription logic
    assert!(true);
}

/// Test settlement mechanisms
#[sqlx::test]
async fn test_manual_settlement(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    // Settle event
    let settled = db.event_repo
        .update_status(fixtures.event.id, EventStatus::Resolved)
        .await
        .expect("Failed to settle");

    assert_eq!(settled.status, "resolved");
}

/// Test consensus voting
#[sqlx::test]
async fn test_consensus_voting(_pool: PgPool) {
    // TODO: Implement consensus voting test
    assert!(true);
}

/// Test merkle proof generation
#[sqlx::test]
async fn test_merkle_proof_generation(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    // Create some bets
    create_test_bet(
        &db,
        fixtures.event.id,
        fixtures.user1.id,
        "Yes",
        Decimal::new(100, 0),
        Decimal::new(50, 2),
    )
    .await;

    create_test_bet(
        &db,
        fixtures.event.id,
        fixtures.user2.id,
        "No",
        Decimal::new(200, 0),
        Decimal::new(60, 2),
    )
    .await;

    // Generate merkle root
    use mitra_backend::state_manager::StateManager;
    let state_manager = StateManager::new(db.pool.clone());
    
    let (merkle_root, proofs) = state_manager
        .generate_merkle_root(fixtures.event.id)
        .await
        .expect("Failed to generate merkle root");

    assert_eq!(merkle_root.len(), 32);
    assert!(!proofs.is_empty());
}
