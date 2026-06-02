use pond_core::TaskStore;
use taskpond_cli::run;

// Two CLI invocations against the same store file must see each other's writes.
#[test]
fn cli_invocations_share_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = TaskStore::new(dir.path().join("tasks.json"));

    let create: Vec<String> = ["taskpond", "item", "create", "-c", "Inbox", "Ship", "it"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut sink: Vec<u8> = Vec::new();
    run(&create, &store, &mut sink).unwrap();

    // A fresh TaskStore on the same path sees the item.
    let store2 = TaskStore::new(dir.path().join("tasks.json"));
    let get: Vec<String> = ["taskpond", "item", "get"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut out: Vec<u8> = Vec::new();
    run(&get, &store2, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("Ship it"));
}
