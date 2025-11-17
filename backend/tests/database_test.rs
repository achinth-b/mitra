mod helpers;

use helpers::*;
use mitra_backend::models::*;
use mitra_backend::repositories::*;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================================
// Connection Pool Tests
// ============================================================================

#[sqlx::test]
async fn test_connection_pool_creation(pool: PgPool) {
    // Test that we can execute a simple query
    let result = sqlx::query("SELECT 1 as test")
        .fetch_one(&pool)
        .await;

    assert!(result.is_ok());
    let row = result.unwrap();
    let value: i32 = row.get("test");
    assert_eq!(value, 1);
}

#[sqlx::test]
async fn test_connection_pool_multiple_queries(pool: PgPool) {
    // Test that we can execute multiple queries
    for i in 1..=5 {
        let result = sqlx::query(&format!("SELECT {} as test", i))
            .fetch_one(&pool)
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Migration Tests
// ============================================================================

#[sqlx::test]
async fn test_migrations_ran(pool: PgPool) {
    // Verify that all tables exist
    let tables = vec!["users", "friend_groups", "group_members", "events", "bets"];

    for table in tables {
        let result = sqlx::query(&format!(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_name = '{}'
            )",
            table
        ))
        .fetch_one(&pool)
        .await;

        assert!(result.is_ok());
        let exists: bool = result.unwrap().get(0);
        assert!(exists, "Table {} should exist", table);
    }
}

// ============================================================================
// User Repository Tests
// ============================================================================

#[sqlx::test]
async fn test_user_create(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let user = db.user_repo
        .create("test_wallet_123")
        .await
        .expect("Failed to create user");

    assert_eq!(user.wallet_address, "test_wallet_123");
    assert!(!user.id.is_nil());
}

#[sqlx::test]
async fn test_user_find_by_id(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let created_user = db.user_repo
        .create("test_wallet_456")
        .await
        .expect("Failed to create user");

    let found_user = db.user_repo
        .find_by_id(created_user.id)
        .await
        .expect("Failed to find user")
        .expect("User should exist");

    assert_users_equal(&created_user, &found_user);
}

#[sqlx::test]
async fn test_user_find_by_wallet(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let created_user = db.user_repo
        .create("test_wallet_789")
        .await
        .expect("Failed to create user");

    let found_user = db.user_repo
        .find_by_wallet("test_wallet_789")
        .await
        .expect("Failed to find user")
        .expect("User should exist");

    assert_users_equal(&created_user, &found_user);
}

#[sqlx::test]
async fn test_user_find_or_create_existing(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let created_user = db.user_repo
        .create("test_wallet_existing")
        .await
        .expect("Failed to create user");

    let found_or_created = db.user_repo
        .find_or_create_by_wallet("test_wallet_existing")
        .await
        .expect("Failed to find or create user");

    assert_eq!(created_user.id, found_or_created.id);
}

#[sqlx::test]
async fn test_user_find_or_create_new(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let user = db.user_repo
        .find_or_create_by_wallet("test_wallet_new")
        .await
        .expect("Failed to find or create user");

    assert_eq!(user.wallet_address, "test_wallet_new");
}

// ============================================================================
// Friend Group Repository Tests
// ============================================================================

#[sqlx::test]
async fn test_friend_group_create(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let group = db.friend_group_repo
        .create("solana_pubkey_123", "Test Group", "admin_wallet")
        .await
        .expect("Failed to create group");

    assert_eq!(group.name, "Test Group");
    assert_eq!(group.solana_pubkey, "solana_pubkey_123");
    assert_eq!(group.admin_wallet, "admin_wallet");
}

#[sqlx::test]
async fn test_friend_group_find_by_id(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let created_group = db.friend_group_repo
        .create("solana_pubkey_456", "Test Group 2", "admin_wallet_2")
        .await
        .expect("Failed to create group");

    let found_group = db.friend_group_repo
        .find_by_id(created_group.id)
        .await
        .expect("Failed to find group")
        .expect("Group should exist");

    assert_groups_equal(&created_group, &found_group);
}

#[sqlx::test]
async fn test_friend_group_find_by_solana_pubkey(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let created_group = db.friend_group_repo
        .create("solana_pubkey_789", "Test Group 3", "admin_wallet_3")
        .await
        .expect("Failed to create group");

    let found_group = db.friend_group_repo
        .find_by_solana_pubkey("solana_pubkey_789")
        .await
        .expect("Failed to find group")
        .expect("Group should exist");

    assert_groups_equal(&created_group, &found_group);
}

#[sqlx::test]
async fn test_friend_group_update_name(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let created_group = db.friend_group_repo
        .create("solana_pubkey_update", "Old Name", "admin_wallet")
        .await
        .expect("Failed to create group");

    let updated_group = db.friend_group_repo
        .update_name(created_group.id, "New Name")
        .await
        .expect("Failed to update group");

    assert_eq!(updated_group.name, "New Name");
    assert_eq!(updated_group.id, created_group.id);
}

#[sqlx::test]
async fn test_friend_group_delete(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let created_group = db.friend_group_repo
        .create("solana_pubkey_delete", "To Delete", "admin_wallet")
        .await
        .expect("Failed to create group");

    let deleted = db.friend_group_repo
        .delete(created_group.id)
        .await
        .expect("Failed to delete group");

    assert!(deleted);

    let found_group = db.friend_group_repo
        .find_by_id(created_group.id)
        .await
        .expect("Failed to query");

    assert!(found_group.is_none());
}

// ============================================================================
// Group Member Repository Tests
// ============================================================================

#[sqlx::test]
async fn test_group_member_add(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let user = create_test_user(&db, "member_wallet").await;
    let group = create_test_group(&db, "group_pubkey", "Test Group", "admin_wallet").await;

    let member = db.group_member_repo
        .add_member(group.id, user.id, MemberRole::Member)
        .await
        .expect("Failed to add member");

    assert_eq!(member.group_id, group.id);
    assert_eq!(member.user_id, user.id);
    assert_eq!(member.role, "member");
}

#[sqlx::test]
async fn test_group_member_find_by_group(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let members = db.group_member_repo
        .find_by_group(fixtures.friend_group.id)
        .await
        .expect("Failed to find members");

    assert_eq!(members.len(), 3);
}

#[sqlx::test]
async fn test_group_member_find_by_user(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let groups = db.group_member_repo
        .find_by_user(fixtures.user1.id)
        .await
        .expect("Failed to find groups");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].group_id, fixtures.friend_group.id);
}

