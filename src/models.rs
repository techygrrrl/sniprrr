use serde::{Deserialize, Serialize};

/// Snippet
/// Snippets have a title and a description
#[derive(Debug, Serialize, Deserialize)]
pub struct Snippet {
    pub title: String,
    pub description: String,
}
