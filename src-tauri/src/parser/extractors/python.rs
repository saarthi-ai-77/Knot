use crate::parser::{ParsedEntity, ParsedRelationship};

pub fn extract(content: &str, file_path: &str) -> Result<(Vec<ParsedEntity>, Vec<ParsedRelationship>), String> {
    let language = tree_sitter_python::language();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language)
        .map_err(|e| format!("Failed to set language: {}", e))?;
    
    let tree = parser.parse(content, None)
        .ok_or("Failed to parse file")?;
    
    let root_node = tree.root_node();
    let mut entities = Vec::new();
    let mut relationships = Vec::new();
    
    // Walk the tree to extract entities
    extract_from_node(root_node, content, &mut entities, &mut relationships, file_path)?;
    
    Ok((entities, relationships))
}

fn extract_from_node(
    node: tree_sitter::Node,
    content: &str,
    entities: &mut Vec<ParsedEntity>,
    relationships: &mut Vec<ParsedRelationship>,
    file_path: &str,
) -> Result<(), String> {
    let kind = node.kind();
    
    // Extract function definitions
    if kind == "function_definition" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = content[name_node.byte_range()].to_string();
            let signature = content[node.byte_range()].lines().next().unwrap_or(&name).to_string();
            
            // Check for public/private by checking first character
            let is_public = !name.starts_with('_');
            
            entities.push(ParsedEntity {
                name,
                kind: "function".to_string(),
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                signature,
                is_public,
            });
        }
    }
    
    // Extract class definitions
    if kind == "class_definition" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = content[name_node.byte_range()].to_string();
            let signature = content[node.byte_range()].lines().next().unwrap_or(&name).to_string();
            let is_public = !name.starts_with('_');
            
            entities.push(ParsedEntity {
                name: name.clone(),
                kind: "class".to_string(),
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                signature,
                is_public,
            });
            
            // Check for inheritance
            if let Some(superclasses_node) = node.child_by_field_name("superclasses") {
                let superclasses_text = content[superclasses_node.byte_range()].to_string();
                // Parse "(Parent1, Parent2)"
                let cleaned = superclasses_text
                    .trim_start_matches('(')
                    .trim_end_matches(')');
                
                for parent in cleaned.split(',') {
                    let parent_name = parent.trim().to_string();
                    if !parent_name.is_empty() {
                        relationships.push(ParsedRelationship {
                            source_name: name.clone(),
                            target_name: parent_name,
                            kind: "extends".to_string(),
                        });
                    }
                }
            }
        }
    }
    
    // Extract import statements
    if kind == "import_statement" || kind == "import_from_statement" {
        let import_text = content[node.byte_range()].to_string();
        let line_start = node.start_position().row as u32 + 1;
        let line_end = node.end_position().row as u32 + 1;
        
        // Handle "import x" or "from x import y"
        let import_name = if import_text.starts_with("from") {
            // from module import name
            import_text
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown")
                .to_string()
        } else {
            // import module
            import_text
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown")
                .to_string()
        };
        
        entities.push(ParsedEntity {
            name: import_name.clone(),
            kind: "import".to_string(),
            line_start,
            line_end,
            signature: import_text,
            is_public: false,
        });
        
        relationships.push(ParsedRelationship {
            source_name: file_path.to_string(),
            target_name: import_name,
            kind: "imports".to_string(),
        });
    }
    
    // Extract decorated functions/classes
    if kind == "decorated_definition" {
        // Find the actual definition inside
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "function_definition" || child.kind() == "class_definition" {
                extract_from_node(child, content, entities, relationships, file_path)?;
            }
        }
    }
    
    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_from_node(child, content, entities, relationships, file_path)?;
    }
    
    Ok(())
}
