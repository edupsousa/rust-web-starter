use chrono::Utc;
use color_eyre::eyre::Result;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool, Pool, FromRow};
use tracing::info;
use uuid::Uuid;

const DB_URL: &str = "sqlite://chat.db";
const SQL_CREATE_MESSAGES_TABLE: &str = "CREATE TABLE IF NOT EXISTS messages (id TEXT PRIMARY KEY NOT NULL, create_time NUMERIC NOT NULL, text TEXT NOT NULL);";

#[derive(Debug, Clone, FromRow)]
pub struct Message {
    id: String,
    pub text: String,
    create_time: i64,
}

#[derive(Clone)]
pub struct ChatDB {
    pool: Pool<Sqlite>,
}

impl ChatDB {
    pub async fn build() -> Result<Self> {
        let mut is_new = false;
        if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
            info!("Creating database {}", DB_URL);
            Sqlite::create_database(DB_URL).await?;
            is_new = true;
        }
        info!("Connecting to database {}", DB_URL);
        let pool = SqlitePool::connect(DB_URL).await?;
        if is_new {
            sqlx::query(SQL_CREATE_MESSAGES_TABLE).execute(&pool).await?;
        }
        return Ok(ChatDB { pool });
    }

    pub async fn push_message(&self, text: &str) -> Result<()> {
        let id: String = Uuid::new_v4().into();
        let create_time = Utc::now().timestamp(); 
        let _result = sqlx::query("INSERT INTO messages (id, create_time, text) VALUES (?, ?, ?);")
            .bind(id)
            .bind(create_time)
            .bind(text)
            .execute(&self.pool)
            .await?;
        return Ok(());
    }

    pub async fn list_all_messages(&self) -> Result<Vec<Message>> {
        let messages_result = sqlx::query_as::<_, Message>("SELECT * FROM messages ORDER BY create_time;")
            .fetch_all(&self.pool)
            .await?;
        return Ok(messages_result);
    }
}

#[cfg(test)]
mod tests {
    use crate::db::ChatDB;

    #[tokio::test]
    async fn build_instance() {
        let db = ChatDB::build().await;
        assert_eq!(db.is_ok(), true);
        let db = db.unwrap();
        let messages = db.list_all_messages().await.unwrap();
        assert_eq!(messages.len(), 0);
        db.push_message("A new message").await.unwrap();
        let messages = db.list_all_messages().await.unwrap();
        assert_eq!(messages.len(), 1);
        println!("{:?}", messages);
    }
}