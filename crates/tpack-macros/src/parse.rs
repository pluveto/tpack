use proc_macro::{Delimiter, TokenStream, TokenTree};

use crate::ast::{EnumVariant, Field, Item, ItemKind};

pub(crate) fn parse_item(input: TokenStream) -> Result<Item, String> {
    let tokens: Vec<_> = input.into_iter().collect();
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index] {
            TokenTree::Ident(ident) if ident.to_string() == "struct" => {
                return parse_struct(&tokens, index + 1);
            }
            TokenTree::Ident(ident) if ident.to_string() == "enum" => {
                return parse_enum(&tokens, index + 1);
            }
            _ => index += 1,
        }
    }
    Err("Tpack derive supports structs and enums".into())
}

fn parse_struct(tokens: &[TokenTree], mut index: usize) -> Result<Item, String> {
    let name = next_ident(tokens, &mut index)?;
    while index < tokens.len() {
        if let TokenTree::Group(group) = &tokens[index] {
            if group.delimiter() == Delimiter::Brace {
                let fields = parse_named_fields(group.stream())?;
                return Ok(Item {
                    name,
                    kind: ItemKind::Struct(fields),
                });
            }
        }
        index += 1;
    }
    Err("Tpack derive supports only named-field structs".into())
}

fn parse_enum(tokens: &[TokenTree], mut index: usize) -> Result<Item, String> {
    let name = next_ident(tokens, &mut index)?;
    while index < tokens.len() {
        if let TokenTree::Group(group) = &tokens[index] {
            if group.delimiter() == Delimiter::Brace {
                let variants = parse_variants(group.stream())?;
                return Ok(Item {
                    name,
                    kind: ItemKind::Enum(variants),
                });
            }
        }
        index += 1;
    }
    Err("malformed enum for Tpack derive".into())
}

fn parse_named_fields(input: TokenStream) -> Result<Vec<Field>, String> {
    let tokens: Vec<_> = input.into_iter().collect();
    let mut fields: Vec<Field> = Vec::new();
    let mut index = 0;
    let mut pending_attr = Attr::default();
    while index < tokens.len() {
        if is_comma(&tokens[index]) {
            index += 1;
            continue;
        }
        if let Some(attr) = parse_tpack_attr_at(&tokens, &mut index)? {
            pending_attr.merge(attr);
            continue;
        }
        skip_visibility(&tokens, &mut index);
        let Some(rust_name) = maybe_ident(&tokens, &mut index) else {
            break;
        };
        expect_punct(&tokens, &mut index, ':')?;
        let ty = collect_until_comma(&tokens, &mut index);
        let field_id = pending_attr
            .field_id
            .ok_or_else(|| format!("field `{rust_name}` is missing #[tpack(field_id = N)]"))?;
        let wire_name = pending_attr
            .rename
            .take()
            .unwrap_or_else(|| rust_name.clone());
        if fields.iter().any(|field| field.field_id == field_id) {
            return Err(format!("duplicate #[tpack(field_id = {field_id})]"));
        }
        if fields.iter().any(|field| field.wire_name == wire_name) {
            return Err(format!("duplicate tpack field name `{wire_name}`"));
        }
        fields.push(Field {
            wire_name,
            rust_name,
            field_id,
            tpack_ty: pending_attr.tpack_ty.take(),
            ty,
        });
        pending_attr = Attr::default();
        if index < tokens.len() && is_comma(&tokens[index]) {
            index += 1;
        }
    }
    Ok(fields)
}

fn parse_variants(input: TokenStream) -> Result<Vec<EnumVariant>, String> {
    let tokens: Vec<_> = input.into_iter().collect();
    let mut variants = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if is_comma(&tokens[index]) {
            index += 1;
            continue;
        }
        if parse_tpack_attr_at(&tokens, &mut index)?.is_some() {
            continue;
        }
        let Some(name) = maybe_ident(&tokens, &mut index) else {
            break;
        };
        let payload_ty = if index < tokens.len() {
            match &tokens[index] {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
                    index += 1;
                    let inner = group.stream().to_string();
                    let pieces = split_top_level_commas(&inner);
                    if pieces.len() != 1 {
                        return Err(format!(
                            "enum variant `{name}` must have zero fields or exactly one unnamed field"
                        ));
                    }
                    Some(pieces[0].trim().to_string())
                }
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    return Err(format!(
                        "enum variant `{name}` uses struct fields, which this derive does not support"
                    ));
                }
                _ => None,
            }
        } else {
            None
        };
        variants.push(EnumVariant { name, payload_ty });
        while index < tokens.len() && !is_comma(&tokens[index]) {
            index += 1;
        }
        if index < tokens.len() && is_comma(&tokens[index]) {
            index += 1;
        }
    }
    Ok(variants)
}

#[derive(Debug, Clone, Default)]
struct Attr {
    field_id: Option<u64>,
    rename: Option<String>,
    tpack_ty: Option<String>,
}

