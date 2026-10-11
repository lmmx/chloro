//! Indentation and width budgets (rustfmt's `shape.rs`).
//!
//! A [`Shape`] describes the space a rewrite may occupy: `width` columns on the first line,
//! with continuation lines starting at `indent`. Every `rewrite` function receives a shape
//! and returns `None` when its output cannot fit.

use std::borrow::Cow;
use std::ops::{Add, Sub};

use super::config::Settings;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Indent {
    /// Width of the block indent, in characters. Must be a multiple of `Config::tab_spaces`.
    pub(crate) block_indent: usize,
    /// Alignment in characters.
    pub(crate) alignment: usize,
}

// INDENT_BUFFER.len() = 81
const INDENT_BUFFER_LEN: usize = 80;
const INDENT_BUFFER: &str =
    "\n                                                                                ";

impl Indent {
    pub(crate) fn new(block_indent: usize, alignment: usize) -> Indent {
        Indent {
            block_indent,
            alignment,
        }
    }

    pub(crate) fn from_width(config: &Settings, width: usize) -> Indent {
        if config.hard_tabs() {
            let tab_num = width / config.tab_spaces();
            let alignment = width % config.tab_spaces();
            Indent::new(config.tab_spaces() * tab_num, alignment)
        } else {
            Indent::new(width, 0)
        }
    }

    pub(crate) fn empty() -> Indent {
        Indent::new(0, 0)
    }

    pub(crate) fn block_only(&self) -> Indent {
        Indent {
            block_indent: self.block_indent,
            alignment: 0,
        }
    }

    pub(crate) fn block_indent(mut self, config: &Settings) -> Indent {
        self.block_indent += config.tab_spaces();
        self
    }

    pub(crate) fn block_unindent(mut self, config: &Settings) -> Indent {
        if self.block_indent < config.tab_spaces() {
            Indent::new(self.block_indent, 0)
        } else {
            self.block_indent -= config.tab_spaces();
            self
        }
    }

    pub(crate) fn width(&self) -> usize {
        self.block_indent + self.alignment
    }

    pub(crate) fn to_string(self, config: &Settings) -> Cow<'static, str> {
        self.to_string_inner(config, 1)
    }

    pub(crate) fn to_string_with_newline(self, config: &Settings) -> Cow<'static, str> {
        self.to_string_inner(config, 0)
    }

    fn to_string_inner(self, config: &Settings, offset: usize) -> Cow<'static, str> {
        let (num_tabs, num_spaces) = if config.hard_tabs() {
            (self.block_indent / config.tab_spaces(), self.alignment)
        } else {
            (0, self.width())
        };
        let num_chars = num_tabs + num_spaces;
        if num_tabs == 0 && num_chars + offset <= INDENT_BUFFER_LEN {
            Cow::from(&INDENT_BUFFER[offset..=num_chars])
        } else {
            let mut indent = String::with_capacity(num_chars + if offset == 0 { 1 } else { 0 });
            if offset == 0 {
                indent.push('\n');
            }
            for _ in 0..num_tabs {
                indent.push('\t');
            }
            for _ in 0..num_spaces {
                indent.push(' ');
            }
            Cow::from(indent)
        }
    }
}

impl Add for Indent {
    type Output = Indent;

    fn add(self, rhs: Indent) -> Indent {
        Indent {
            block_indent: self.block_indent + rhs.block_indent,
            alignment: self.alignment + rhs.alignment,
        }
    }
}

impl Sub for Indent {
    type Output = Indent;

    fn sub(self, rhs: Indent) -> Indent {
        Indent::new(
            self.block_indent - rhs.block_indent,
            self.alignment - rhs.alignment,
        )
    }
}

impl Add<usize> for Indent {
    type Output = Indent;

    fn add(self, rhs: usize) -> Indent {
        Indent::new(self.block_indent, self.alignment + rhs)
    }
}

impl Sub<usize> for Indent {
    type Output = Indent;

    fn sub(self, rhs: usize) -> Indent {
        Indent::new(self.block_indent, self.alignment - rhs)
    }
}

// 8096 is close enough to infinite for rustfmt.
const INFINITE_SHAPE_WIDTH: usize = 8096;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Shape {
    pub(crate) width: usize,
    /// The current indentation of code.
    pub(crate) indent: Indent,
    /// Indentation + any already emitted text on the first line of the current statement.
    pub(crate) offset: usize,
}

impl Shape {
    /// `indent` is the indentation of the first line. The next lines
    /// should begin with at least `indent` spaces (except backwards
    /// indentation). The first line should not begin with indentation.
    /// `width` is the maximum number of characters on the last line
    /// (excluding `indent`). The width of other lines is not limited by
    /// `width`.
    pub(crate) fn legacy(width: usize, indent: Indent) -> Shape {
        Shape {
            width,
            indent,
            offset: indent.alignment,
        }
    }

