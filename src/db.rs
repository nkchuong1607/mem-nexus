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

    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
            content,
            content='memories',
            content_rowid='id'
        )",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
            INSERT INTO memories_fts(rowid, content) VALUES (new.id, new.content);
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.id, old.content);
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.id, old.content);
            INSERT INTO memories_fts(rowid, content) VALUES (new.id, new.content);
        END;",
        [],
    )?;

    Ok(conn)
}
