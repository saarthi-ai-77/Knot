use crate::parser::{ParsedEntity, ParsedRelationship};

pub fn extract(content: &str, file_path: &str) -> Result<(Vec<ParsedEntity>, Vec<ParsedRelationship>), String> {
    let language = tree_sitter_javascript::language();
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
    
    // Extract functions
    if kind == "function_declaration" || kind == "function" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = content[name_node.byte_range()].to_string();
            let signature = content[node.byte_range()].lines().next().unwrap_or(&name).to_string();
            
            entities.push(ParsedEntity {
                name,
                kind: "function".to_string(),
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                signature,
                is_public: true,
            });
        }
    }
    
    // Extract arrow functions (const x = () => {})
    if kind == "lexical_declaration" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = content[name_node.byte_range()].to_string();
                    
                    // Check if value is an arrow function
                    if let Some(value_node) = child.child_by_field_name("value") {
                        if value_node.kind() == "arrow_function" {
                            let signature = content[child.byte_range()].lines().next().unwrap_or(&name).to_string();
                            
                            entities.push(ParsedEntity {
                                name,
                                kind: "function".to_string(),
                                line_start: node.start_position().row as u32 + 1,
                                line_end: node.end_position().row as u32 + 1,
                                signature,
                                is_public: true,
                            });
                        }
                    }
                }
            }
        }
    }
    
    // Extract class declarations
    if kind == "class_declaration" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = content[name_node.byte_range()].to_string();
            let signature = content[node.byte_range()].lines().next().unwrap_or(&name).to_string();
            
            entities.push(ParsedEntity {
                name: name.clone(),
                kind: "class".to_string(),
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                signature,
                is_public: true,
            });
            
            // Check for extends
            if let Some(super_node) = node.child_by_field_name("superclass") {
                let super_name = content[super_node.byte_range()].to_string();
                relationships.push(ParsedRelationship {
                    source_name: name.clone(),
                    target_name: super_name,
                    kind: "extends".to_string(),
                });
            }
        }
    }
    
    // Extract method definitions
    if kind == "method_definition" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = content[name_node.byte_range()].to_string();
            let signature = content[node.byte_range()].lines().next().unwrap_or(&name).to_string();
            
            entities.push(ParsedEntity {
                name,
                kind: "function".to_string(),
                line_start: node.start_position().row as u32 + 1,
                line_end: node.end_position().row as u32 + 1,
                signature,
                is_public: true,
            });
        }
    }
    
    // Extract imports
    if kind == "import_statement" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "import_clause" || child.kind() == "identifier" {
                let import_name = content[child.byte_range()].to_string();
                
                entities.push(ParsedEntity {
                    name: import_name.clone(),
                    kind: "import".to_string(),
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    signature: content[node.byte_range()].to_string(),
                    is_public: false,
                });
                
                // Get source
                if let Some(source_node) = node.child_by_field_name("source") {
                    let source = content[source_node.byte_range()]
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    
                    relationships.push(ParsedRelationship {
                        source_name: import_name,
                        target_name: source,
                        kind: "imports".to_string(),
                    });
                }
            }
        }
    }
    
    // Extract exports
    if kind == "export_statement" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let export_name = content[child.byte_range()].to_string();
                
                entities.push(ParsedEntity {
                    name: export_name.clone(),
                    kind: "export".to_string(),
                    line_start: node.start_position().row as u32 + 1,
                    line_end: node.end_position().row as u32 + 1,
                    signature: content[node.byte_range()].to_string(),
                    is_public: true,
                });
                
                relationships.push(ParsedRelationship {
                    source_name: file_path.to_string(),
                    target_name: export_name,
                    kind: "exports".to_string(),
                });
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
