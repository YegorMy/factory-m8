#[cfg(feature = "derive")]
pub use factory_m8_derive::Factory;

//! Factory Core - Shared traits for test data factories
//!
//! This crate provides traits that factories implement to enable
//! database insertion with automatic FK resolution.
//!
//! ## Traits
//!
//! - [`FactoryCreate`] - Async trait for creating entities in the database
//! - [`Sentinel`] - Trait for detecting "unset" values that trigger auto-creation
//!
//! ## Database Agnostic
//!
//! `FactoryCreate` is generic over the connection/pool type, so it works with any database:
//!
//! - `sqlx::PgPool` (PostgreSQL)
//! - `sqlx::SqlitePool` (SQLite)
//! - `sqlx::MySqlPool` (MySQL)
//! - `mongodb::Database` (MongoDB)
//! - Any custom connection type
//!
//! ## Example
//!
//! ```ignore
//! use factory_core::{FactoryCreate, FactoryResult, Sentinel};
//! use sqlx::PgPool;
//!
//! // Implement Sentinel for your ID types
//! #[derive(Clone, Default, PartialEq)]
//! pub struct UserId(pub i64);
//!
//! impl Sentinel for UserId {
//!     fn sentinel() -> Self { UserId(0) }
//!     fn is_sentinel(&self) -> bool { self.0 == 0 }
//! }
//!
//! // Use in factory
//! #[derive(Factory)]
//! #[factory(entity = User)]
//! pub struct UserFactory {
//!     #[pk]
//!     pub id: UserId,
//!
//!     #[fk(Tenant, "id", TenantFactory)]
//!     pub tenant_id: TenantId,  // Auto-creates if sentinel
//! }
//!
//! // Implement for your chosen database
//! #[async_trait]
//! impl FactoryCreate<PgPool> for UserFactory {
//!     type Entity = User;
//!
//!     async fn create(self, pool: &PgPool) -> FactoryResult<User> {
//!         // ... INSERT query
//!     }
//! }
//! ```
//!
//! ## Mixed Database Backends
//!
//! If your project uses multiple databases (e.g., Postgres + MongoDB), you can:
//!
//! 1. Implement `FactoryCreate<PgPool>` for Postgres entities
//! 2. Implement `FactoryCreate<mongodb::Database>` for MongoDB entities
//! 3. Use `no_default` on cross-backend FKs to disable auto-creation:
//!
//! ```ignore
//! #[derive(Factory)]
//! #[factory(entity = Patient)]
//! pub struct PatientFactory {
//!     // Same backend - auto-creates work
//!     #[fk(Practice, "id", PracticeFactory)]
//!     pub practice_id: PracticeId,
//!
//!     // Different backend - set manually, no auto-create
//!     #[fk(AuditLog, "id", AuditLogFactory, no_default)]
//!     pub audit_log_id: Option<AuditLogId>,
//! }
//! ```

use async_trait::async_trait;
use std::error::Error;

// =============================================================================
// RESULT TYPE
// =============================================================================

/// Result type for factory create operations.
pub type FactoryResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

// =============================================================================
// SENTINEL TRAIT
// =============================================================================

/// Trait for detecting "sentinel" values that trigger auto-creation of FK dependencies.
///
/// When a factory field has a sentinel value, `build_with_fks()` will automatically
/// create the dependency using its factory instead of using the sentinel value.
///
/// ## Common Sentinel Values
///
/// - For numeric IDs: `0` (database IDs typically start at 1)
/// - For UUIDs: `Uuid::nil()`
/// - For `Option<T>`: `None`
///
/// ## Example
///
/// ```
/// use factory_core::Sentinel;
///
/// #[derive(Clone, Copy, Default, PartialEq)]
/// pub struct UserId(pub i64);
///
/// impl Sentinel for UserId {
///     fn sentinel() -> Self {
///         UserId(0)
///     }
///
///     fn is_sentinel(&self) -> bool {
///         self.0 == 0
///     }
/// }
/// ```
pub trait Sentinel: Clone {
    /// Returns the sentinel value for this type.
    ///
    /// This is the value that indicates "not set, please auto-generate".
    fn sentinel() -> Self;

    /// Returns true if this value is the sentinel (i.e., should trigger auto-creation).
    fn is_sentinel(&self) -> bool;
}

// =============================================================================
// SENTINEL IMPLEMENTATIONS FOR COMMON TYPES
// =============================================================================

impl Sentinel for i64 {
    fn sentinel() -> Self { 0 }
    fn is_sentinel(&self) -> bool { *self == 0 }
}

