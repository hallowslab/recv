use std::collections::HashSet;
use std::sync::RwLock;

use exifflow_core::utils::generate_random_string;

/// Compares two byte slices without short-circuiting on the first difference.
///
/// Length is compared first, which leaks only the length, never the content.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// In-memory set of paired browser sessions. Sessions live only for the
/// server's lifetime and are never persisted.
#[derive(Default)]
pub struct SessionStore {
    sessions: RwLock<HashSet<String>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mints a new session id and records it.
    pub fn create(&self) -> String {
        let id = generate_random_string(48);
        self.sessions.write().unwrap().insert(id.clone());
        id
    }

    /// Returns true when `id` belongs to a live session.
    pub fn is_valid(&self, id: &str) -> bool {
        self.sessions.read().unwrap().contains(id)
    }
}
