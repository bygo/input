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

fn exchange_lemma(conn: &Connection, word: &str) -> Result<Option<String>, String> {
    let ex: Option<String> = conn
        .query_row(
            "SELECT exchange FROM stardict WHERE word = ?1 COLLATE NOCASE LIMIT 1",
            [word],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    let Some(ex) = ex else {
        return Ok(None);
    };
    for part in ex.split('/') {
        if let Some(lemma) = part.strip_prefix("0:") {
            let lemma = lemma.trim();
            if lemma.len() >= 2 {
                return Ok(Some(lemma.to_ascii_lowercase()));
            }
        }
    }
    Ok(None)
}

fn regular_s_form(surface: &str, lemma: &str) -> bool {
    let s = surface.to_ascii_lowercase();
    let l = lemma.to_ascii_lowercase();
    s == format!("{l}s")
        || s == format!("{l}es")
        || (l.ends_with('y') && l.len() > 1 && s == format!("{}ies", &l[..l.len() - 1]))
}

fn plural_s_ending(phonetic_core: &str) -> &'static str {
    let last = phonetic_core
        .chars()
        .rev()
        .find(|c| c.is_alphabetic())
        .map(|c| c.to_ascii_lowercase());
    match last {
        Some('s' | 'z' | 'ʃ' | 'ʒ' | 'ɕ' | 'ʑ') => "iz",
        Some('p' | 't' | 'k' | 'f' | 'θ') => "s",
        _ => "z",
    }
}

pub(super) fn with_regular_s_phone(phonetic: &str, surface: &str, lemma: &str) -> String {
    if !regular_s_form(surface, lemma) {
        return phonetic.to_string();
    }
    let (open, core, close) = if phonetic.starts_with('/') && phonetic.ends_with('/') && phonetic.len() >= 2 {
        ("/", &phonetic[1..phonetic.len() - 1], "/")
    } else if phonetic.starts_with('[') && phonetic.ends_with(']') && phonetic.len() >= 2 {
        ("[", &phonetic[1..phonetic.len() - 1], "]")
    } else {
        ("", phonetic, "")
    };
    format!("{open}{core}{}{close}", plural_s_ending(core))
}

pub(super) fn phonetic_from_lemma(conn: &Connection, surface: &str) -> Result<Option<String>, String> {
    let mut lemmas = Vec::new();
    if let Some(lemma) = exchange_lemma(conn, surface)? {
        lemmas.push(lemma);
    }
    for cand in lemma_candidates(surface) {
        if !lemmas.iter().any(|x| x == &cand) {
            lemmas.push(cand);
        }
    }
    for lemma in lemmas {
        if let Some(e) = query_exact(conn, &lemma)? {
            if let Some(p) = e.phonetic {
                return Ok(Some(with_regular_s_phone(&p, surface, &lemma)));
            }
        }
    }
    Ok(None)
}

pub(super) fn query_via_exchange(conn: &Connection, word: &str) -> Result<Option<DictEntry>, String> {

    for cand in lemma_candidates(word) {
        if let Some(mut e) = query_exact(conn, &cand)? {
            if let Some(p) = e.phonetic.take() {
                e.phonetic = Some(with_regular_s_phone(&p, word, &cand));
            }
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
            let lemma = r.get::<_, String>(0)?;
            let mut e = row_to_entry(
                lemma.clone(),
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            );
            if let Some(p) = e.phonetic.take() {
                e.phonetic = Some(with_regular_s_phone(&p, word, &lemma));
            }
            Ok(e)
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
        if let Some(mut e) = query_exact(conn, &key)? {
            if e.phonetic.is_none() {
                e.phonetic = phonetic_from_lemma(conn, &key)?;
            }
            return Ok(Some(e));
        }

        Ok(query_via_exchange(conn, &key)?.map(|mut e| {
            e.word = key.clone();
            e
        }))
    })
}
