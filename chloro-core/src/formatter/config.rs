//! Configuration for chloro formatting behavior.
//!
//! [`Config`] mirrors rustfmt's option table (rustfmt 1.9, `rustfmt --print-config default`).
//! Every *stable* rustfmt option is a public field with the same name and default value, so a
//! `rustfmt.toml` translates field-for-field. rustfmt's *unstable* options are not
//! configurable: chloro always behaves as rustfmt does with those options at their defaults,
//! and the crate-private accessors at the bottom of this file name each one so that the
//! formatting code reads like the rustfmt code it was ported from.
//!
//! The width heuristics (`fn_call_width`, `chain_width`, ...) follow rustfmt's resolution
//! rules: each is derived from `max_width` and `use_small_heuristics` unless explicitly set,
//! and an explicit value larger than `max_width` is clamped to `max_width`.

use super::lists::{ListTactic, SeparatorPlace, SeparatorTactic};

/// Maximum line width before we wrap types onto new lines.
pub(crate) static MAX_WIDTH: usize = 100;

/// Line endings written to the output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NewlineStyle {
    /// Use the line ending of the first newline in the input (`\n` if there is none).
    #[default]
    Auto,
    /// `\r\n` on Windows hosts, `\n` elsewhere.
    Native,
    /// Always `\n`.
    Unix,
    /// Always `\r\n`.
    Windows,
}

/// How the width heuristics are derived from `max_width`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Heuristics {
    /// Scale rustfmt's default heuristic widths by `max_width / 100` (never below 1.0).
    #[default]
    Default,
    /// Every heuristic width equals `max_width`.
    Max,
    /// Disable the heuristics: lists, chains and struct literals are bounded by `max_width`
    /// only, and struct literals, struct variants and single-line `if`/`let else` never stay
    /// on one line.
    Off,
}

/// Layout of the parameters of a function signature.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    /// Fit as many parameters per line as possible.
    Compressed,
    /// All parameters on one line, or one parameter per line.
    #[default]
    Tall,
    /// Always one parameter per line.
    Vertical,
}

impl Density {
    pub(crate) fn to_list_tactic(self, len: usize) -> ListTactic {
        match self {
            Density::Compressed => ListTactic::Mixed,
            Density::Tall => ListTactic::HorizontalVertical,
            Density::Vertical if len == 1 => ListTactic::Horizontal,
            Density::Vertical => ListTactic::Vertical,
        }
    }
}

/// Whether match arms begin with a leading `|`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MatchArmLeadingPipe {
    /// Always add a leading pipe.
    Always,
    /// Remove leading pipes.
    #[default]
    Never,
    /// Keep leading pipes as written.
    Preserve,
}

/// The edition of the Rust Style Guide applied by the formatter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StyleEdition {
    /// Style Guide as of Rust 2015.
    Edition2015,
    /// Style Guide as of Rust 2018 (identical to 2015).
    Edition2018,
    /// Style Guide as of Rust 2021 (identical to 2015).
    Edition2021,
    /// Style Guide as of Rust 2024.
    #[default]
    Edition2024,
}

impl Edition {
    /// The parser edition passed to `ra_ap_syntax`.
    pub(crate) fn to_ra(self) -> ra_ap_syntax::Edition {
        match self {
            Edition::Edition2015 => ra_ap_syntax::Edition::Edition2015,
            Edition::Edition2018 => ra_ap_syntax::Edition::Edition2018,
            Edition::Edition2021 => ra_ap_syntax::Edition::Edition2021,
            Edition::Edition2024 => ra_ap_syntax::Edition::Edition2024,
        }
    }
}

/// The Rust language edition used to parse the source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Edition {
    /// Rust 2015.
    Edition2015,
    /// Rust 2018.
    Edition2018,
    /// Rust 2021.
    Edition2021,
    /// Rust 2024.
    #[default]
    Edition2024,
}

impl From<Edition> for StyleEdition {
    fn from(edition: Edition) -> Self {
        match edition {
            Edition::Edition2015 => StyleEdition::Edition2015,
            Edition::Edition2018 => StyleEdition::Edition2018,
            Edition::Edition2021 => StyleEdition::Edition2021,
            Edition::Edition2024 => StyleEdition::Edition2024,
        }
    }
}

