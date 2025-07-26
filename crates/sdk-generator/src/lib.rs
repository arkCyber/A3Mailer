//! # Stalwart SDK Generator
//!
//! Multi-language SDK generator for Stalwart Mail Server.
//! Automatically generates client SDKs in various programming languages
//! from API specifications, making it easy for developers to integrate
//! with Stalwart Mail Server.
//!
//! ## Features
//!
//! - **Multi-language Support**: Generate SDKs for Rust, Python, JavaScript, Go, Java, C#, PHP
//! - **OpenAPI Integration**: Generate from OpenAPI/Swagger specifications
//! - **GraphQL Support**: Generate GraphQL schema and resolvers
//! - **Template Engine**: Customizable code generation templates
//! - **Documentation Generation**: Automatic API documentation
//! - **Type Safety**: Strong typing in generated SDKs
//! - **Async Support**: Async/await patterns where applicable
//!
//! ## Architecture
//!
//! The SDK generator consists of:
//! - Generator Manager: Main generation orchestrator
//! - Language Generators: Language-specific code generators
//! - Template Engine: Customizable template system
//! - API Introspector: API specification analyzer
//! - Documentation Generator: API documentation generator
//!
//! ## Example
//!
//! ```rust,no_run
//! use stalwart_sdk_generator::{SdkGenerator, GeneratorConfig, Language};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = GeneratorConfig::default();
//!     let generator = SdkGenerator::new(config).await?;
//!
//!     // Generate Python SDK
//!     generator.generate_sdk(Language::Python, "output/python").await?;
//!
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;
use std::collections::HashMap;
use tracing::{info, warn, error};

pub mod config;
pub mod generator;
pub mod languages;
pub mod templates;
pub mod introspection;
pub mod documentation;
pub mod openapi;
pub mod graphql;
pub mod error;

pub use config::GeneratorConfig;
pub use generator::SdkGenerator;
pub use languages::{Language, LanguageGenerator, GenerationContext};
pub use templates::{TemplateEngine, Template, TemplateContext};
pub use introspection::{ApiIntrospector, ApiSpec, EndpointSpec};
pub use documentation::{DocumentationGenerator, DocFormat};
pub use error::{GeneratorError, Result};



/// SDK generation options
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerationOptions {
    /// Target language
    pub language: Language,
    /// Output directory
    pub output_dir: PathBuf,
    /// Package name
    pub package_name: String,
    /// Package version
    pub package_version: String,
    /// Include documentation
    pub include_docs: bool,
    /// Include examples
    pub include_examples: bool,
    /// Include tests
    pub include_tests: bool,
    /// Custom templates directory
    pub custom_templates: Option<PathBuf>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Generated SDK artifact
#[derive(Debug, Clone)]
pub struct SdkArtifact {
    /// Language
    pub language: Language,
    /// Generated files
    pub files: Vec<GeneratedFile>,
    /// Package metadata
    pub metadata: SdkMetadata,
    /// Generation timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Generated file information
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// File path relative to output directory
    pub path: PathBuf,
    /// File content
    pub content: String,
    /// File type
    pub file_type: FileType,
}

/// File types in generated SDK
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    /// Source code file
    Source,
    /// Configuration file
    Config,
    /// Documentation file
    Documentation,
    /// Example file
    Example,
    /// Test file
    Test,
    /// Build script
    Build,
}

/// SDK metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SdkMetadata {
    /// SDK name
    pub name: String,
    /// SDK version
    pub version: String,
    /// Target language
    pub language: Language,
    /// API version
    pub api_version: String,
    /// Generator version
    pub generator_version: String,
    /// Dependencies
    pub dependencies: Vec<Dependency>,
    /// Build instructions
    pub build_instructions: Option<String>,
    /// Installation instructions
    pub installation_instructions: Option<String>,
}

/// SDK dependency
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    /// Dependency name
    pub name: String,
    /// Version requirement
    pub version: String,
    /// Optional dependency
    pub optional: bool,
    /// Development dependency
    pub dev_only: bool,
}

/// Main SDK generator context
pub struct SdkGeneratorContext {
    pub config: GeneratorConfig,
    pub api_spec: Option<ApiSpec>,
    pub template_engine: TemplateEngine,
    pub supported_languages: Vec<Language>,
}

impl SdkGeneratorContext {
    /// Create a new SDK generator context
    pub fn new(config: GeneratorConfig) -> Self {
        let template_engine = TemplateEngine::new();
        let supported_languages = vec![
            Language::Rust,
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
            Language::Go,
            Language::Java,
            Language::CSharp,
            Language::PHP,
        ];

        Self {
            config,
            api_spec: None,
            template_engine,
            supported_languages,
        }
    }

    /// Set API specification
    pub fn set_api_spec(&mut self, api_spec: ApiSpec) {
        self.api_spec = Some(api_spec);
    }

    /// Get supported languages
    pub fn supported_languages(&self) -> &[Language] {
        &self.supported_languages
    }

    /// Check if language is supported
    pub fn is_language_supported(&self, language: &Language) -> bool {
        self.supported_languages.contains(language)
    }
}

/// Initialize the SDK generator
pub async fn init_sdk_generator(config: GeneratorConfig) -> Result<SdkGeneratorContext> {
    info!("Initializing SDK generator");

    let context = SdkGeneratorContext::new(config);

    // TODO: Initialize generator components
    // - Set up template engine
    // - Load API specifications
    // - Initialize language generators
    // - Set up documentation generator

    info!("SDK generator initialized successfully");
    Ok(context)
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            language: Language::Rust,
            output_dir: PathBuf::from("./generated"),
            package_name: "stalwart-client".to_string(),
            package_version: "0.1.0".to_string(),
            include_docs: true,
            include_examples: true,
            include_tests: true,
            custom_templates: None,
            metadata: HashMap::new(),
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "rust"),
            Language::Python => write!(f, "python"),
            Language::JavaScript => write!(f, "javascript"),
            Language::TypeScript => write!(f, "typescript"),
            Language::Go => write!(f, "go"),
            Language::Java => write!(f, "java"),
            Language::CSharp => write!(f, "csharp"),
            Language::PHP => write!(f, "php"),
            Language::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sdk_generator_context_creation() {
        let config = GeneratorConfig::default();
        let context = SdkGeneratorContext::new(config);

        assert!(context.is_language_supported(&Language::Rust));
        assert!(context.is_language_supported(&Language::Python));
        assert!(!context.is_language_supported(&Language::Custom("unknown".to_string())));
    }

    #[test]
    fn test_language_display() {
        assert_eq!(Language::Rust.to_string(), "rust");
        assert_eq!(Language::Python.to_string(), "python");
        assert_eq!(Language::Custom("kotlin".to_string()).to_string(), "kotlin");
    }

    #[test]
    fn test_default_generation_options() {
        let options = GenerationOptions::default();
        assert_eq!(options.language, Language::Rust);
        assert_eq!(options.package_name, "stalwart-client");
        assert!(options.include_docs);
    }
}
