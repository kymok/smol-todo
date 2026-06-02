use rand::Rng;
use std::collections::HashSet;

const ID_CHARS: &[u8] = b"0123456789abcdef";
const VERSION_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn random_string(chars: &[u8], len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| chars[rng.gen_range(0..chars.len())] as char)
        .collect()
}

/// 8-character lowercase hex id, unique against `existing`.
pub fn make_id(existing: &HashSet<String>) -> String {
    loop {
        let id = random_string(ID_CHARS, 8);
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// 12-character alphanumeric version, unique against `existing`.
pub fn make_version(existing: &HashSet<String>) -> String {
    loop {
        let version = random_string(VERSION_CHARS, 12);
        if !existing.contains(&version) {
            return version;
        }
    }
}

pub fn is_valid_id(id: &str) -> bool {
    id.len() == 8 && id.bytes().all(|b| ID_CHARS.contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_valid_and_unique() {
        let mut seen = HashSet::new();
        for _ in 0..200 {
            let id = make_id(&seen);
            assert!(is_valid_id(&id), "{id} should be valid");
            assert!(seen.insert(id));
        }
    }

    #[test]
    fn make_id_avoids_existing() {
        // Exhaust nothing, but force collision avoidance with a near-full small probe.
        let existing: HashSet<String> = ["deadbeef".to_string()].into_iter().collect();
        let id = make_id(&existing);
        assert_ne!(id, "deadbeef");
    }

    #[test]
    fn version_is_twelve_alnum() {
        let v = make_version(&HashSet::new());
        assert_eq!(v.len(), 12);
        assert!(v.bytes().all(|b| b.is_ascii_alphanumeric()));
    }

    #[test]
    fn invalid_ids_rejected() {
        assert!(!is_valid_id("abc")); // too short
        assert!(!is_valid_id("deadbeeff")); // too long
        assert!(!is_valid_id("deadbeeg")); // non-hex
        assert!(is_valid_id("0123abcd"));
    }
}