/// Formatting options, named and defaulted as rustfmt's stable options.
///
/// The defaults reproduce `rustfmt --edition 2024` with no `rustfmt.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Maximum width of each line.
    pub max_width: usize,
    /// Use tab characters for indentation, spaces for alignment.
    pub hard_tabs: bool,
    /// Number of spaces per tab.
    pub tab_spaces: usize,
    /// Unix or Windows line endings.
    pub newline_style: NewlineStyle,
    /// How the width heuristics below are derived when not set explicitly.
    pub use_small_heuristics: Heuristics,
    /// Maximum width of the args of a function call before falling back to vertical formatting.
    pub fn_call_width: Option<usize>,
    /// Maximum width of the args of a function-like attribute before falling back to vertical
    /// formatting.
    pub attr_fn_like_width: Option<usize>,
    /// Maximum width in the body of a struct literal before falling back to vertical formatting.
    pub struct_lit_width: Option<usize>,
    /// Maximum width in the body of a struct variant before falling back to vertical formatting.
    pub struct_variant_width: Option<usize>,
    /// Maximum width of an array literal before falling back to vertical formatting.
    pub array_width: Option<usize>,
    /// Maximum length of a chain to fit on a single line.
    pub chain_width: Option<usize>,
    /// Maximum line length for single line if-else expressions (0 always breaks).
    pub single_line_if_else_max_width: Option<usize>,
    /// Maximum line length for single line let-else statements (0 always breaks).
    pub single_line_let_else_max_width: Option<usize>,
    /// Reorder import and extern crate statements alphabetically.
    pub reorder_imports: bool,
    /// Reorder module statements alphabetically in group.
    pub reorder_modules: bool,
    /// Remove nested parens.
    pub remove_nested_parens: bool,
    /// Width threshold for an array element to be considered short.
    pub short_array_element_width_threshold: usize,
    /// Determines whether leading pipes are emitted on match arms.
    pub match_arm_leading_pipes: MatchArmLeadingPipe,
    /// Control the layout of parameters in function signatures.
    pub fn_params_layout: Density,
    /// Put a trailing comma after a block based match arm.
    pub match_block_trailing_comma: bool,
    /// The edition of the parser.
    pub edition: Edition,
    /// The edition of the Style Guide. `None` follows `edition`, as in rustfmt, where an
    /// unset `style_edition` is derived from `edition`.
    pub style_edition: Option<StyleEdition>,
    /// Merge multiple `#[derive(...)]` into a single one.
    pub merge_derives: bool,
    /// Replace uses of the `try!` macro by the `?` shorthand.
    pub use_try_shorthand: bool,
    /// Use field initialization shorthand if possible.
    pub use_field_init_shorthand: bool,
    /// Always print the abi for extern items.
    pub force_explicit_abi: bool,
    /// Don't reformat anything.
    pub disable_all_formatting: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_width: MAX_WIDTH,
            hard_tabs: false,
            tab_spaces: 4,
            newline_style: NewlineStyle::Auto,
            use_small_heuristics: Heuristics::Default,
            fn_call_width: None,
            attr_fn_like_width: None,
            struct_lit_width: None,
            struct_variant_width: None,
            array_width: None,
            chain_width: None,
            single_line_if_else_max_width: None,
            single_line_let_else_max_width: None,
            reorder_imports: true,
            reorder_modules: true,
            remove_nested_parens: true,
            short_array_element_width_threshold: 10,
            match_arm_leading_pipes: MatchArmLeadingPipe::Never,
            fn_params_layout: Density::Tall,
            match_block_trailing_comma: false,
            edition: Edition::Edition2024,
            style_edition: None,
            merge_derives: true,
            use_try_shorthand: false,
            use_field_init_shorthand: false,
            force_explicit_abi: true,
            disable_all_formatting: false,
        }
    }
}

/// The eight width heuristics after resolution against `max_width`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WidthHeuristics {
    pub(crate) fn_call_width: usize,
    pub(crate) attr_fn_like_width: usize,
    pub(crate) struct_lit_width: usize,
    pub(crate) struct_variant_width: usize,
    pub(crate) array_width: usize,
    pub(crate) chain_width: usize,
    pub(crate) single_line_if_else_max_width: usize,
    pub(crate) single_line_let_else_max_width: usize,
}

impl WidthHeuristics {
    fn null() -> Self {
        WidthHeuristics {
            fn_call_width: usize::MAX,
            attr_fn_like_width: usize::MAX,
            struct_lit_width: 0,
            struct_variant_width: 0,
            array_width: usize::MAX,
            chain_width: usize::MAX,
            single_line_if_else_max_width: 0,
            single_line_let_else_max_width: 0,
        }
    }