#[sqlx::test]
async fn test_group_member_remove(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let removed = db.group_member_repo
        .remove_member(fixtures.friend_group.id, fixtures.user2.id)
        .await
        .expect("Failed to remove member");

    assert!(removed);

    let members = db.group_member_repo
        .find_by_group(fixtures.friend_group.id)
        .await
        .expect("Failed to find members");

    assert_eq!(members.len(), 2);
}

#[sqlx::test]
async fn test_group_member_find_role(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let role = db.group_member_repo
        .find_role(fixtures.friend_group.id, fixtures.user1.id)
        .await
        .expect("Failed to find role")
        .expect("Role should exist");

    assert_eq!(role, MemberRole::Admin);
}

#[sqlx::test]
async fn test_group_member_update_role(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let updated = db.group_member_repo
        .update_role(fixtures.friend_group.id, fixtures.user2.id, MemberRole::Admin)
        .await
        .expect("Failed to update role");

    assert_eq!(updated.role, "admin");
}

#[sqlx::test]
async fn test_group_member_count(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let count = db.group_member_repo
        .count_by_group(fixtures.friend_group.id)
        .await
        .expect("Failed to count members");

    assert_eq!(count, 3);
}

// ============================================================================
// Event Repository Tests
// ============================================================================

#[sqlx::test]
async fn test_event_create(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let group = create_test_group(&db, "group_pubkey", "Test Group", "admin").await;
    let outcomes = serde_json::json!(["Yes", "No", "Maybe"]);

    let event = db.event_repo
        .create(
            group.id,
            "Test Event",
            Some("Test description"),
            &outcomes,
            "manual",
            None,
        )
        .await
        .expect("Failed to create event");

    assert_eq!(event.title, "Test Event");
    assert_eq!(event.group_id, group.id);
    assert_eq!(event.status, "active");
}

