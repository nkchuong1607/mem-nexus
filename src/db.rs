use rusqlite::{Connection, Result};

pub fn init_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS wings (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            type TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS rooms (
            id INTEGER PRIMARY KEY,
            wing_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            FOREIGN KEY(wing_id) REFERENCES wings(id),
            UNIQUE(wing_id, name)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY,
            room_id INTEGER NOT NULL,
            content TEXT NOT NULL,
            embedding BLOB NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(room_id) REFERENCES rooms(id)
        )",
        [],
    )?;

    Ok(conn)
}