    fn set(max_width: usize) -> Self {
        WidthHeuristics {
            fn_call_width: max_width,
            attr_fn_like_width: max_width,
            struct_lit_width: max_width,
            struct_variant_width: max_width,
            array_width: max_width,
            chain_width: max_width,
            single_line_if_else_max_width: max_width,
            single_line_let_else_max_width: max_width,
        }
    }

    fn scaled(max_width: usize) -> Self {
        const DEFAULT_MAX_WIDTH: usize = 100;
        let ratio = if max_width > DEFAULT_MAX_WIDTH {
            let ratio = max_width as f32 / DEFAULT_MAX_WIDTH as f32;
            // round to the closest 0.1
            (ratio * 10.0).round() / 10.0
        } else {
            1.0
        };
        let scale = |w: f32| (w * ratio).round() as usize;
        WidthHeuristics {
            fn_call_width: scale(60.0),
            attr_fn_like_width: scale(70.0),
            struct_lit_width: scale(18.0),
            struct_variant_width: scale(35.0),
            array_width: scale(60.0),
            chain_width: scale(60.0),
            single_line_if_else_max_width: scale(50.0),
            single_line_let_else_max_width: scale(50.0),
        }
    }
}

impl Config {
    /// Resolve the width heuristics against `max_width` and `use_small_heuristics`.
    pub(crate) fn width_heuristics(&self) -> WidthHeuristics {
        let base = match self.use_small_heuristics {
            Heuristics::Default => WidthHeuristics::scaled(self.max_width),
            Heuristics::Max => WidthHeuristics::set(self.max_width),
            Heuristics::Off => WidthHeuristics::null(),
        };
        let pick = |set: Option<usize>, heuristic: usize| match set {
            Some(v) => v.min(self.max_width),
            None => heuristic,
        };
        WidthHeuristics {
            fn_call_width: pick(self.fn_call_width, base.fn_call_width),
            attr_fn_like_width: pick(self.attr_fn_like_width, base.attr_fn_like_width),
            struct_lit_width: pick(self.struct_lit_width, base.struct_lit_width),
            struct_variant_width: pick(self.struct_variant_width, base.struct_variant_width),
            array_width: pick(self.array_width, base.array_width),
            chain_width: pick(self.chain_width, base.chain_width),
            single_line_if_else_max_width: pick(
                self.single_line_if_else_max_width,
                base.single_line_if_else_max_width,
            ),
            single_line_let_else_max_width: pick(
                self.single_line_let_else_max_width,
                base.single_line_let_else_max_width,
            ),
        }
    }

