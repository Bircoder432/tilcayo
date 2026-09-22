use std::time::Duration;

use redis::AsyncCommands;
use tilcayo_core::UserId;

pub struct Session {
    pub id: String,
    pub user_id: UserId,
    pub refresh_token_hash: String,
}

pub struct SessionStore {
    client: redis::Client,
}

impl SessionStore {
    pub fn new(url: &str) -> redis::RedisResult<Self> {
        Ok(Self {
            client: redis::Client::open(url)?,
        })
    }

    pub async fn create(
        &self,
        id: &str,
        user_id: &UserId,
        refresh_token_hash: &str,
        ttl: Duration,
    ) -> redis::RedisResult<()> {
        let mut connection = self.client.get_multiplexed_async_connection().await?;

        let key = format!("session:{id}");

        let _: () = connection
            .hset_multiple(
                &key,
                &[
                    ("user_id", user_id.0.to_string()),
                    ("refresh_token_hash", refresh_token_hash.to_owned()),
                ],
            )
            .await?;

        let _: () = connection.expire(&key, ttl.as_secs() as i64).await?;

        Ok(())
    }

    pub async fn find(&self, id: &str) -> redis::RedisResult<Option<Session>> {
        let mut connection = self.client.get_multiplexed_async_connection().await?;

        let key = format!("session:{id}");

        let exists: bool = connection.exists(&key).await?;

        if !exists {
            return Ok(None);
        }

        let values: Vec<String> = connection
            .hmget(&key, &["user_id", "refresh_token_hash"])
            .await?;

        let user_id = match values[0].parse::<usize>() {
            Ok(user_id) => user_id,
            Err(_) => return Ok(None),
        };

        Ok(Some(Session {
            id: id.to_owned(),
            user_id: UserId(user_id),
            refresh_token_hash: values[1].clone(),
        }))
    }

    pub async fn delete(&self, id: &str) -> redis::RedisResult<()> {
        let mut connection = self.client.get_multiplexed_async_connection().await?;

        let key = format!("session:{id}");

        let _: () = connection.del(key).await?;

        Ok(())
    }
}
