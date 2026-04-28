pub mod extractors;
pub mod languages;

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParsedEntity {
    pub name: String,
    pub kind: String, // "function" | "class" | "interface" | "import" | "export"
    pub line_start: u32,
    pub line_end: u32,
    pub signature: String,
    pub is_public: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParsedRelationship {
    pub source_name: String,
    pub target_name: String,
    pub kind: String, // "imports" | "exports" | "calls" | "extends" | "implements"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParsedFile {
    pub file_path: String,
    pub language: String,
    pub entities: Vec<ParsedEntity>,
    pub relationships: Vec<ParsedRelationship>,
}

#[derive(Debug, Clone, Copy)]
pub enum Language {
    JavaScript,
    TypeScript,
    TSX,
    Python,
    Rust,
    Go,
    Java,
}

pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    
    match ext {
        "js" | "mjs" | "cjs" => Some(Language::JavaScript),
        "ts" | "mts" | "cts" => Some(Language::TypeScript),
        "tsx" => Some(Language::TSX),
        "py" => Some(Language::Python),
        "rs" => Some(Language::Rust),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        _ => detect_from_shebang(path),
    }
}

fn detect_from_shebang(path: &Path) -> Option<Language> {
    // Read first line to check for shebang
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Some(first_line) = content.lines().next() {
            if first_line.starts_with("#!/usr/bin/env python") || first_line.starts_with("#!/usr/bin/python") {
                return Some(Language::Python);
            }
        }
    }
    None
}

pub async fn parse_file(file_path: &str) -> Result<ParsedFile, String> {
    let path = Path::new(file_path);
    
    let language = detect_language(path)
        .ok_or_else(|| format!("Unsupported file type: {}", file_path))?;
    
    let content = tokio::fs::read_to_string(file_path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let (entities, relationships) = match language {
        Language::JavaScript => extractors::javascript::extract(&content, file_path),
        Language::TypeScript => extractors::typescript::extract(&content, file_path, false),
        Language::TSX => extractors::typescript::extract(&content, file_path, true),
        Language::Python => extractors::python::extract(&content, file_path),
        _ => Err(format!("Language not yet implemented: {:?}", language)),
    }?;
    
    let lang_str = match language {
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::TSX => "tsx",
        Language::Python => "python",
        Language::Rust => "rust",
        Language::Go => "go",
        Language::Java => "java",
    };
    
    Ok(ParsedFile {
        file_path: file_path.to_string(),
        language: lang_str.to_string(),
        entities,
        relationships,
    })
}
