use chrono::Utc;
use color_eyre::eyre::Result;
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool, Pool, FromRow};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Message {
    pub id: String,
    pub text: String,
    pub create_time: i64,
}

#[derive(Clone)]
pub struct ChatDB {
    pub pool: Pool<Sqlite>,
}

impl ChatDB {
    pub async fn build(db_url: &str) -> Result<Self> {
        if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
            info!("Creating database {}", db_url);
            Sqlite::create_database(db_url).await?;
        }
        info!("Connecting to database {}", db_url);
        let pool = SqlitePool::connect(db_url).await?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;
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
    use crate::{db::ChatDB, config::Config};

    #[tokio::test]
    async fn build_instance() {
        let config = Config::init();
        let db = ChatDB::build(&config.database_url).await;
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