    pub(crate) fn indented(indent: Indent, config: &Settings) -> Shape {
        Shape {
            width: config.max_width().saturating_sub(indent.width()),
            indent,
            offset: indent.alignment,
        }
    }

    pub(crate) fn with_max_width(&self, config: &Settings) -> Shape {
        Shape {
            width: config.max_width().saturating_sub(self.indent.width()),
            ..*self
        }
    }

    pub(crate) fn visual_indent(&self, delta: usize) -> Shape {
        let alignment = self.offset + delta;
        Shape {
            width: self.width,
            indent: Indent::new(self.indent.block_indent, alignment),
            offset: alignment,
        }
    }

    pub(crate) fn block_indent(&self, delta: usize) -> Shape {
        if self.indent.alignment == 0 {
            Shape {
                width: self.width,
                indent: Indent::new(self.indent.block_indent + delta, 0),
                offset: 0,
            }
        } else {
            Shape {
                width: self.width,
                indent: self.indent + delta,
                offset: self.indent.alignment + delta,
            }
        }
    }

    pub(crate) fn block_left(&self, delta: usize) -> Option<Shape> {
        self.block_indent(delta).sub_width(delta)
    }

    pub(crate) fn add_offset(&self, delta: usize) -> Shape {
        Shape {
            offset: self.offset + delta,
            ..*self
        }
    }

    pub(crate) fn block(&self) -> Shape {
        Shape {
            indent: self.indent.block_only(),
            ..*self
        }
    }

    pub(crate) fn saturating_sub_width(&self, delta: usize) -> Shape {
        Shape {
            width: self.width.saturating_sub(delta),
            ..*self
        }
    }

    pub(crate) fn sub_width(&self, delta: usize) -> Option<Shape> {
        self.width
            .checked_sub(delta)
            .map(|width| Shape { width, ..*self })
    }

    pub(crate) fn shrink_left(&self, delta: usize) -> Option<Shape> {
        self.width.checked_sub(delta).map(|width| Shape {
            width,
            indent: self.indent + delta,
            offset: self.offset + delta,
        })
    }

    pub(crate) fn offset_left(&self, delta: usize) -> Option<Shape> {
        self.add_offset(delta).sub_width(delta)
    }

    pub(crate) fn used_width(&self) -> usize {
        self.indent.block_indent + self.offset
    }

    pub(crate) fn rhs_overhead(&self, config: &Settings) -> usize {
        config
            .max_width()
            .saturating_sub(self.used_width() + self.width)
    }

    pub(crate) fn comment(&self, config: &Settings) -> Shape {
        let width = self
            .width
            .min(config.comment_width().saturating_sub(self.indent.width()));
        Shape { width, ..*self }
    }

    pub(crate) fn to_string_with_newline(self, config: &Settings) -> Cow<'static, str> {
        let mut offset_indent = self.indent;
        offset_indent.alignment = self.offset;
        offset_indent.to_string_inner(config, 0)
    }

    /// Creates a `Shape` with a virtually infinite width.
    pub(crate) fn infinite_width(&self) -> Shape {
        Shape {
            width: INFINITE_SHAPE_WIDTH,
            ..*self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indent_add_sub() {
        let indent = Indent::new(4, 8) + Indent::new(8, 12);
        assert_eq!((indent.block_indent, indent.alignment), (12, 20));
        let indent = indent - Indent::new(4, 4);
        assert_eq!((indent.block_indent, indent.alignment), (8, 16));
    }

    #[test]
    fn indent_to_string() {
        use crate::formatter::config::Config;
        let mut config = Config::default();
        assert_eq!(
            "            ",
            Indent::new(4, 8).to_string(&Settings::new(&config))
        );
        config.hard_tabs = true;
        assert_eq!(
            "\t\t    ",
            Indent::new(8, 4).to_string(&Settings::new(&config))
        );
    }

    #[test]
    fn shape_visual_and_block_indent() {
        let shape = Shape::legacy(100, Indent::new(4, 8)).visual_indent(20);
        assert_eq!((shape.indent.block_indent, shape.indent.alignment), (4, 28));
        let shape = Shape::legacy(100, Indent::new(4, 0)).block_indent(20);
        assert_eq!((shape.indent.block_indent, shape.offset), (24, 0));
        let shape = Shape::legacy(100, Indent::new(4, 8)).block_indent(20);
        assert_eq!((shape.indent.alignment, shape.offset), (28, 28));
    }
}
