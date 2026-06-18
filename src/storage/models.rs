use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    #[serde(rename = "conversation")]
    Conversation,
    #[serde(rename = "knowledge")]
    Knowledge,
    #[serde(rename = "learning")]
    Learning,
    #[serde(rename = "decision")]
    Decision,
    #[serde(rename = "architecture")]
    Architecture,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Conversation => "conversation",
            MemoryType::Knowledge => "knowledge",
            MemoryType::Learning => "learning",
            MemoryType::Decision => "decision",
            MemoryType::Architecture => "architecture",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "conversation" => Some(MemoryType::Conversation),
            "knowledge" => Some(MemoryType::Knowledge),
            "learning" => Some(MemoryType::Learning),
            "decision" => Some(MemoryType::Decision),
            "architecture" => Some(MemoryType::Architecture),
            _ => None,
        }
    }

    pub fn all() -> [MemoryType; 5] {
        [
            MemoryType::Conversation,
            MemoryType::Knowledge,
            MemoryType::Learning,
            MemoryType::Decision,
            MemoryType::Architecture,
        ]
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Sensitivity {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "internal")]
    #[default]
    Internal,
    #[serde(rename = "secret")]
    Secret,
}

impl Sensitivity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Sensitivity::Public => "public",
            Sensitivity::Internal => "internal",
            Sensitivity::Secret => "secret",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "public" => Some(Sensitivity::Public),
            "internal" => Some(Sensitivity::Internal),
            "secret" => Some(Sensitivity::Secret),
            _ => None,
        }
    }
}

impl std::fmt::Display for Sensitivity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUnit {
    pub hash: String,
    pub project: String,
    pub owner_id: String,
    pub memory_type: MemoryType,
    pub essence: String,
    pub summary: Option<String>,
    pub facts: Vec<String>,
    pub source_ref: String,
    pub sensitivity: Sensitivity,
    pub importance: u8,
    pub verified: bool,
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: Option<String>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub last_access: i64,
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreInput {
    pub project: String,
    pub memory_type: MemoryType,
    pub essence: String,
    pub summary: Option<String>,
    pub facts: Vec<String>,
    pub source_ref: String,
    #[serde(default)]
    pub sensitivity: Sensitivity,
    pub importance: Option<u8>,
    #[serde(default)]
    pub tags: Vec<String>,
}
