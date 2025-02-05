use log::{debug, info};
use sqlx::SqlitePool;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    path::PathBuf,
};

const MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

pub async fn init_connection(db_path: PathBuf) -> Result<SqlitePool, Box<dyn Error>> {
    info!(
        "Connecting to schedule.sqlite3 in {}",
        std::path::absolute(&db_path).unwrap().display()
    );
    let conn_str = format!("sqlite:{}", db_path.to_str().unwrap());
    let pool = SqlitePool::connect(&conn_str).await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}

pub async fn get_educators_from_db(
    pool: &SqlitePool,
) -> Result<HashMap<i64, String>, Box<dyn Error>> {
    Ok(sqlx::query!("SELECT id, events FROM educator")
        .map(|row| (row.id, row.events))
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect::<HashMap<i64, String>>())
}

pub async fn get_educators_ids_from_db(pool: &SqlitePool) -> Result<HashSet<i64>, Box<dyn Error>> {
    Ok(sqlx::query!("SELECT id FROM educator")
        .map(|row| row.id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect::<HashSet<i64>>())
}

pub async fn add_new_educator_to_db(
    pool: &SqlitePool,
    id: i64,
    educator_events: &str,
) -> Result<(), Box<dyn Error>> {
    debug!("Adding educator {} to db", id);

    sqlx::query!(
        "INSERT INTO educator (id, events) VALUES ($1, $2)",
        id,
        educator_events
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn remove_educator_from_db(pool: &SqlitePool, id: i64) -> Result<(), Box<dyn Error>> {
    debug!("Remove educator {} from db", id);

    sqlx::query!("DELETE FROM educator WHERE id = $1", id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_educator_events_in_db(
    pool: &SqlitePool,
    id: i64,
    educator_events: &str,
) -> Result<(), Box<dyn Error>> {
    debug!("Updating educator {} events in db", id);

    sqlx::query!(
        "UPDATE educator SET events = $2 WHERE id = $1",
        id,
        educator_events
    )
    .execute(pool)
    .await?;

    Ok(())
}
