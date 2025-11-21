//! Renders a bit of code as HTML.

use hir::{EditionedFileId, Semantics};
use ide_db::MiniCore;
use oorandom::Rand32;
use stdx::format_to;
use syntax::AstNode;

use crate::{
    FileId, RootDatabase,
    syntax_highlighting::{HighlightConfig, highlight},
};

pub(crate) fn highlight_as_html_with_config(
    db: &RootDatabase,
    config: &HighlightConfig<'_>,
    file_id: FileId,
    rainbow: bool,
) -> String {
    let sema = Semantics::new(db);
    let file_id = sema
        .attach_first_edition(file_id)
        .unwrap_or_else(|| EditionedFileId::current_edition(db, file_id));
    let file = sema.parse(file_id);
    let file = file.syntax();
    fn rainbowify(seed: u64) -> String {
        let mut rng = Rand32::new(seed);
        format!(
            "hsl({h},{s}%,{l}%)",
            h = rng.rand_range(0..361),
            s = rng.rand_range(42..99),
            l = rng.rand_range(40..91),
        )
    }
    let hl_ranges = highlight(db, config, file_id.file_id(db), None);
    let text = file.to_string();
    let mut buf = String::new();
    buf.push_str(STYLE);
    buf.push_str("<pre><code>");
    for r in &hl_ranges {
        let chunk = html_escape(&text[r.range]);
        if r.highlight.is_empty() {
            format_to!(buf, "{}", chunk);
            continue;
        }

        let class = r.highlight.to_string().replace('.', " ");
        let color = match (rainbow, r.binding_hash) {
            (true, Some(hash)) => {
                format!(" data-binding-hash=\"{hash}\" style=\"color: {};\"", rainbowify(hash))
            }
            _ => "".into(),
        };
        format_to!(buf, "<span class=\"{}\"{}>{}</span>", class, color, chunk);
    }
    buf.push_str("</code></pre>");
    buf
}

pub(crate) fn highlight_as_html(
    db: &RootDatabase,
    file_id: FileId,
    rainbow: bool,
) -> String {
    highlight_as_html_with_config(
        db,
        &HighlightConfig {
            strings: true,
            comments: true,
            punctuation: true,
            specialize_punctuation: true,
            specialize_operator: true,
            operator: true,
            inject_doc_comment: true,
            macro_bang: true,
            syntactic_name_ref_highlighting: false,
            minicore: MiniCore::default(),
        },
        file_id,
        rainbow,
    )
}

fn html_escape(text: &str) -> String {
    text.replace('<', "&lt;").replace('>', "&gt;")
}

