# factory-m8

Core traits for test data factories with automatic FK resolution.

This crate provides the foundational traits used by `factory-m8-derive` to generate test factories that automatically create database dependencies.

## Installation

```toml
[dev-dependencies]
factory-m8 = "0.1"
factory-m8-derive = "0.1"
```

## Traits

### `FactoryCreate<Pool>`

The main trait for factories that create entities in a database. Generic over the connection pool type, so it works with any database backend.

```rust
use factory_m8::{FactoryCreate, FactoryResult};
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
impl FactoryCreate<PgPool> for UserFactory {
    type Entity = User;

    async fn create(self, pool: &PgPool) -> FactoryResult<User> {
        let entity = self.build_with_fks(pool).await?;

        sqlx::query_as!(User,
            "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *",
            entity.name,
            entity.email,
        )
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
```

### `Sentinel`

Trait for detecting "sentinel" values that trigger auto-creation of FK dependencies.

```rust
use factory_m8::Sentinel;

#[derive(Clone, Copy, Default)]
pub struct UserId(pub i64);

impl Sentinel for UserId {
    fn sentinel() -> Self {
        UserId(0)  // 0 means "not set"
    }

    fn is_sentinel(&self) -> bool {
        self.0 == 0
    }
}
```

Built-in implementations for: `i16`, `i32`, `i64`, `u32`, `u64`, `String`, and `Option<T>`.

## Database Backends

`FactoryCreate` is generic over the pool type, supporting any database:

| Backend | Pool Type |
|---------|-----------|
| PostgreSQL | `sqlx::PgPool` |
| SQLite | `sqlx::SqlitePool` |
| MySQL | `sqlx::MySqlPool` |
| MongoDB | `mongodb::Database` |

## License

MIT License - see [LICENSE](LICENSE) for details.