    /// Parse one `key = value` setting using rustfmt's option names and value spellings,
    /// as accepted by `rustfmt --config key=value` and `rustfmt.toml`.
    ///
    /// Unstable rustfmt options are rejected unless set to their default value.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let value = value.trim().trim_matches('"');
        let bool_ = || match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!("`{key}` expects true or false, got `{value}`")),
        };
        let usize_ = || {
            value
                .parse::<usize>()
                .map_err(|_| format!("`{key}` expects an unsigned integer, got `{value}`"))
        };
        let bad = || format!("unsupported value `{value}` for `{key}`");
        match key {
            "max_width" => self.max_width = usize_()?,
            "hard_tabs" => self.hard_tabs = bool_()?,
            "tab_spaces" => self.tab_spaces = usize_()?,
            "newline_style" => {
                self.newline_style = match value {
                    "Auto" => NewlineStyle::Auto,
                    "Native" => NewlineStyle::Native,
                    "Unix" => NewlineStyle::Unix,
                    "Windows" => NewlineStyle::Windows,
                    _ => return Err(bad()),
                }
            }
            "use_small_heuristics" => {
                self.use_small_heuristics = match value {
                    "Default" => Heuristics::Default,
                    "Max" => Heuristics::Max,
                    "Off" => Heuristics::Off,
                    _ => return Err(bad()),
                }
            }
            "fn_call_width" => self.fn_call_width = Some(usize_()?),
            "attr_fn_like_width" => self.attr_fn_like_width = Some(usize_()?),
            "struct_lit_width" => self.struct_lit_width = Some(usize_()?),
            "struct_variant_width" => self.struct_variant_width = Some(usize_()?),
            "array_width" => self.array_width = Some(usize_()?),
            "chain_width" => self.chain_width = Some(usize_()?),
            "single_line_if_else_max_width" => self.single_line_if_else_max_width = Some(usize_()?),
            "single_line_let_else_max_width" => {
                self.single_line_let_else_max_width = Some(usize_()?)
            }
            "reorder_imports" => self.reorder_imports = bool_()?,
            "reorder_modules" => self.reorder_modules = bool_()?,
            "remove_nested_parens" => self.remove_nested_parens = bool_()?,
            "short_array_element_width_threshold" => {
                self.short_array_element_width_threshold = usize_()?
            }
            "match_arm_leading_pipes" => {
                self.match_arm_leading_pipes = match value {
                    "Always" => MatchArmLeadingPipe::Always,
                    "Never" => MatchArmLeadingPipe::Never,
                    "Preserve" => MatchArmLeadingPipe::Preserve,
                    _ => return Err(bad()),
                }
            }
            "fn_params_layout" | "fn_args_layout" => {
                self.fn_params_layout = match value {
                    "Compressed" => Density::Compressed,
                    "Tall" => Density::Tall,
                    "Vertical" => Density::Vertical,
                    _ => return Err(bad()),
                }
            }
            "match_block_trailing_comma" => self.match_block_trailing_comma = bool_()?,
            "edition" | "style_edition" => {
                let (edition, style) = match value {
                    "2015" => (Edition::Edition2015, StyleEdition::Edition2015),
                    "2018" => (Edition::Edition2018, StyleEdition::Edition2018),
                    "2021" => (Edition::Edition2021, StyleEdition::Edition2021),
                    "2024" => (Edition::Edition2024, StyleEdition::Edition2024),
                    _ => return Err(bad()),
                };
                if key == "edition" {
                    self.edition = edition;
                } else {
                    self.style_edition = Some(style);
                }
            }
            "merge_derives" => self.merge_derives = bool_()?,
            "use_try_shorthand" => self.use_try_shorthand = bool_()?,
            "use_field_init_shorthand" => self.use_field_init_shorthand = bool_()?,
            "force_explicit_abi" => self.force_explicit_abi = bool_()?,
            "disable_all_formatting" => self.disable_all_formatting = bool_()?,
            _ => match UNSTABLE_DEFAULTS.iter().find(|(k, _)| *k == key) {
                Some((_, default)) if *default == value => {}
                Some((_, default)) => {
                    return Err(format!(
                        "`{key}` is an unstable rustfmt option; only its default `{default}` is supported"
                    ));
                }
                None => return Err(format!("unknown option `{key}`")),
            },
        }
        Ok(())
    }
}

/// rustfmt's unstable options and the default value chloro always applies.
const UNSTABLE_DEFAULTS: &[(&str, &str)] = &[
    ("indent_style", "Block"),
    ("wrap_comments", "false"),
    ("format_code_in_doc_comments", "false"),
    ("doc_comment_code_block_width", "100"),
    ("comment_width", "80"),
    ("normalize_comments", "false"),
    ("normalize_doc_attributes", "false"),
    ("format_strings", "false"),
    ("format_macro_matchers", "false"),
    ("format_macro_bodies", "true"),
    ("skip_macro_invocations", "[]"),
    ("hex_literal_case", "Preserve"),
    ("float_literal_trailing_zero", "Preserve"),
    ("empty_item_single_line", "true"),
    ("struct_lit_single_line", "true"),
    ("fn_single_line", "false"),
    ("where_single_line", "false"),
    ("imports_indent", "Block"),
    ("imports_layout", "Mixed"),
    ("imports_granularity", "Preserve"),
    ("group_imports", "Preserve"),
    ("reorder_impl_items", "false"),
    ("type_punctuation_density", "Wide"),
    ("space_before_colon", "false"),
    ("space_after_colon", "true"),
    ("spaces_around_ranges", "false"),
    ("binop_separator", "Front"),
    ("combine_control_expr", "true"),
    ("overflow_delimited_expr", "false"),
    ("struct_field_align_threshold", "0"),
    ("enum_discrim_align_threshold", "0"),
    ("match_arm_blocks", "true"),
    ("match_arm_indent", "true"),
    ("force_multiline_blocks", "false"),
    ("brace_style", "SameLineWhere"),
    ("control_brace_style", "AlwaysSameLine"),
    ("trailing_semicolon", "true"),
    ("trailing_comma", "Vertical"),
    ("blank_lines_upper_bound", "1"),
    ("blank_lines_lower_bound", "0"),
    ("inline_attribute_width", "0"),
    ("format_generated_files", "true"),
    ("condense_wildcard_suffixes", "false"),
];