impl Attr {
    fn merge(&mut self, other: Attr) {
        if other.field_id.is_some() {
            self.field_id = other.field_id;
        }
        if other.rename.is_some() {
            self.rename = other.rename;
        }
        if other.tpack_ty.is_some() {
            self.tpack_ty = other.tpack_ty;
        }
    }
}

fn parse_tpack_attr_at(tokens: &[TokenTree], index: &mut usize) -> Result<Option<Attr>, String> {
    if *index + 1 >= tokens.len() || !is_punct(&tokens[*index], '#') {
        return Ok(None);
    }
    let TokenTree::Group(group) = &tokens[*index + 1] else {
        return Ok(None);
    };
    if group.delimiter() != Delimiter::Bracket {
        return Ok(None);
    }
    *index += 2;
    let mut inner = group.stream().into_iter();
    let Some(TokenTree::Ident(name)) = inner.next() else {
        return Ok(None);
    };
    if name.to_string() != "tpack" {
        return Ok(None);
    }
    let Some(TokenTree::Group(args)) = inner.next() else {
        return Ok(Some(Attr::default()));
    };
    let mut attr = Attr::default();
    let args: Vec<_> = args.stream().into_iter().collect();
    let mut i = 0;
    while i < args.len() {
        if is_comma(&args[i]) {
            i += 1;
            continue;
        }
        let key = token_text(&args[i]);
        i += 1;
        expect_punct(&args, &mut i, '=')?;
        let Some(value) = args.get(i) else {
            return Err(format!("missing value for tpack attribute `{key}`"));
        };
        match key.as_str() {
            "field_id" => {
                attr.field_id = Some(
                    token_text(value)
                        .parse()
                        .map_err(|_| "field_id must be an integer literal".to_string())?,
                );
            }
            "rename" => {
                attr.rename = Some(unquote_literal(value)?);
            }
            "type" | "ty" => {
                attr.tpack_ty = Some(unquote_literal(value)?);
            }
            _ => return Err(format!("unsupported tpack attribute `{key}`")),
        }
        i += 1;
    }
    Ok(Some(attr))
}

fn next_ident(tokens: &[TokenTree], index: &mut usize) -> Result<String, String> {
    while let Some(token) = tokens.get(*index) {
        *index += 1;
        if let TokenTree::Ident(ident) = token {
            return Ok(ident.to_string());
        }
    }
    Err("expected identifier".into())
}

fn maybe_ident(tokens: &[TokenTree], index: &mut usize) -> Option<String> {
    match tokens.get(*index) {
        Some(TokenTree::Ident(ident)) => {
            *index += 1;
            Some(ident.to_string())
        }
        _ => None,
    }
}

fn skip_visibility(tokens: &[TokenTree], index: &mut usize) {
    if matches!(tokens.get(*index), Some(TokenTree::Ident(ident)) if ident.to_string() == "pub") {
        *index += 1;
        if matches!(tokens.get(*index), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
        {
            *index += 1;
        }
    }
}

fn expect_punct(tokens: &[TokenTree], index: &mut usize, ch: char) -> Result<(), String> {
    if tokens.get(*index).is_some_and(|token| is_punct(token, ch)) {
        *index += 1;
        Ok(())
    } else {
        Err(format!("expected `{ch}`"))
    }
}

fn collect_until_comma(tokens: &[TokenTree], index: &mut usize) -> String {
    let mut parts = Vec::new();
    while *index < tokens.len() && !is_comma(&tokens[*index]) {
        parts.push(tokens[*index].to_string());
        *index += 1;
    }
    join_type_tokens(&parts)
}

fn join_type_tokens(parts: &[String]) -> String {
    let mut out = String::new();
    for part in parts {
        if should_join_without_space(out.chars().last(), part.chars().next()) {
            out.push_str(part);
        } else {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(part);
        }
    }
    out
}

fn should_join_without_space(previous: Option<char>, next: Option<char>) -> bool {
    matches!(
        (previous, next),
        (
            _,
            Some('<' | '>' | ':' | ',' | '&' | '\'' | '[' | ']' | '(' | ')')
        ) | (Some('<' | ':' | '&' | '\'' | '[' | '('), _)
    )
}

fn split_top_level_commas(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    for ch in input.chars() {
        match ch {
            '<' | '(' | '[' => {
                depth += 1;
                current.push(ch);
            }
            '>' | ')' | ']' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                if !current.trim().is_empty() {
                    out.push(current.trim().to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

fn is_comma(token: &TokenTree) -> bool {
    is_punct(token, ',')
}

fn is_punct(token: &TokenTree, ch: char) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == ch)
}

fn token_text(token: &TokenTree) -> String {
    token.to_string()
}

fn unquote_literal(token: &TokenTree) -> Result<String, String> {
    match token {
        TokenTree::Literal(literal) => {
            let text = literal.to_string();
            text.strip_prefix('"')
                .and_then(|text| text.strip_suffix('"'))
                .map(|text| text.to_string())
                .ok_or_else(|| "rename must be a string literal".to_string())
        }
        _ => Err("rename must be a string literal".into()),
    }
}
