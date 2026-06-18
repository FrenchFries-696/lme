use lme::storage::models::{MemoryType, Sensitivity, StoreInput};
use lme::storage::Storage;

fn make_input(essence: &str, facts: Vec<&str>, project: &str) -> StoreInput {
    StoreInput {
        project: project.to_string(),
        memory_type: MemoryType::Knowledge,
        essence: essence.to_string(),
        summary: Some(format!("Summary: {}", essence)),
        facts: facts.iter().map(|s| s.to_string()).collect(),
        source_ref: "tests/conversation-001.md".to_string(),
        sensitivity: Sensitivity::Internal,
        importance: Some(3),
        tags: vec![],
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn test_insert_and_get() {
    let storage = Storage::open_in_memory().unwrap();
    let input = make_input("test essence", vec!["fact1".into()], "test-proj");
    let ts = now();

    let hash = storage.insert_memory(&input, None, ts).unwrap();
    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 64); // SHA-256 hex

    let memory = storage.get_by_hash(&hash).unwrap();
    assert_eq!(memory.essence, "test essence");
    assert_eq!(memory.project, "test-proj");
    assert_eq!(memory.facts, vec!["fact1"]);
}

#[test]
fn test_dedup_same_hash_skips_insert() {
    let storage = Storage::open_in_memory().unwrap();
    let input = make_input("dedup test", vec!["f1"], "dedup-proj");
    let ts = now();

    let hash1 = storage.insert_memory(&input, None, ts).unwrap();
    let hash2 = storage.insert_memory(&input, None, ts + 10).unwrap();

    // Same hash returned
    assert_eq!(hash1, hash2);

    // Only one row exists
    let count = storage.count_by_project("dedup-proj").unwrap();
    assert_eq!(count, 1);

    // last_access was updated to newer timestamp
    let memory = storage.get_by_hash(&hash1).unwrap();
    assert_eq!(memory.last_access, ts + 10);
}

#[test]
fn test_fts5_search_vietnamese() {
    let storage = Storage::open_in_memory().unwrap();
    let ts = now();

    let vn = make_input(
        "quyết định kiến trúc hệ thống",
        vec!["dùng Rust", "single binary"],
        "vn-proj",
    );
    let en = make_input(
        "architecture decision about caching",
        vec!["use Redis"],
        "vn-proj",
    );

    storage.insert_memory(&vn, None, ts).unwrap();
    storage.insert_memory(&en, None, ts).unwrap();

    // Search in Vietnamese
    let results = storage
        .search_fts("quyết định", Some("vn-proj"), 10)
        .unwrap();
    assert!(!results.is_empty());
    // Vietnamese result should rank higher
    assert_eq!(results[0].essence, "quyết định kiến trúc hệ thống");

    // English search
    let results = storage
        .search_fts("caching", Some("vn-proj"), 10)
        .unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].essence, "architecture decision about caching");
}

#[test]
fn test_list_by_project() {
    let storage = Storage::open_in_memory().unwrap();
    let ts = now();

    let a = make_input("memory a", vec!["a1"], "list-proj");
    let b = make_input("memory b", vec!["b1"], "list-proj");

    storage.insert_memory(&a, None, ts).unwrap();
    storage.insert_memory(&b, None, ts + 1).unwrap();

    let list = storage.list_by_project("list-proj").unwrap();
    assert_eq!(list.len(), 2);
    // Most recent first
    assert_eq!(list[0].essence, "memory b");
    assert_eq!(list[1].essence, "memory a");
}

#[test]
fn test_update_last_access_reheating() {
    let storage = Storage::open_in_memory().unwrap();
    let ts = now();

    let input = make_input("reheat test", vec!["r1"], "reheat-proj");
    let hash = storage.insert_memory(&input, None, ts).unwrap();

    let new_ts = ts + 3600;
    storage.update_last_access(&hash, new_ts).unwrap();

    let memory = storage.get_by_hash(&hash).unwrap();
    assert_eq!(memory.last_access, new_ts);
}

#[test]
fn test_mark_superseded() {
    let storage = Storage::open_in_memory().unwrap();
    let ts = now();

    let old = make_input("old fact: x=1", vec!["x=1"], "conflict-proj");
    let new = make_input("new fact: x=2", vec!["x=2"], "conflict-proj");

    let old_hash = storage.insert_memory(&old, None, ts).unwrap();
    let new_hash = storage.insert_memory(&new, None, ts).unwrap();

    storage
        .mark_superseded(&old_hash, &new_hash)
        .unwrap();

    let old_memory = storage.get_by_hash(&old_hash).unwrap();
    assert_eq!(old_memory.superseded_by.unwrap(), new_hash);
}

#[test]
fn test_count_by_type() {
    let storage = Storage::open_in_memory().unwrap();
    let ts = now();

    let mut k = make_input("knowledge 1", vec!["k1"], "count-proj");
    k.memory_type = MemoryType::Knowledge;
    let mut d = make_input("decision 1", vec!["d1"], "count-proj");
    d.memory_type = MemoryType::Decision;

    storage.insert_memory(&k, None, ts).unwrap();
    storage.insert_memory(&d, None, ts).unwrap();

    let counts = storage.count_by_type("count-proj").unwrap();
    assert_eq!(*counts.get(&MemoryType::Knowledge).unwrap(), 1);
    assert_eq!(*counts.get(&MemoryType::Decision).unwrap(), 1);
    assert_eq!(*counts.get(&MemoryType::Conversation).unwrap(), 0);
    assert_eq!(*counts.get(&MemoryType::Learning).unwrap(), 0);
    assert_eq!(*counts.get(&MemoryType::Architecture).unwrap(), 0);
}

#[test]
fn test_wal_mode_active() {
    let storage = Storage::open_in_memory().unwrap();
    let mode: String = storage
        .conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    // WAL or memory — both acceptable (in-memory DBs may report "memory")
    let mode_lower = mode.to_lowercase();
    assert!(mode_lower == "wal" || mode_lower == "memory",
        "expected wal or memory, got: {}", mode);
}

#[test]
fn test_db_size_bytes() {
    let storage = Storage::open_in_memory().unwrap();
    let ts = now();

    let input = make_input("size test", vec!["s1"], "size-proj");
    storage.insert_memory(&input, None, ts).unwrap();

    let size = storage.db_size_bytes().unwrap();
    assert!(size > 0);
}

#[test]
fn test_not_found_error() {
    let storage = Storage::open_in_memory().unwrap();
    let result = storage.get_by_hash("nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, lme::error::LmeError::NotFound(_)));
}

#[test]
fn test_empty_project_list() {
    let storage = Storage::open_in_memory().unwrap();
    let list = storage.list_by_project("no-memories-here").unwrap();
    assert!(list.is_empty());
}

#[test]
fn test_empty_fts5_search() {
    let storage = Storage::open_in_memory().unwrap();
    let results = storage
        .search_fts("nothing matches this", None, 10)
        .unwrap();
    assert!(results.is_empty());
}
