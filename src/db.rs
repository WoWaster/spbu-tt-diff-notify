use log::{debug, info};
use rusqlite::{named_params, Connection};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

pub fn init_connection(db_path: PathBuf) -> Result<Connection, rusqlite::Error> {
    info!(
        "Connecting to schedule.sqlite3 in {}",
        std::path::absolute(&db_path).unwrap().display()
    );
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS educator (
                id INTEGER PRIMARY KEY,
                events JSON NOT NULL
            )",
        (),
    )?;

    Ok(conn)
}

pub fn get_educators_from_db(conn: &Connection) -> Result<HashMap<u32, String>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached("SELECT id, events FROM educator")?;
    let educator_info = stmt.query_map([], |row| {
        let id: u32 = row.get(0)?;
        let events: String = row.get(1)?;
        Ok((id, events))
    })?;
    educator_info.into_iter().collect()
}

pub fn get_educators_ids_from_db(conn: &Connection) -> Result<HashSet<u32>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached("SELECT id FROM educator")?;
    let ids = stmt.query_map([], |row| row.get(0))?;
    ids.into_iter().collect()
}

pub fn add_new_educator_to_db(
    conn: &Connection,
    id: u32,
    educator_events: &str,
) -> Result<(), rusqlite::Error> {
    debug!("Adding educator {} to db", id);

    let mut stmt =
        conn.prepare_cached("INSERT INTO educator (id, events) VALUES (:id, :events)")?;
    stmt.execute(named_params! {":id": id, ":events": educator_events})?;

    Ok(())
}

pub fn remove_educator_from_db(conn: &Connection, id: u32) -> Result<(), rusqlite::Error> {
    debug!("Remove educator {} from db", id);

    let mut stmt = conn.prepare_cached("DELETE FROM educator WHERE id = :id")?;
    stmt.execute((id,))?;

    Ok(())
}

pub fn update_educator_events_in_db(
    conn: &Connection,
    id: u32,
    educator_events: &str,
) -> Result<(), rusqlite::Error> {
    debug!("Updating educator {} events in db", id);

    let mut stmt = conn.prepare_cached("UPDATE educator SET events = :events WHERE id = :id")?;
    stmt.execute(named_params! {":id": id, ":events": educator_events})?;

    Ok(())
}