/// The configuration as seen by the formatting code: a [`Config`] with its width heuristics
/// resolved once, plus accessors named after rustfmt's options.
///
/// Stable options read the public fields; unstable rustfmt options return the value rustfmt
/// uses by default. Keeping the unstable ones as named accessors documents, at each use site,
/// which rustfmt behaviour is being reproduced.
#[derive(Clone, Debug)]
pub(crate) struct Settings {
    config: Config,
    heuristics: WidthHeuristics,
}

impl Settings {
    pub(crate) fn new(config: &Config) -> Self {
        Settings {
            heuristics: config.width_heuristics(),
            config: config.clone(),
        }
    }
}

impl Settings {
    /// The same settings with another `max_width`, re-deriving the width heuristics as
    /// rustfmt's `config.set().max_width(..)` does.
    pub(crate) fn with_max_width(&self, max_width: usize) -> Settings {
        let mut config = self.config.clone();
        config.max_width = max_width;
        Settings::new(&config)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings::new(&Config::default())
    }
}

impl Settings {
    pub(crate) fn max_width(&self) -> usize {
        self.config.max_width
    }
    pub(crate) fn hard_tabs(&self) -> bool {
        self.config.hard_tabs
    }
    pub(crate) fn tab_spaces(&self) -> usize {
        self.config.tab_spaces
    }
    pub(crate) fn newline_style(&self) -> NewlineStyle {
        self.config.newline_style
    }
    pub(crate) fn fn_call_width(&self) -> usize {
        self.heuristics.fn_call_width
    }
    pub(crate) fn attr_fn_like_width(&self) -> usize {
        self.heuristics.attr_fn_like_width
    }
    pub(crate) fn struct_lit_width(&self) -> usize {
        self.heuristics.struct_lit_width
    }
    pub(crate) fn struct_variant_width(&self) -> usize {
        self.heuristics.struct_variant_width
    }
    pub(crate) fn array_width(&self) -> usize {
        self.heuristics.array_width
    }
    pub(crate) fn chain_width(&self) -> usize {
        self.heuristics.chain_width
    }
    pub(crate) fn single_line_if_else_max_width(&self) -> usize {
        self.heuristics.single_line_if_else_max_width
    }
    pub(crate) fn single_line_let_else_max_width(&self) -> usize {
        self.heuristics.single_line_let_else_max_width
    }
    pub(crate) fn reorder_imports(&self) -> bool {
        self.config.reorder_imports
    }
    pub(crate) fn reorder_modules(&self) -> bool {
        self.config.reorder_modules
    }
    pub(crate) fn remove_nested_parens(&self) -> bool {
        self.config.remove_nested_parens
    }
    pub(crate) fn short_array_element_width_threshold(&self) -> usize {
        self.config.short_array_element_width_threshold
    }
    pub(crate) fn match_arm_leading_pipes(&self) -> MatchArmLeadingPipe {
        self.config.match_arm_leading_pipes
    }
    pub(crate) fn fn_params_layout(&self) -> Density {
        self.config.fn_params_layout
    }
    pub(crate) fn match_block_trailing_comma(&self) -> bool {
        self.config.match_block_trailing_comma
    }
    pub(crate) fn edition(&self) -> Edition {
        self.config.edition
    }
    pub(crate) fn style_edition(&self) -> StyleEdition {
        // rustfmt's `Config::default_for_possible_style_edition`: an explicit
        // `style_edition` wins, otherwise the Style Guide edition is the language edition.
        self.config
            .style_edition
            .unwrap_or(self.config.edition.into())
    }
    pub(crate) fn merge_derives(&self) -> bool {
        self.config.merge_derives
    }
    pub(crate) fn use_try_shorthand(&self) -> bool {
        self.config.use_try_shorthand
    }
    pub(crate) fn use_field_init_shorthand(&self) -> bool {
        self.config.use_field_init_shorthand
    }
    pub(crate) fn force_explicit_abi(&self) -> bool {
        self.config.force_explicit_abi
    }
    pub(crate) fn disable_all_formatting(&self) -> bool {
        self.config.disable_all_formatting
    }