impl Sentinel for i32 {
    fn sentinel() -> Self { 0 }
    fn is_sentinel(&self) -> bool { *self == 0 }
}

impl Sentinel for i16 {
    fn sentinel() -> Self { 0 }
    fn is_sentinel(&self) -> bool { *self == 0 }
}

impl Sentinel for u64 {
    fn sentinel() -> Self { 0 }
    fn is_sentinel(&self) -> bool { *self == 0 }
}

impl Sentinel for u32 {
    fn sentinel() -> Self { 0 }
    fn is_sentinel(&self) -> bool { *self == 0 }
}

impl Sentinel for String {
    fn sentinel() -> Self { String::new() }
    fn is_sentinel(&self) -> bool { self.is_empty() }
}

/// Blanket implementation for `Option<T>`.
///
/// - `None` is always a sentinel
/// - `Some(value)` is a sentinel if `value.is_sentinel()` returns true
impl<T: Sentinel> Sentinel for Option<T> {
    fn sentinel() -> Self {
        None
    }

    fn is_sentinel(&self) -> bool {
        match self {
            None => true,
            Some(v) => v.is_sentinel(),
        }
    }
}

// =============================================================================
// FACTORY CREATE TRAIT
// =============================================================================

/// Trait for factories that can create entities in the database.
///
/// The `Pool` type parameter allows factories to work with any database backend:
///
/// - `sqlx::PgPool` (PostgreSQL)
/// - `sqlx::SqlitePool` (SQLite)
/// - `sqlx::MySqlPool` (MySQL)
/// - `mongodb::Database` (MongoDB)
/// - Any custom connection type
///
/// ## Example
///
/// ```ignore
/// use factory_core::{FactoryCreate, FactoryResult};
/// use sqlx::PgPool;
///
/// #[async_trait]
/// impl FactoryCreate<PgPool> for PatientFactory {
///     type Entity = Patient;
///
///     async fn create(self, pool: &PgPool) -> FactoryResult<Patient> {
///         // build_with_fks resolves all FK dependencies automatically
///         let entity = self.build_with_fks(pool).await?;
///
///         // User writes the INSERT query
///         let patient = sqlx::query_as!(Patient,
///             "INSERT INTO patient (practice_id, name) VALUES ($1, $2) RETURNING *",
///             entity.practice_id.0,
///             entity.name,
///         )
///         .fetch_one(pool)
///         .await?;
///
///         Ok(patient)
///     }
/// }
/// ```
#[async_trait]
pub trait FactoryCreate<Pool>: Sized
where
    Pool: Sync,
{
    /// The entity type this factory creates.
    type Entity;

    /// Create the entity in the database.
    ///
    /// Implementations should:
    /// 1. Call `self.build_with_fks(pool).await?` to resolve FK dependencies
    /// 2. Execute an INSERT query with the resolved entity fields
    /// 3. Return the created entity (usually with RETURNING *)
    async fn create(self, pool: &Pool) -> FactoryResult<Self::Entity>;
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Default, PartialEq, Debug)]
    struct TestId(i64);

    impl Sentinel for TestId {
        fn sentinel() -> Self { TestId(0) }
        fn is_sentinel(&self) -> bool { self.0 == 0 }
    }

    #[test]
    fn test_sentinel_i64() {
        assert!(0_i64.is_sentinel());
        assert!(!1_i64.is_sentinel());
        assert!(!(-1_i64).is_sentinel());
    }

    #[test]
    fn test_sentinel_option_none() {
        let none: Option<i64> = None;
        assert!(none.is_sentinel());
    }

    #[test]
    fn test_sentinel_option_some_sentinel() {
        let some_zero: Option<i64> = Some(0);
        assert!(some_zero.is_sentinel());
    }

    #[test]
    fn test_sentinel_option_some_non_sentinel() {
        let some_one: Option<i64> = Some(1);
        assert!(!some_one.is_sentinel());
    }

    #[test]
    fn test_sentinel_custom_type() {
        assert!(TestId(0).is_sentinel());
        assert!(!TestId(1).is_sentinel());
        assert!(!TestId(999).is_sentinel());
    }

    #[test]
    fn test_sentinel_option_custom_type() {
        let none: Option<TestId> = None;
        let some_zero: Option<TestId> = Some(TestId(0));
        let some_one: Option<TestId> = Some(TestId(1));

        assert!(none.is_sentinel());
        assert!(some_zero.is_sentinel());
        assert!(!some_one.is_sentinel());
    }
}
