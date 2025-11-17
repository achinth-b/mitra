use mitra_backend::config::DatabaseConfig;
use mitra_backend::database::{create_pool, run_migrations};
use mitra_backend::models::*;
use mitra_backend::repositories::*;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Test database configuration
pub struct TestDatabase {
    pub pool: PgPool,
    pub friend_group_repo: Arc<FriendGroupRepository>,
    pub user_repo: Arc<UserRepository>,
    pub group_member_repo: Arc<GroupMemberRepository>,
    pub event_repo: Arc<EventRepository>,
    pub bet_repo: Arc<BetRepository>,
}

impl TestDatabase {
    /// Create a new test database connection (creates its own pool)
    pub async fn new() -> Self {
        // Use test database URL from environment or default
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/mitra_test".to_string());

        let config = DatabaseConfig {
            url: database_url,
            max_connections: 5,
            acquire_timeout_secs: 10,
            idle_timeout_secs: 300,
            max_lifetime_secs: 600,
            test_before_acquire: true,
        };

        let pool = create_pool(&config)
            .await
            .expect("Failed to create test database pool");

        // Run migrations
        run_migrations(&pool, None)
            .await
            .expect("Failed to run migrations");

        Self::from_pool(pool).await
    }

    /// Create TestDatabase from an existing pool (useful with sqlx::test)
    pub async fn from_pool(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            friend_group_repo: Arc::new(FriendGroupRepository::new(pool.clone())),
            user_repo: Arc::new(UserRepository::new(pool.clone())),
            group_member_repo: Arc::new(GroupMemberRepository::new(pool.clone())),
            event_repo: Arc::new(EventRepository::new(pool.clone())),
            bet_repo: Arc::new(BetRepository::new(pool)),
        }
    }

    /// Clean up all test data
    pub async fn cleanup(&self) {
        sqlx::query("TRUNCATE TABLE bets, events, group_members, friend_groups, users RESTART IDENTITY CASCADE")
            .execute(&self.pool)
            .await
            .expect("Failed to cleanup test data");
    }
}

/// Test data fixtures
pub struct TestFixtures {
    pub user1: User,
    pub user2: User,
    pub user3: User,
    pub friend_group: FriendGroup,
    pub event: Event,
}

impl TestFixtures {
    /// Create test fixtures with sample data
    pub async fn create(db: &TestDatabase) -> Self {
        // Create users
        let user1 = db.user_repo
            .create("test_wallet_1")
            .await
            .expect("Failed to create user1");

        let user2 = db.user_repo
            .create("test_wallet_2")
            .await
            .expect("Failed to create user2");

        let user3 = db.user_repo
            .create("test_wallet_3")
            .await
            .expect("Failed to create user3");

        // Create friend group
        let friend_group = db.friend_group_repo
            .create(
                "test_solana_pubkey_123",
                "Test Group",
                "test_wallet_1",
            )
            .await
            .expect("Failed to create friend group");

        // Add members
        db.group_member_repo
            .add_member(friend_group.id, user1.id, MemberRole::Admin)
            .await
            .expect("Failed to add user1 as admin");

        db.group_member_repo
            .add_member(friend_group.id, user2.id, MemberRole::Member)
            .await
            .expect("Failed to add user2 as member");

        db.group_member_repo
            .add_member(friend_group.id, user3.id, MemberRole::Member)
            .await
            .expect("Failed to add user3 as member");

        // Create event
        let outcomes = serde_json::json!(["Yes", "No"]);
        let event = db.event_repo
            .create(
                friend_group.id,
                "Test Event",
                Some("Test event description"),
                &outcomes,
                "manual",
                None,
            )
            .await
            .expect("Failed to create event");

        Self {
            user1,
            user2,
            user3,
            friend_group,
            event,
        }
    }
}

/// Helper function to create a test user
pub async fn create_test_user(db: &TestDatabase, wallet: &str) -> User {
    db.user_repo
        .create(wallet)
        .await
        .expect("Failed to create test user")
}

/// Helper function to create a test friend group
pub async fn create_test_group(
    db: &TestDatabase,
    solana_pubkey: &str,
    name: &str,
    admin_wallet: &str,
) -> FriendGroup {
    db.friend_group_repo
        .create(solana_pubkey, name, admin_wallet)
        .await
        .expect("Failed to create test group")
}

/// Helper function to create a test event
pub async fn create_test_event(
    db: &TestDatabase,
    group_id: Uuid,
    title: &str,
    outcomes: Vec<&str>,
) -> Event {
    let outcomes_json = serde_json::to_value(outcomes).unwrap();
    db.event_repo
        .create(
            group_id,
            title,
            None,
            &outcomes_json,
            "manual",
            None,
        )
        .await
        .expect("Failed to create test event")
}

/// Helper function to create a test bet
pub async fn create_test_bet(
    db: &TestDatabase,
    event_id: Uuid,
    user_id: Uuid,
    outcome: &str,
    amount: Decimal,
    price: Decimal,
) -> Bet {
    let shares = amount / price;
    db.bet_repo
        .create(event_id, user_id, outcome, shares, price, amount)
        .await
        .expect("Failed to create test bet")
}

/// Assert that two users are equal (ignoring timestamps)
pub fn assert_users_equal(user1: &User, user2: &User) {
    assert_eq!(user1.id, user2.id);
    assert_eq!(user1.wallet_address, user2.wallet_address);
}

/// Assert that two friend groups are equal (ignoring timestamps)
pub fn assert_groups_equal(group1: &FriendGroup, group2: &FriendGroup) {
    assert_eq!(group1.id, group2.id);
    assert_eq!(group1.solana_pubkey, group2.solana_pubkey);
    assert_eq!(group1.name, group2.name);
    assert_eq!(group1.admin_wallet, group2.admin_wallet);
}

/// Assert that two events are equal (ignoring timestamps)
pub fn assert_events_equal(event1: &Event, event2: &Event) {
    assert_eq!(event1.id, event2.id);
    assert_eq!(event1.group_id, event2.group_id);
    assert_eq!(event1.title, event2.title);
    assert_eq!(event1.status, event2.status);
}

/// Assert that two bets are equal (ignoring timestamps)
pub fn assert_bets_equal(bet1: &Bet, bet2: &Bet) {
    assert_eq!(bet1.id, bet2.id);
    assert_eq!(bet1.event_id, bet2.event_id);
    assert_eq!(bet1.user_id, bet2.user_id);
    assert_eq!(bet1.outcome, bet2.outcome);
    assert_eq!(bet1.shares, bet2.shares);
    assert_eq!(bet1.price, bet2.price);
    assert_eq!(bet1.amount_usdc, bet2.amount_usdc);
}

/// Helper to run a test with a clean database
pub async fn with_test_db<F, Fut>(test: F)
where
    F: FnOnce(TestDatabase) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let db = TestDatabase::new().await;
    db.cleanup().await;
    test(db).await;
    db.cleanup().await;
}

