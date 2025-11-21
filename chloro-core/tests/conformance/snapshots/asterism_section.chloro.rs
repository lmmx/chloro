//! Section representation for tree-sitter parsed documents.
//!
//! A section represents a hierarchical division of a document, typically
//! corresponding to a heading in markdown. Sections track their position
//! in the document tree through parent/child relationships and maintain
//! precise byte and line coordinates for content extraction and modification.
#[derive(Clone)]
pub struct Section {
    pub title: String,
    pub level: usize,
    pub line_start: i64,
    pub line_end: i64,
    pub column_start: i64,
    pub column_end: i64,
    pub byte_start: usize,
    pub byte_end: usize,
    pub file_path: String,
    pub parent_index: Option<usize>,
    pub children_indices: Vec<usize>,
    pub section_content: Option<Vec<String>>,
    pub chunk_type: Option<ChunkType>,
    pub lhs_content: Option<String>,
    pub rhs_content: Option<String>,
}

/// What sort of hunk (syntactic diff atomic unit) it is.
#[derive(Clone)]
pub enum ChunkType {
    /// Only RHS exists
    Added,
    /// Only LHS exists
    Deleted,
    /// Both LHS and RHS exist (and differ)
    Modified,
    /// Both LHS and RHS exist (and are the same, at least syntactically)
    Unchanged,
}

/// Types of nodes that can appear in the file tree view.
#[derive(Clone)]
pub enum NodeType {
    /// Directory node showing a path component
    Directory {
        /// Directory name (not full path)
        name: String,
        /// Full path for reference
        path: String,
    },
    /// File node (non-navigable, just shows filename)
    File {
        /// File name
        name: String,
        /// Full path for reference
        path: String,
    },
    /// Actual document section (navigable)
    Section(Section),
}

#[derive(Clone)]
pub struct TreeNode {
    pub node_type: NodeType,
    pub tree_level: usize,
    pub navigable: bool,
    pub section_index: Option<usize>,
}

impl TreeNode {
    #[must_use]
    pub fn directory(name: String, path: String, tree_level: usize) -> Self {
        Self {
            node_type: NodeType::Directory { name, path },
            tree_level,
            navigable: false,
            section_index: None,
        }
    }
    #[must_use]
    pub fn file(name: String, path: String, tree_level: usize) -> Self {
        Self {
            node_type: NodeType::File { name, path },
            tree_level,
            navigable: false,
            section_index: None,
        }
    }
    #[must_use]
    pub fn section(section: Section, tree_level: usize, section_index: usize) -> Self {
        Self {
            node_type: NodeType::Section(section),
            tree_level,
            navigable: true,
            section_index: Some(section_index),
        }
    }
}
