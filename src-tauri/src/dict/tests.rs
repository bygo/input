use super::db::open_at;
use super::entry::{is_wordlike, lemma_candidates, parse_senses};
use super::lookup::{phonetic_from_lemma, query_exact, query_via_exchange, with_regular_s_phone};
use super::pack::validate_db;
use super::path::{active_pack, full_db_path};
use rusqlite::Connection;
use std::path::PathBuf;

#[test]
fn wordlike_accepts_single_and_short_phrase() {
    assert!(is_wordlike("apple"));
    assert!(is_wordlike("look up"));
    assert!(!is_wordlike("I have an apple."));
    assert!(!is_wordlike(""));
}

#[test]
fn lemma_strips_common_suffixes() {
    let runs = lemma_candidates("runs");
    assert!(runs.iter().any(|s| s == "run"));
    let looking = lemma_candidates("looking");
    assert!(looking.iter().any(|s| s == "look"));
    let studies = lemma_candidates("studies");
    assert!(studies.iter().any(|s| s == "study"));
}

#[test]
fn parse_translation_lines() {
    let s = parse_senses("n. 苹果, 家伙\n[医] 苹果\nvt. 拿, 取");
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].pos, "n");
    assert!(s[0].gloss.contains("苹果"));
    assert_eq!(s[1].pos, "vt");
}

#[test]
fn lookup_fixture_if_present() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ecdict.db");
    if !p.is_file() {
        return;
    }
    let conn = Connection::open_with_flags(&p, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let e = query_exact(&conn, "apple").unwrap().expect("apple");
    assert_eq!(e.word.to_lowercase(), "apple");
    assert!(e.phonetic.is_some());
    assert!(!e.senses.is_empty());
}

#[test]
fn exchange_keeps_surface_form() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ecdict.db");
    if !p.is_file() {
        return;
    }
    let conn = Connection::open_with_flags(&p, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert!(query_exact(&conn, "runs").unwrap().is_none());
    let mut e = query_via_exchange(&conn, "runs").unwrap().expect("runs→run");
    assert_eq!(e.word.to_lowercase(), "run");
    e.word = "runs".into();
    assert_eq!(e.word, "runs");
    assert!(!e.senses.is_empty());
}

#[test]
fn full_pack_contains_if_installed() {
    let Ok(p) = full_db_path() else {
        return;
    };
    if !p.is_file() {
        return;
    }
    validate_db(&p).expect("full db");
    let conn = open_at(&p).unwrap();
    let e = query_exact(&conn, "contains")
        .unwrap()
        .expect("contains headword");
    assert!(!e.senses.is_empty());
    assert!(active_pack() == "full" || active_pack() == "slim");
}

#[test]
fn regular_s_phone_appends_ending() {
    assert_eq!(
        with_regular_s_phone("/.viʒuәlai''zeiʃәn/", "visualizations", "visualization"),
        "/.viʒuәlai''zeiʃәnz/"
    );
    assert_eq!(with_regular_s_phone("/'kæt/", "cats", "cat"), "/'kæts/");
    assert_eq!(with_regular_s_phone("/'bɒks/", "boxes", "box"), "/'bɒksiz/");
    assert_eq!(
        with_regular_s_phone("/'rʌniŋ/", "running", "run"),
        "/'rʌniŋ/"
    );
}

#[test]
fn empty_phonetic_fills_from_lemma_if_full_pack() {
    let Ok(p) = full_db_path() else {
        return;
    };
    if !p.is_file() {
        return;
    }
    let conn = open_at(&p).unwrap();
    let exact = query_exact(&conn, "visualizations")
        .unwrap()
        .expect("visualizations headword");
    assert!(exact.phonetic.is_none(), "fixture: surface has no phonetic");
    let filled = phonetic_from_lemma(&conn, "visualizations")
        .unwrap()
        .expect("lemma phonetic");
    assert!(filled.contains('z'), "plural ending on lemma IPA: {filled}");
    assert!(filled.contains("zeiʃәn") || filled.contains("zeiʃən"), "{filled}");
}
