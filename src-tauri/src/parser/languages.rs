// Language definitions and constants

use tree_sitter::Language;

pub fn get_javascript_language() -> Language {
    tree_sitter_javascript::language()
}

pub fn get_typescript_language() -> Language {
    tree_sitter_typescript::language_typescript()
}

pub fn get_tsx_language() -> Language {
    tree_sitter_typescript::language_tsx()
}

pub fn get_python_language() -> Language {
    tree_sitter_python::language()
}
