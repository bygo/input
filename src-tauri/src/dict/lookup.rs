use super::db::with_db;
use super::entry::{is_wordlike, lemma_candidates, normalize, row_to_entry};
use super::path::active_pack;
use super::types::DictEntry;
use rusqlite::{Connection, OptionalExtension};
use tauri::AppHandle;

pub(super) fn query_exact(conn: &Connection, word: &str) -> Result<Option<DictEntry>, String> {
    conn.query_row(
        "SELECT word, phonetic, translation FROM stardict WHERE word = ?1 COLLATE NOCASE LIMIT 1",
        [word],
        |r| {
            Ok(row_to_entry(
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub(super) fn query_via_exchange(conn: &Connection, word: &str) -> Result<Option<DictEntry>, String> {

    for cand in lemma_candidates(word) {
        if let Some(e) = query_exact(conn, &cand)? {
            return Ok(Some(e));
        }
    }


    if active_pack() != "slim" {
        return Ok(None);
    }
    let like = format!("%:{word}/%");
    let like_end = format!("%:{word}");
    conn.query_row(
        "SELECT word, phonetic, translation FROM stardict
         WHERE exchange LIKE ?1 OR exchange LIKE ?2
         LIMIT 1",
        rusqlite::params![like, like_end],
        |r| {
            Ok(row_to_entry(
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dict_lookup(app: AppHandle, word: String) -> Result<Option<DictEntry>, String> {
    if !is_wordlike(&word) {
        return Ok(None);
    }
    let key = normalize(&word);
    if key.is_empty() {
        return Ok(None);
    }
    with_db(&app, |conn| {
        if let Some(e) = query_exact(conn, &key)? {
            return Ok(Some(e));
        }

        Ok(query_via_exchange(conn, &key)?.map(|mut e| {
            e.word = key.clone();
            e
        }))
    })
}