    // Unstable options, fixed at rustfmt's defaults.
    pub(crate) fn comment_width(&self) -> usize {
        80
    }
    pub(crate) fn normalize_comments(&self) -> bool {
        false
    }
    pub(crate) fn wrap_comments(&self) -> bool {
        false
    }
    pub(crate) fn format_strings(&self) -> bool {
        false
    }
    pub(crate) fn format_macro_matchers(&self) -> bool {
        false
    }
    pub(crate) fn format_macro_bodies(&self) -> bool {
        true
    }
    pub(crate) fn empty_item_single_line(&self) -> bool {
        true
    }
    pub(crate) fn struct_lit_single_line(&self) -> bool {
        true
    }
    pub(crate) fn fn_single_line(&self) -> bool {
        false
    }
    pub(crate) fn where_single_line(&self) -> bool {
        false
    }
    pub(crate) fn combine_control_expr(&self) -> bool {
        true
    }
    pub(crate) fn overflow_delimited_expr(&self) -> bool {
        false
    }
    pub(crate) fn struct_field_align_threshold(&self) -> usize {
        0
    }
    pub(crate) fn enum_discrim_align_threshold(&self) -> usize {
        0
    }
    pub(crate) fn match_arm_blocks(&self) -> bool {
        true
    }
    pub(crate) fn match_arm_indent(&self) -> bool {
        true
    }
    pub(crate) fn force_multiline_blocks(&self) -> bool {
        false
    }
    pub(crate) fn trailing_semicolon(&self) -> bool {
        true
    }
    pub(crate) fn blank_lines_upper_bound(&self) -> usize {
        1
    }
    pub(crate) fn blank_lines_lower_bound(&self) -> usize {
        0
    }
    pub(crate) fn inline_attribute_width(&self) -> usize {
        0
    }
    pub(crate) fn spaces_around_ranges(&self) -> bool {
        false
    }
    pub(crate) fn space_before_colon(&self) -> bool {
        false
    }
    pub(crate) fn space_after_colon(&self) -> bool {
        true
    }
    pub(crate) fn condense_wildcard_suffixes(&self) -> bool {
        false
    }
    pub(crate) fn reorder_impl_items(&self) -> bool {
        false
    }
    pub(crate) fn normalize_doc_attributes(&self) -> bool {
        false
    }
    pub(crate) fn trailing_comma(&self) -> SeparatorTactic {
        SeparatorTactic::Vertical
    }
    pub(crate) fn binop_separator(&self) -> SeparatorPlace {
        SeparatorPlace::Front
    }
    pub(crate) fn imports_layout(&self) -> ListTactic {
        ListTactic::Mixed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_heuristics_match_rustfmt() {
        let h = Config::default().width_heuristics();
        assert_eq!(
            (h.fn_call_width, h.attr_fn_like_width, h.struct_lit_width),
            (60, 70, 18)
        );
        assert_eq!(
            (h.struct_variant_width, h.array_width, h.chain_width),
            (35, 60, 60)
        );
        assert_eq!(
            (
                h.single_line_if_else_max_width,
                h.single_line_let_else_max_width
            ),
            (50, 50)
        );
    }

    #[test]
    fn heuristics_scale_above_default_width_only() {
        let mut config = Config {
            max_width: 80,
            ..Config::default()
        };
        assert_eq!(config.width_heuristics().fn_call_width, 60);
        config.max_width = 120;
        assert_eq!(config.width_heuristics().fn_call_width, 72);
    }

    #[test]
    fn explicit_width_is_clamped_to_max_width() {
        let mut config = Config::default();
        config.set("chain_width", "500").unwrap();
        assert_eq!(config.width_heuristics().chain_width, 100);
    }

    #[test]
    fn unstable_options_accept_only_their_default() {
        let mut config = Config::default();
        assert!(config.set("wrap_comments", "true").is_err());
        assert!(config.set("wrap_comments", "false").is_ok());
        assert!(config.set("no_such_option", "1").is_err());
    }

    #[test]
    fn style_edition_follows_edition_unless_set() {
        let mut config = Config::default();
        config.set("edition", "2021").unwrap();
        assert_eq!(
            Settings::new(&config).style_edition(),
            StyleEdition::Edition2021
        );
        config.set("style_edition", "2024").unwrap();
        config.set("edition", "2018").unwrap();
        assert_eq!(
            Settings::new(&config).style_edition(),
            StyleEdition::Edition2024
        );
    }
}
