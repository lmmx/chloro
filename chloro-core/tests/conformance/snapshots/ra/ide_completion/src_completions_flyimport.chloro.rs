//! See [`import_on_the_fly`].

use hir::{ItemInNs, ModuleDef};
use ide_db::imports::{
    import_assets::{ImportAssets, LocatedImport},
    insert_use::ImportScope,
};
use itertools::Itertools;
use syntax::{AstNode, SyntaxNode, ast};

use crate::{
    Completions,
    config::AutoImportExclusionType,
    context::{
        CompletionContext, DotAccess, PathCompletionCtx, PathKind, PatternContext, Qualified,
        TypeLocation,
    },
    render::{RenderContext, render_resolution_with_import, render_resolution_with_import_pat},
};

pub(crate) fn import_on_the_fly_path(
    acc: &mut Completions,
    ctx: &CompletionContext<'_>,
    path_ctx: &PathCompletionCtx<'_>,
) -> Option<()> {
    if !ctx.config.enable_imports_on_the_fly {
        return None;
    }
    let qualified = match path_ctx {
        PathCompletionCtx {
            kind:
                PathKind::Expr { .. }
                | PathKind::Type { .. }
                | PathKind::Attr { .. }
                | PathKind::Derive { .. }
                | PathKind::Item { .. }
                | PathKind::Pat { .. },
            qualified,
            ..
        } => qualified,
        _ => return None,
    };
    let potential_import_name = import_name(ctx);
    let qualifier = match qualified {
        Qualified::With { path, .. } => Some(path.clone()),
        _ => None,
    };
    let import_assets = import_assets_for_path(ctx, &potential_import_name, qualifier.clone())?;

    import_on_the_fly(
        acc,
        ctx,
        path_ctx,
        import_assets,
        qualifier.map(|it| it.syntax().clone()).or_else(|| ctx.original_token.parent())?,
        potential_import_name,
    )
}

pub(crate) fn import_on_the_fly_pat(
    acc: &mut Completions,
    ctx: &CompletionContext<'_>,
    pattern_ctx: &PatternContext,
) -> Option<()> {
    if !ctx.config.enable_imports_on_the_fly {
        return None;
    }
    if let PatternContext { record_pat: Some(_), .. } = pattern_ctx {
        return None;
    }

    let potential_import_name = import_name(ctx);
    let import_assets = import_assets_for_path(ctx, &potential_import_name, None)?;

    import_on_the_fly_pat_(
        acc,
        ctx,
        pattern_ctx,
        import_assets,
        ctx.original_token.parent()?,
        potential_import_name,
    )
}

pub(crate) fn import_on_the_fly_dot(
    acc: &mut Completions,
    ctx: &CompletionContext<'_>,
    dot_access: &DotAccess<'_>,
) -> Option<()> {
    if !ctx.config.enable_imports_on_the_fly {
        return None;
    }
    let receiver = dot_access.receiver.as_ref()?;
    let ty = dot_access.receiver_ty.as_ref()?;
    let potential_import_name = import_name(ctx);
    let import_assets = ImportAssets::for_fuzzy_method_call(
        ctx.module,
        ty.original.clone(),
        potential_import_name.clone(),
        receiver.syntax().clone(),
    )?;

    import_on_the_fly_method(
        acc,
        ctx,
        dot_access,
        import_assets,
        receiver.syntax().clone(),
        potential_import_name,
    )
}

