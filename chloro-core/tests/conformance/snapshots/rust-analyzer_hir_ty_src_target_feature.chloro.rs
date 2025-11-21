//! Stuff for handling `#[target_feature]` (needed for unsafe check).

use std::sync::LazyLock;

use hir_def::attr::Attrs;
use hir_def::tt;
use intern::{Symbol, sym};
use rustc_hash::{FxHashMap, FxHashSet};
#[derive(Debug, Default, Clone)]
pub struct TargetFeatures {
    pub(crate) enabled: FxHashSet<Symbol>,
}

impl TargetFeatures {
    pub fn from_attrs(attrs: &Attrs) -> Self {
        let mut result = TargetFeatures::from_attrs_no_implications(attrs);
        result.expand_implications();
        result
    }

    fn expand_implications(&mut self) {
        let all_implications = LazyLock::force(&TARGET_FEATURE_IMPLICATIONS);
        let mut queue = self.enabled.iter().cloned().collect::<Vec<_>>();
        while let Some(feature) = queue.pop() {
            if let Some(implications) = all_implications.get(&feature) {
                for implication in implications {
                    if self.enabled.insert(implication.clone()) {
                        queue.push(implication.clone());
                    }
                }
            }
        }
    }

    /// Retrieves the target features from the attributes, and does not expand the target features implied by them.
    pub(crate) fn from_attrs_no_implications(attrs: &Attrs) -> Self {
        let enabled = attrs
            .by_key(sym::target_feature)
            .tt_values()
            .filter_map(|tt| match tt.token_trees().flat_tokens() {
                [
                    tt::TokenTree::Leaf(tt::Leaf::Ident(enable_ident)),
                    tt::TokenTree::Leaf(tt::Leaf::Punct(tt::Punct { char: '=', .. })),
                    tt::TokenTree::Leaf(tt::Leaf::Literal(tt::Literal {
                        kind: tt::LitKind::Str,
                        symbol: features,
                        ..
                    })),
                ] if enable_ident.sym == sym::enable => Some(features),
                _ => None,
            })
            .flat_map(|features| features.as_str().split(',').map(Symbol::intern))
            .collect();
        Self { enabled }
    }
}

{
    let mut result = FxHashMap::<Symbol, FxHashSet<Symbol>>::default();
    for &(feature_str, implications) in TARGET_FEATURE_IMPLICATIONS_RAW {
            let feature = Symbol::intern(feature_str);
            let implications = implications.iter().copied().map(Symbol::intern);
            // Some target features appear in two archs, e.g. Arm and x86.
            // Sometimes they contain different implications, e.g. `aes`.
            // We should probably choose by the active arch, but for now just merge them.
            result.entry(feature).or_default().extend(implications);
        }
    let mut result = result
            .into_iter()
            .map(|(feature, implications)| (feature, Box::from_iter(implications)))
            .collect::<FxHashMap<_, _>>();
    result.shrink_to_fit();
    result;
}
// spellchecker:off
// Arm
// Aarch64
// x86
/*"fma4", */
// Hexagon
// PowerPC
// MIPS
// RISC-V
// WASM
// BPF
// CSKY
// LoongArch
// IBM Z
// SPARC
// m68k
