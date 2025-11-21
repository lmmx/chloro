//! User (postfix)-snippet definitions.
//!
//! Actual logic is implemented in [`crate::completions::postfix`] and [`crate::completions::snippet`] respectively.

use hir::{ModPath, Name, Symbol};
use ide_db::imports::import_assets::LocatedImport;
use itertools::Itertools;

use crate::context::CompletionContext;

/// A snippet scope describing where a snippet may apply to.
/// These may differ slightly in meaning depending on the snippet trigger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnippetScope {
    Item,
    Expr,
    Type,
}

/// A user supplied snippet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snippet {
    pub postfix_triggers: Box<[Box<str>]>,
    pub prefix_triggers: Box<[Box<str>]>,
    pub scope: SnippetScope,
    pub description: Option<Box<str>>,
    snippet: String,
    requires: Box<[ModPath]>,
}

impl Snippet {
    pub fn new(prefix_triggers: &[String], postfix_triggers: &[String], snippet: &[String], description: &str, requires: &[String], scope: SnippetScope) -> Option<Self> {
        if prefix_triggers.is_empty() && postfix_triggers.is_empty() {
            return None;
        }
        let (requires, snippet, description) = validate_snippet(snippet, description, requires)?;
        Some(Snippet {
            postfix_triggers: postfix_triggers.iter().map(String::as_str).map(Into::into).collect(),
            prefix_triggers: prefix_triggers.iter().map(String::as_str).map(Into::into).collect(),
            scope,
            snippet,
            description,
            requires,
        })
    }

    /// Returns [`None`] if the required items do not resolve.
    pub(crate) fn imports(&self, ctx: &CompletionContext<'_>) -> Option<Vec<LocatedImport>> {
        import_edits(ctx, &self.requires)
    }

    pub fn snippet(&self) -> String {
        self.snippet.replace("${receiver}", "$0")
    }

    pub fn postfix_snippet(&self, receiver: &str) -> String {
        self.snippet.replace("${receiver}", receiver)
    }
}

fn import_edits(ctx: &CompletionContext<'_>, requires: &[ModPath]) -> Option<Vec<LocatedImport>> {
    let import_cfg = ctx.config.find_path_config(ctx.is_nightly);
    let resolve = |import| {
        let item = ctx.scope.resolve_mod_path(import).next()?;
        let path = ctx.module.find_use_path(
            ctx.db,
            item,
            ctx.config.insert_use.prefix_kind,
            import_cfg,
        )?;
        Some((path.len() > 1).then(|| LocatedImport::new_no_completion(path.clone(), item, item)))
    };
    let mut res = Vec::with_capacity(requires.len());
    for import in requires {
        match resolve(import) {
            Some(first) => res.extend(first),
            None => return None,
        }
    }
    Some(res)
}

fn validate_snippet(snippet: &[String], description: &str, requires: &[String]) -> Option<(Box<[ModPath]>, String, Option<Box<str>>)> {
    let mut imports = Vec::with_capacity(requires.len());
    for path in requires.iter() {
        let use_path = ModPath::from_segments(
            hir::PathKind::Plain,
            path.split("::").map(Symbol::intern).map(Name::new_symbol_root),
        );
        imports.push(use_path);
    }
    let snippet = snippet.iter().join("\n");
    let description = (!description.is_empty())
        .then(|| description.split_once('\n').map_or(description, |(it, _)| it))
        .map(ToOwned::to_owned)
        .map(Into::into);
    Some((imports.into_boxed_slice(), snippet, description))
}