fn import_on_the_fly(
    acc: &mut Completions,
    ctx: &CompletionContext<'_>,
    path_ctx @ PathCompletionCtx { kind, .. }: &PathCompletionCtx<'_>,
    import_assets: ImportAssets<'_>,
    position: SyntaxNode,
    potential_import_name: String,
) -> Option<()> {
    let _p = tracing::info_span!("import_on_the_fly", ?potential_import_name).entered();

    ImportScope::find_insert_use_container(&position, &ctx.sema)?;

    let ns_filter = |import: &LocatedImport| {
        match (kind, import.original_item) {
            // Aren't handled in flyimport
            (PathKind::Vis { .. } | PathKind::Use, _) => false,
            // modules are always fair game
            (_, ItemInNs::Types(hir::ModuleDef::Module(_))) => true,
            // and so are macros(except for attributes)
            (
                PathKind::Expr { .. }
                | PathKind::Type { .. }
                | PathKind::Item { .. }
                | PathKind::Pat { .. },
                ItemInNs::Macros(mac),
            ) => mac.is_fn_like(ctx.db),
            (PathKind::Item { .. }, ..) => false,

            (PathKind::Expr { .. }, ItemInNs::Types(_) | ItemInNs::Values(_)) => true,

            (PathKind::Pat { .. }, ItemInNs::Types(_)) => true,
            (PathKind::Pat { .. }, ItemInNs::Values(def)) => {
                matches!(def, hir::ModuleDef::Const(_))
            }

            (PathKind::Type { location }, ItemInNs::Types(ty)) => {
                if matches!(location, TypeLocation::TypeBound) {
                    matches!(ty, ModuleDef::Trait(_))
                } else if matches!(location, TypeLocation::ImplTrait) {
                    matches!(ty, ModuleDef::Trait(_) | ModuleDef::Module(_))
                } else {
                    true
                }
            }
            (PathKind::Type { .. }, ItemInNs::Values(_)) => false,

            (PathKind::Attr { .. }, ItemInNs::Macros(mac)) => mac.is_attr(ctx.db),
            (PathKind::Attr { .. }, _) => false,

            (PathKind::Derive { existing_derives }, ItemInNs::Macros(mac)) => {
                mac.is_derive(ctx.db) && !existing_derives.contains(&mac)
            }
            (PathKind::Derive { .. }, _) => false,
        }
    };
    let user_input_lowercased = potential_import_name.to_lowercase();

    let import_cfg = ctx.config.import_path_config();

    import_assets
        .search_for_imports(&ctx.sema, import_cfg, ctx.config.insert_use.prefix_kind)
        .filter(ns_filter)
        .filter(|import| {
            let original_item = &import.original_item;
            !ctx.is_item_hidden(&import.item_to_import)
                && !ctx.is_item_hidden(original_item)
                && ctx.check_stability(original_item.attrs(ctx.db).as_deref())
        })
        .filter(|import| filter_excluded_flyimport(ctx, import))
        .sorted_by(|a, b| {
            let key = |import_path| {
                (
                    compute_fuzzy_completion_order_key(import_path, &user_input_lowercased),
                    import_path,
                )
            };
            key(&a.import_path).cmp(&key(&b.import_path))
        })
        .filter_map(|import| {
            render_resolution_with_import(RenderContext::new(ctx), path_ctx, import)
        })
        .map(|builder| builder.build(ctx.db))
        .for_each(|item| acc.add(item));
    Some(())
}

fn import_on_the_fly_pat_(
    acc: &mut Completions,
    ctx: &CompletionContext<'_>,
    pattern_ctx: &PatternContext,
    import_assets: ImportAssets<'_>,
    position: SyntaxNode,
    potential_import_name: String,
) -> Option<()> {
    let _p = tracing::info_span!("import_on_the_fly_pat_", ?potential_import_name).entered();

    ImportScope::find_insert_use_container(&position, &ctx.sema)?;

    let ns_filter = |import: &LocatedImport| match import.original_item {
        ItemInNs::Macros(mac) => mac.is_fn_like(ctx.db),
        ItemInNs::Types(_) => true,
        ItemInNs::Values(def) => matches!(def, hir::ModuleDef::Const(_)),
    };
    let user_input_lowercased = potential_import_name.to_lowercase();
    let cfg = ctx.config.import_path_config();

    import_assets
        .search_for_imports(&ctx.sema, cfg, ctx.config.insert_use.prefix_kind)
        .filter(ns_filter)
        .filter(|import| {
            let original_item = &import.original_item;
            !ctx.is_item_hidden(&import.item_to_import)
                && !ctx.is_item_hidden(original_item)
                && ctx.check_stability(original_item.attrs(ctx.db).as_deref())
        })
        .sorted_by(|a, b| {
            let key = |import_path| {
                (
                    compute_fuzzy_completion_order_key(import_path, &user_input_lowercased),
                    import_path,
                )
            };
            key(&a.import_path).cmp(&key(&b.import_path))
        })
        .filter_map(|import| {
            render_resolution_with_import_pat(RenderContext::new(ctx), pattern_ctx, import)
        })
        .map(|builder| builder.build(ctx.db))
        .for_each(|item| acc.add(item));
    Some(())
}