#[sqlx::test]
async fn test_event_find_by_id(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let found_event = db.event_repo
        .find_by_id(fixtures.event.id)
        .await
        .expect("Failed to find event")
        .expect("Event should exist");

    assert_events_equal(&fixtures.event, &found_event);
}

#[sqlx::test]
async fn test_event_find_by_group(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    // Create another event
    let outcomes2 = serde_json::json!(["Win", "Lose"]);
    db.event_repo
        .create(
            fixtures.friend_group.id,
            "Event 2",
            None,
            &outcomes2,
            "oracle",
            None,
        )
        .await
        .expect("Failed to create event 2");

    let events = db.event_repo
        .find_by_group(fixtures.friend_group.id)
        .await
        .expect("Failed to find events");

    assert_eq!(events.len(), 2);
}

#[sqlx::test]
async fn test_event_update_status(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let updated = db.event_repo
        .update_status(fixtures.event.id, EventStatus::Resolved)
        .await
        .expect("Failed to update status");

    assert_eq!(updated.status, "resolved");
}

#[sqlx::test]
async fn test_event_update_solana_pubkey(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let updated = db.event_repo
        .update_solana_pubkey(fixtures.event.id, "new_solana_pubkey")
        .await
        .expect("Failed to update pubkey");

    assert_eq!(updated.solana_pubkey, Some("new_solana_pubkey".to_string()));
}

#[sqlx::test]
async fn test_event_find_active_events(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    // Resolve one event
    db.event_repo
        .update_status(fixtures.event.id, EventStatus::Resolved)
        .await
        .expect("Failed to update status");

    // Create another active event
    let outcomes2 = serde_json::json!(["A", "B"]);
    db.event_repo
        .create(
            fixtures.friend_group.id,
            "Active Event",
            None,
            &outcomes2,
            "manual",
            None,
        )
        .await
        .expect("Failed to create active event");

    let active_events = db.event_repo
        .find_active_events()
        .await
        .expect("Failed to find active events");

    assert_eq!(active_events.len(), 1);
    assert_eq!(active_events[0].title, "Active Event");
}

// ============================================================================
// Bet Repository Tests
// ============================================================================

#[sqlx::test]
async fn test_bet_create(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    let bet = create_test_bet(
        &db,
        fixtures.event.id,
        fixtures.user1.id,
        "Yes",
        Decimal::new(100, 0), // $100
        Decimal::new(50, 2),  // $0.50
    )
    .await;

    assert_eq!(bet.event_id, fixtures.event.id);
    assert_eq!(bet.user_id, fixtures.user1.id);
    assert_eq!(bet.outcome, "Yes");
    assert_eq!(bet.amount_usdc, Decimal::new(100, 0));
    assert_eq!(bet.price, Decimal::new(50, 2));
}

#[sqlx::test]
async fn test_bet_find_by_id(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;
    let created_bet = create_test_bet(
        &db,
        fixtures.event.id,
        fixtures.user1.id,
        "Yes",
        Decimal::new(100, 0),
        Decimal::new(50, 2),
    )
    .await;

    let found_bet = db.bet_repo
        .find_by_id(created_bet.id)
        .await
        .expect("Failed to find bet")
        .expect("Bet should exist");

    assert_bets_equal(&created_bet, &found_bet);
}

#[sqlx::test]
async fn test_bet_find_by_event(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

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

    let bets = db.bet_repo
        .find_by_event(fixtures.event.id)
        .await
        .expect("Failed to find bets");

    assert_eq!(bets.len(), 2);
}

#[sqlx::test]
async fn test_bet_find_by_user(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

    create_test_bet(
        &db,
        fixtures.event.id,
        fixtures.user1.id,
        "Yes",
        Decimal::new(100, 0),
        Decimal::new(50, 2),
    )
    .await;

    // Create another event and bet
    let event2 = create_test_event(&db, fixtures.friend_group.id, "Event 2", vec!["A", "B"]).await;
    create_test_bet(
        &db,
        event2.id,
        fixtures.user1.id,
        "A",
        Decimal::new(50, 0),
        Decimal::new(40, 2),
    )
    .await;

    let bets = db.bet_repo
        .find_by_user(fixtures.user1.id)
        .await
        .expect("Failed to find bets");

    assert_eq!(bets.len(), 2);
}

