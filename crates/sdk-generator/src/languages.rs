//! Language generators

/// Supported programming languages
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Language {
    /// Rust programming language
    Rust,
    /// Python programming language
    Python,
    /// JavaScript programming language
    JavaScript,
    /// TypeScript programming language
    TypeScript,
    /// Go programming language
    Go,
    /// Java programming language
    Java,
    /// C# programming language
    CSharp,
    /// PHP programming language
    PHP,
    /// Custom language
    Custom(String),
}

/// Language generator trait
pub trait LanguageGenerator {
    fn generate(&self) -> String;
}

/// Generation context
pub struct GenerationContext;
