// load the config struct the config module
use crate::config::Config;
// load these struts from the models module
use crate::models::{CreatePromptResponse, Prompt};
// load error handling and result types
use anyhow::{Context, Result};
// date and time handling
use chrono::{Utc, NaiveDateTime};
// load mysql pools and database row modules
use sqlx::{mysql::MySqlPool, Row};

pub async fn create_database_pool(config: &Config) -> Result<MySqlPool> {
    // create a connection pool to the MySQL database using the URL from the config
    let pool = MySqlPool::connect(&config.database_url)
        .await
        .context("Failed to connect to MySQL database")?;

    // check if a test query runs on the db
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .context("Database health check failed")?;

    println!("✅ Successfully connected to MySQL database");
    Ok(pool)
}

pub async fn init_database(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS prompts (
            id INTEGER PRIMARY KEY AUTO_INCREMENT,
            prompt TEXT NOT NULL,
            response MEDIUMTEXT,
            created_at DATETIME NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_prompt_record(
    pool: &MySqlPool,
    prompt: String,
    response: Option<&String>, // save the response or null if not provided
) -> Result<CreatePromptResponse, sqlx::Error> {
    let insert_result = sqlx::query(
        "INSERT INTO prompts (prompt, response, created_at) VALUES (?, ?, ?)"
    )
        .bind(&prompt)
        .bind(&response)
        .bind(Utc::now().naive_utc())
        .execute(pool)
        .await?;

    let id = insert_result.last_insert_id() as i64;

    let row = sqlx::query(
        "SELECT id, prompt, response, created_at FROM prompts WHERE id = ?"
    )
        .bind(id)
        .fetch_one(pool)
        .await?;

    let naive_datetime: NaiveDateTime = row.get(3);

    Ok(CreatePromptResponse {
        id: row.get("id"),
        prompt: row.get("prompt"),
        response: row.get("response"),
        created_at: naive_datetime.and_utc(),
    })
}


pub async fn get_all_prompts(pool: &MySqlPool) -> Result<Vec<Prompt>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, prompt, created_at FROM prompts ORDER BY created_at DESC"
    )
        .fetch_all(pool)
        .await?;

    let prompts = rows.into_iter().map(|row| {
        let naive_datetime: NaiveDateTime = row.get("created_at");
        Prompt {
            id: row.get("id"),
            prompt: row.get("prompt"),
            created_at: naive_datetime.and_utc(),
        }
    }).collect();

    Ok(prompts)
}