#[sqlx::test]
async fn test_bet_get_total_volume(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

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

    let total_volume = db.bet_repo
        .get_total_volume_for_event(fixtures.event.id)
        .await
        .expect("Failed to get total volume")
        .expect("Total volume should exist");

    assert_eq!(total_volume, Decimal::new(300, 0));
}

#[sqlx::test]
async fn test_bet_get_volume_by_outcome(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

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

    let volumes = db.bet_repo
        .get_volume_by_outcome(fixtures.event.id)
        .await
        .expect("Failed to get volumes by outcome");

    assert_eq!(volumes.len(), 2);
    assert!(volumes.iter().any(|(outcome, _)| outcome == "Yes"));
    assert!(volumes.iter().any(|(outcome, _)| outcome == "No"));
}

#[sqlx::test]
async fn test_bet_count_by_event(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let fixtures = TestFixtures::create(&db).await;

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

    let count = db.bet_repo
        .count_by_event(fixtures.event.id)
        .await
        .expect("Failed to count bets");

    assert_eq!(count, 2);
}

// ============================================================================
// Transaction Tests
// ============================================================================

#[sqlx::test]
async fn test_transaction_rollback(pool: PgPool) {
    let db = TestDatabase::from_pool(pool.clone()).await;
    db.cleanup().await;

    let mut tx = pool.begin().await.expect("Failed to begin transaction");

    // Create a user in transaction
    let user = db.user_repo
        .create("transaction_wallet")
        .await
        .expect("Failed to create user");

    // Rollback transaction
    tx.rollback().await.expect("Failed to rollback");

    // User should not exist after rollback
    let found_user = db.user_repo
        .find_by_id(user.id)
        .await
        .expect("Failed to query");

    assert!(found_user.is_none());
}

#[sqlx::test]
async fn test_transaction_commit(pool: PgPool) {
    let db = TestDatabase::from_pool(pool.clone()).await;
    db.cleanup().await;

    let mut tx = pool.begin().await.expect("Failed to begin transaction");

    // Create a user in transaction
    let user = db.user_repo
        .create("commit_wallet")
        .await
        .expect("Failed to create user");

    // Commit transaction
    tx.commit().await.expect("Failed to commit");

    // User should exist after commit
    let found_user = db.user_repo
        .find_by_id(user.id)
        .await
        .expect("Failed to query")
        .expect("User should exist");

    assert_eq!(found_user.wallet_address, "commit_wallet");
}

// ============================================================================
// Error Case Tests
// ============================================================================

#[sqlx::test]
async fn test_user_not_found(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let non_existent_id = Uuid::new_v4();
    let user = db.user_repo
        .find_by_id(non_existent_id)
        .await
        .expect("Query should succeed");

    assert!(user.is_none());
}

#[sqlx::test]
async fn test_group_not_found(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let non_existent_id = Uuid::new_v4();
    let group = db.friend_group_repo
        .find_by_id(non_existent_id)
        .await
        .expect("Query should succeed");

    assert!(group.is_none());
}

#[sqlx::test]
#[should_panic(expected = "duplicate key value violates unique constraint")]
async fn test_unique_constraint_violation(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    // Create user with wallet
    db.user_repo
        .create("duplicate_wallet")
        .await
        .expect("Failed to create user");

    // Try to create another user with same wallet (should fail)
    db.user_repo
        .create("duplicate_wallet")
        .await
        .expect_err("Should fail due to unique constraint");
}

#[sqlx::test]
#[should_panic(expected = "violates foreign key constraint")]
async fn test_foreign_key_constraint(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;

    let non_existent_group_id = Uuid::new_v4();
    let outcomes = serde_json::json!(["Yes", "No"]);

    // Try to create event with non-existent group (should fail)
    db.event_repo
        .create(
            non_existent_group_id,
            "Test Event",
            None,
            &outcomes,
            "manual",
            None,
        )
        .await
        .expect_err("Should fail due to foreign key constraint");
}

