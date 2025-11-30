use ra_ap_syntax::ast::{AstNode, AstToken, Comment, Use};

use crate::formatter::node::format_use;
use crate::formatter::node::useitem::sort;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportGroup {
    Std,
    External,
    Internal,
}

pub fn classify_import(use_: &Use) -> (ImportGroup, String) {
    let path = if let Some(tree) = use_.use_tree() {
        tree.syntax().text().to_string()
    } else {
        return (ImportGroup::External, String::new());
    };

    let group =
        if path.starts_with("std::") || path.starts_with("core::") || path.starts_with("alloc::") {
            ImportGroup::Std
        } else if path.starts_with("crate::")
            || path.starts_with("self::")
            || path.starts_with("super::")
        {
            ImportGroup::Internal
        } else {
            ImportGroup::External
        };

    (group, path)
}

pub fn sort_and_format_imports(
    use_items: &[(Vec<Comment>, Use, Vec<Comment>)],
    buf: &mut String,
    indent: usize,
) {
    // Sort the use statements by group and path
    let mut sorted_uses = use_items.to_vec();
    sorted_uses.sort_by(|(_, a, _), (_, b, _)| {
        let (group_a, path_a) = classify_import(a);
        let (group_b, path_b) = classify_import(b);

        // Sort by group first, then by path using rustfmt's sorting rules
        group_a
            .cmp(&group_b)
            .then_with(|| sort::sort_key(&path_a).cmp(&sort::sort_key(&path_b)))
    });

    let mut last_group = None;
    for (before_comments, use_, trailing_comments) in sorted_uses {
        // Check if we need a blank line between groups
        let (group, _) = classify_import(&use_);
        if let Some(last) = last_group
            && last != group
        {
            buf.push('\n');
        }
        last_group = Some(group);

        // Output preceding comments
        for comment in &before_comments {
            buf.push_str(comment.text());
            buf.push('\n');
        }

        // Output the use statement
        format_use(use_.syntax(), buf, indent);

        // Output trailing comments
        for comment in &trailing_comments {
            buf.push_str(comment.text());
            buf.push('\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ra_ap_syntax::{Edition, SourceFile};

    #[test]
    fn test_sort_imports_simple() {
        let input = "use a::{A, Ab, Ac, a, ab, ac};";

        let parse = SourceFile::parse(input, Edition::CURRENT);
        let root = parse.syntax_node();

        let mut use_items = Vec::new();
        for item in root.descendants() {
            if let Some(use_) = Use::cast(item) {
                use_items.push((Vec::new(), use_, Vec::new()));
            }
        }

        let mut buf = String::new();
        sort_and_format_imports(&use_items, &mut buf, 0);

        assert_snapshot!(buf, @"use a::{a, ab, ac, Ab, Ac, A};");
    }

    #[test]
    fn test_sort_imports_with_line_width() {
        // This test was lexicographically sorted at line width 100
        let input = r#"use a::{
    A, AA, AAA, AAAA, AAAZ, AAAa, AAAz, AAZ, AAZA, AAZZ,
    AAZa, AAZz, AAa, AAaA, AAaZ, AAaa, AAaz, AAz, AAzA,
    AAzZ, AAza, AAzz, AZ, AZA, AZAA, AZAZ, AZAa, AZAz, AZZ,
    AZZA, AZZZ, AZZa, AZZz, AZa, AZaA, AZaZ, AZaa, AZaz,
    AZz, AZzA, AZzZ, AZza, AZzz, Aa, AaA, AaAA, AaAZ, AaAa,
    AaAz, AaZ, AaZA, AaZZ, AaZa, AaZz, Aaa, AaaA, AaaZ,
    Aaaa, Aaaz, Aaz, AazA, AazZ, Aaza, Aazz, Az, AzA, AzAA,
    AzAZ, AzAa, AzAz, AzZ, AzZA, AzZZ, AzZa, AzZz, Aza,
    AzaA, AzaZ, Azaa, Azaz, Azz, AzzA, AzzZ, Azza, Azzz, Z,
    ZA, ZAA, ZAAA, ZAAZ, ZAAa, ZAAz, ZAZ, ZAZA, ZAZZ, ZAZa,
    ZAZz, ZAa, ZAaA, ZAaZ, ZAaa, ZAaz, ZAz, ZAzA, ZAzZ,
    ZAza, ZAzz, ZZ, ZZA, ZZAA, ZZAZ, ZZAa, ZZAz, ZZZ, ZZZA,
    ZZZZ, ZZZa, ZZZz, ZZa, ZZaA, ZZaZ, ZZaa, ZZaz, ZZz,
    ZZzA, ZZzZ, ZZza, ZZzz, Za, ZaA, ZaAA, ZaAZ, ZaAa, ZaAz,
    ZaZ, ZaZA, ZaZZ, ZaZa, ZaZz, Zaa, ZaaA, ZaaZ, Zaaa,
    Zaaz, Zaz, ZazA, ZazZ, Zaza, Zazz, Zz, ZzA, ZzAA, ZzAZ,
    ZzAa, ZzAz, ZzZ, ZzZA, ZzZZ, ZzZa, ZzZz, Zza, ZzaA,
    ZzaZ, Zzaa, Zzaz, Zzz, ZzzA, ZzzZ, Zzza, Zzzz, a, aA,
    aAA, aAAA, aAAZ, aAAa, aAAz, aAZ, aAZA, aAZZ, aAZa,
    aAZz, aAa, aAaA, aAaZ, aAaa, aAaz, aAz, aAzA, aAzZ,
    aAza, aAzz, aZ, aZA, aZAA, aZAZ, aZAa, aZAz, aZZ, aZZA,
    aZZZ, aZZa, aZZz, aZa, aZaA, aZaZ, aZaa, aZaz, aZz,
    aZzA, aZzZ, aZza, aZzz, aa, aaA, aaAA, aaAZ, aaAa, aaAz,
    aaZ, aaZA, aaZZ, aaZa, aaZz, aaa, aaaA, aaaZ, aaaa,
    aaaz, aaz, aazA, aazZ, aaza, aazz, az, azA, azAA, azAZ,
    azAa, azAz, azZ, azZA, azZZ, azZa, azZz, aza, azaA,
    azaZ, azaa, azaz, azz, azzA, azzZ, azza, azzz, z, zA,
    zAA, zAAA, zAAZ, zAAa, zAAz, zAZ, zAZA, zAZZ, zAZa,
    zAZz, zAa, zAaA, zAaZ, zAaa, zAaz, zAz, zAzA, zAzZ,
    zAza, zAzz, zZ, zZA, zZAA, zZAZ, zZAa, zZAz, zZZ, zZZA,
    zZZZ, zZZa, zZZz, zZa, zZaA, zZaZ, zZaa, zZaz, zZz,
    zZzA, zZzZ, zZza, zZzz, za, zaA, zaAA, zaAZ, zaAa, zaAz,
    zaZ, zaZA, zaZZ, zaZa, zaZz, zaa, zaaA, zaaZ, zaaa,
    zaaz, zaz, zazA, zazZ, zaza, zazz, zz, zzA, zzAA, zzAZ,
    zzAa, zzAz, zzZ, zzZA, zzZZ, zzZa, zzZz, zza, zzaA,
    zzaZ, zzaa, zzaz, zzz, zzzA, zzzZ, zzza, zzzz, _A, _Aa,
    _AaA, _Aaa, __A, __Aa, ___A, ___a, __a, __aA, _a, _aA,
    _aAA
};"#;

        let parse = SourceFile::parse(input, Edition::CURRENT);
        let root = parse.syntax_node();

        let mut use_items = Vec::new();
        for item in root.descendants() {
            if let Some(use_) = Use::cast(item) {
                use_items.push((Vec::new(), use_, Vec::new()));
            }
        }

        let mut buf = String::new();
        sort_and_format_imports(&use_items, &mut buf, 0);

        assert_snapshot!(buf, @r"
        use a::{
            a, aA, aAA, aAAA, aAAZ, aAAa, aAAz, aAZ, aAZA, aAZZ, aAZa, aAZz, aAa, aAaA, aAaZ, aAaa, aAaz,
            aAz, aAzA, aAzZ, aAza, aAzz, aZ, aZA, aZAA, aZAZ, aZAa, aZAz, aZZ, aZZA, aZZZ, aZZa, aZZz, aZa,
            aZaA, aZaZ, aZaa, aZaz, aZz, aZzA, aZzZ, aZza, aZzz, aa, aaA, aaAA, aaAZ, aaAa, aaAz, aaZ,
            aaZA, aaZZ, aaZa, aaZz, aaa, aaaA, aaaZ, aaaa, aaaz, aaz, aazA, aazZ, aaza, aazz, az, azA,
            azAA, azAZ, azAa, azAz, azZ, azZA, azZZ, azZa, azZz, aza, azaA, azaZ, azaa, azaz, azz, azzA,
            azzZ, azza, azzz, z, zA, zAA, zAAA, zAAZ, zAAa, zAAz, zAZ, zAZA, zAZZ, zAZa, zAZz, zAa, zAaA,
            zAaZ, zAaa, zAaz, zAz, zAzA, zAzZ, zAza, zAzz, zZ, zZA, zZAA, zZAZ, zZAa, zZAz, zZZ, zZZA,
            zZZZ, zZZa, zZZz, zZa, zZaA, zZaZ, zZaa, zZaz, zZz, zZzA, zZzZ, zZza, zZzz, za, zaA, zaAA,
            zaAZ, zaAa, zaAz, zaZ, zaZA, zaZZ, zaZa, zaZz, zaa, zaaA, zaaZ, zaaa, zaaz, zaz, zazA, zazZ,
            zaza, zazz, zz, zzA, zzAA, zzAZ, zzAa, zzAz, zzZ, zzZA, zzZZ, zzZa, zzZz, zza, zzaA, zzaZ,
            zzaa, zzaz, zzz, zzzA, zzzZ, zzza, zzzz, AAAa, AAAz, AAZa, AAZz, AAa, AAaA, AAaZ, AAaa, AAaz,
            AAz, AAzA, AAzZ, AAza, AAzz, AZAa, AZAz, AZZa, AZZz, AZa, AZaA, AZaZ, AZaa, AZaz, AZz, AZzA,
            AZzZ, AZza, AZzz, Aa, AaA, AaAA, AaAZ, AaAa, AaAz, AaZ, AaZA, AaZZ, AaZa, AaZz, Aaa, AaaA,
            AaaZ, Aaaa, Aaaz, Aaz, AazA, AazZ, Aaza, Aazz, Az, AzA, AzAA, AzAZ, AzAa, AzAz, AzZ, AzZA,
            AzZZ, AzZa, AzZz, Aza, AzaA, AzaZ, Azaa, Azaz, Azz, AzzA, AzzZ, Azza, Azzz, ZAAa, ZAAz, ZAZa,
            ZAZz, ZAa, ZAaA, ZAaZ, ZAaa, ZAaz, ZAz, ZAzA, ZAzZ, ZAza, ZAzz, ZZAa, ZZAz, ZZZa, ZZZz, ZZa,
            ZZaA, ZZaZ, ZZaa, ZZaz, ZZz, ZZzA, ZZzZ, ZZza, ZZzz, Za, ZaA, ZaAA, ZaAZ, ZaAa, ZaAz, ZaZ,
            ZaZA, ZaZZ, ZaZa, ZaZz, Zaa, ZaaA, ZaaZ, Zaaa, Zaaz, Zaz, ZazA, ZazZ, Zaza, Zazz, Zz, ZzA,
            ZzAA, ZzAZ, ZzAa, ZzAz, ZzZ, ZzZA, ZzZZ, ZzZa, ZzZz, Zza, ZzaA, ZzaZ, Zzaa, Zzaz, Zzz, ZzzA,
            ZzzZ, Zzza, Zzzz, _Aa, _AaA, _Aaa, __Aa, ___a, __a, __aA, _a, _aA, _aAA, A, AA, AAA, AAAA,
            AAAZ, AAZ, AAZA, AAZZ, AZ, AZA, AZAA, AZAZ, AZZ, AZZA, AZZZ, Z, ZA, ZAA, ZAAA, ZAAZ, ZAZ, ZAZA,
            ZAZZ, ZZ, ZZA, ZZAA, ZZAZ, ZZZ, ZZZA, ZZZZ, _A, __A, ___A,
        };
        ");
    }
}
