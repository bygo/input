use rusqlite::Connection;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictSense {
    pub pos: String,
    pub gloss: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictEntry {
    pub word: String,
    pub phonetic: Option<String>,
    pub senses: Vec<DictSense>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictPackRow {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub installed: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictStatus {
    pub pack: String,
    pub packs: Vec<DictPackRow>,
    pub busy: bool,
    pub message: String,
    pub percent: u8,
}

pub(super) struct DictProgress {
    pub busy: bool,
    pub message: String,
    pub percent: u8,
}

pub struct DictDb {
    pub(super) conn: Mutex<Option<Connection>>,
    pub(super) progress: Mutex<DictProgress>,
}

pub fn managed() -> DictDb {
    DictDb {
        conn: Mutex::new(None),
        progress: Mutex::new(DictProgress {
            busy: false,
            message: String::new(),
            percent: 0,
        }),
    }
}
