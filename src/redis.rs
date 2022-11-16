/* This module defines ergonomic ways to create and use a Redis connection pool.
The mobc crate is used to create an async pool. This was chosen for two reasons,
1) familiar paralellism with the mobc-postgres crate I have already used extensively,
2) This page reports successful deployment in production using mobc:
    https://blog.logrocket.com/using-redis-in-a-rust-web-service/ */

use std::{env, time::Duration};
use serde::de::DeserializeOwned;
use serde_json;
//use async_trait::async_trait;
//pub use redis::{RedisResult, Client, aio::Connection}; // note aio::Connection vs Connection for async
use mobc::Pool;
use mobc_redis::{RedisConnectionManager, redis::{AsyncCommands, RedisResult, Client, aio::Connection}};
use crate::core::GenericError;

// constants for mobc redis connection pools
// see https://blog.logrocket.com/using-redis-in-a-rust-web-service/
const CACHE_POOL_MAX_OPEN: u64 = 16;
const CACHE_POOL_MAX_IDLE: u64 = 8;
const CACHE_POOL_TIMEOUT_SECONDS: u64 = 1;
const CACHE_POOL_EXPIRE_SECONDS: u64 = 60;
const OBSCURE_TEST_KEY: &'static str = "obscure_test_key";

pub type MobcPool = Pool<RedisConnectionManager>;
pub type MobcCon = Connection<RedisConnectionManager>;


pub async fn new_pool_from_client(client: Client) -> Result<MobcPool, GenericError> {
    let manager = RedisConnectionManager::new(client);
    let pool = Pool::builder()
        .get_timeout(Some(Duration::from_secs(CACHE_POOL_TIMEOUT_SECONDS)))
        .max_open(CACHE_POOL_MAX_OPEN)
        .max_idle(CACHE_POOL_MAX_IDLE)
        .max_lifetime(Some(Duration::from_secs(CACHE_POOL_EXPIRE_SECONDS)))
        .build(manager);
    // try to connect now so you fail early
    let mut conn = pool.get().await?;
    let _x: Option<String> = conn.get(OBSCURE_TEST_KEY).await?;
    Ok(pool)
}

pub async fn new_pool_from_env() -> Result<MobcPool, GenericError> {
    let client = new_client_from_env()?;
    new_pool_from_client(client).await
}


/// Generate a new client based on a uri scheme, a host, and a password
pub fn new_client(uri_scheme: &str, redis_host: &str, redis_pw: &str) -> RedisResult<Client> {
    let redis_conn_url = format!("{}://:{}@{}", uri_scheme, redis_pw, redis_host);
    Client::open(redis_conn_url)
}

/// Generate a new client from environment variables
pub fn new_client_from_env() -> RedisResult<Client>  {
    let uri_scheme = match env::var("IS_TLS") {
        Ok(_) => "rediss",
        Err(_) => "redis",
    };

    let redis_host: String = match env::var("REDIS_HOST") {
        Ok(val) => val,
        Err(_) => {
            match env::var("REDIS_PORT")  {
                Ok(port) => format!("127.0.0.1:{}", port),
                Err(_) => "127.0.0.1:6379".to_string(),
            }
        },
    };
    let redis_pw: String = match env::var("REDIS_PW") {
        Ok(val) => val,
        Err(_) => "".to_string(),
    };
    new_client(&uri_scheme, &redis_host, &redis_pw)
}


/// For a struct that can be deserialized,
/// This helpful method gets a connection, gets the value stored at the key,
/// deserializes it, and returns the desired struct
pub async fn from_redis_str<T: DeserializeOwned>(pool: &MobcPool, key: &str) -> Result<T, GenericError> {
    let mut conn = pool.get().await?;
    let jz: String = conn.get(key).await?;
    let t: T = serde_json::from_str(&jz)?;
    Ok(t)
}