fn import_on_the_fly_method(
    acc: &mut Completions,
    ctx: &CompletionContext<'_>,
    dot_access: &DotAccess<'_>,
    import_assets: ImportAssets<'_>,
    position: SyntaxNode,
    potential_import_name: String,
) -> Option<()> {
    let _p = tracing::info_span!("import_on_the_fly_method", ?potential_import_name).entered();

    ImportScope::find_insert_use_container(&position, &ctx.sema)?;

    let user_input_lowercased = potential_import_name.to_lowercase();

    let cfg = ctx.config.import_path_config();

    import_assets
        .search_for_imports(&ctx.sema, cfg, ctx.config.insert_use.prefix_kind)
        .filter(|import| {
            !ctx.is_item_hidden(&import.item_to_import)
                && !ctx.is_item_hidden(&import.original_item)
        })
        .filter(|import| filter_excluded_flyimport(ctx, import))
        .sorted_by(|a, b| {
            let key = |import_path| {
                (
                    compute_fuzzy_completion_order_key(import_path, &user_input_lowercased),
                    import_path,
                )
            };
            key(&a.import_path).cmp(&key(&b.import_path))
        })
        .for_each(|import| {
            if let ItemInNs::Values(hir::ModuleDef::Function(f)) = import.original_item {
                acc.add_method_with_import(ctx, dot_access, f, import);
            }
        });
    Some(())
}

fn filter_excluded_flyimport(ctx: &CompletionContext<'_>, import: &LocatedImport) -> bool {
    let def = import.item_to_import.into_module_def();
    let is_exclude_flyimport = ctx.exclude_flyimport.get(&def).copied();

    if matches!(is_exclude_flyimport, Some(AutoImportExclusionType::Always))
        || !import.complete_in_flyimport.0
    {
        return false;
    }
    let method_imported = import.item_to_import != import.original_item;
    if method_imported
        && (is_exclude_flyimport.is_some()
            || ctx.exclude_flyimport.contains_key(&import.original_item.into_module_def()))
    {
        // If this is a method, exclude it either if it was excluded itself (which may not be caught above,
        // because `item_to_import` is the trait), or if its trait was excluded. We don't need to check
        // the attributes here, since they pass from trait to methods on import map construction.
        return false;
    }
    true
}

fn import_name(ctx: &CompletionContext<'_>) -> String {
    let token_kind = ctx.token.kind();

    if token_kind.is_any_identifier() {
        ctx.token.to_string()
    } else {
        String::new()
    }
}

fn import_assets_for_path<'db>(
    ctx: &CompletionContext<'db>,
    potential_import_name: &str,
    qualifier: Option<ast::Path>,
) -> Option<ImportAssets<'db>> {
    let _p =
        tracing::info_span!("import_assets_for_path", ?potential_import_name, ?qualifier).entered();

    let fuzzy_name_length = potential_import_name.len();
    let mut assets_for_path = ImportAssets::for_fuzzy_path(
        ctx.module,
        qualifier,
        potential_import_name.to_owned(),
        &ctx.sema,
        ctx.token.parent()?,
    )?;
    if fuzzy_name_length == 0 {
        // nothing matches the empty string exactly, but we still compute assoc items in this case
        assets_for_path.path_fuzzy_name_to_exact();
    } else if fuzzy_name_length < 3 {
        cov_mark::hit!(flyimport_prefix_on_short_path);
        assets_for_path.path_fuzzy_name_to_prefix();
    }
    Some(assets_for_path)
}

fn compute_fuzzy_completion_order_key(
    proposed_mod_path: &hir::ModPath,
    user_input_lowercased: &str,
) -> usize {
    cov_mark::hit!(certain_fuzzy_order_test);
    let import_name = match proposed_mod_path.segments().last() {
        // FIXME: nasty alloc, this is a hot path!
        Some(name) => name.as_str().to_ascii_lowercase(),
        None => return usize::MAX,
    };
    match import_name.match_indices(user_input_lowercased).next() {
        Some((first_matching_index, _)) => first_matching_index,
        None => usize::MAX,
    }
}
