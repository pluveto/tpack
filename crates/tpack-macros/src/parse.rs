use proc_macro::{Delimiter, TokenStream, TokenTree};

use crate::ast::{EnumVariant, Field, Item, ItemKind};

pub(crate) fn parse_item(input: TokenStream) -> Result<Item, String> {
    let tokens: Vec<_> = input.into_iter().collect();
    Cursor::new(&tokens).parse_item()
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

struct Cursor<'a> {
    tokens: &'a [TokenTree],
    index: usize,
}

impl<'a> Cursor<'a> {
    fn new(tokens: &'a [TokenTree]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_item(mut self) -> Result<Item, String> {
        while let Some(token) = self.tokens.get(self.index) {
            match token {
                TokenTree::Ident(ident) if ident.to_string() == "struct" => {
                    self.index += 1;
                    return self.parse_struct();
                }
                TokenTree::Ident(ident) if ident.to_string() == "enum" => {
                    self.index += 1;
                    return self.parse_enum();
                }
                _ => self.index += 1,
            }
        }
        Err("Tpack derive supports structs and enums".into())
    }

    fn parse_struct(&mut self) -> Result<Item, String> {
        let name = self.next_ident()?;
        while let Some(token) = self.tokens.get(self.index) {
            if let TokenTree::Group(group) = token
                && group.delimiter() == Delimiter::Brace
            {
                let fields = Self::parse_named_fields_stream(group.stream())?;
                return Ok(Item {
                    name,
                    kind: ItemKind::Struct(fields),
                });
            }
            self.index += 1;
        }
        Err("Tpack derive supports only named-field structs".into())
    }

    fn parse_enum(&mut self) -> Result<Item, String> {
        let name = self.next_ident()?;
        while let Some(token) = self.tokens.get(self.index) {
            if let TokenTree::Group(group) = token
                && group.delimiter() == Delimiter::Brace
            {
                let variants = Self::parse_variants_stream(group.stream())?;
                return Ok(Item {
                    name,
                    kind: ItemKind::Enum(variants),
                });
            }
            self.index += 1;
        }
        Err("malformed enum for Tpack derive".into())
    }

    fn parse_named_fields_stream(input: TokenStream) -> Result<Vec<Field>, String> {
        let tokens: Vec<_> = input.into_iter().collect();
        Cursor::new(&tokens).parse_named_fields()
    }

    fn parse_named_fields(mut self) -> Result<Vec<Field>, String> {
        let mut fields = Vec::new();
        let mut pending_attr = Attr::default();
        while self.index < self.tokens.len() {
            if self.current_is_comma() {
                self.index += 1;
                continue;
            }
            if let Some(attr) = self.parse_tpack_attr()? {
                pending_attr.merge(attr);
                continue;
            }
            self.skip_visibility();
            let Some(rust_name) = self.maybe_ident() else {
                break;
            };
            self.expect_punct(':')?;
            let ty = self.collect_until_comma();
            let field_id = pending_attr
                .field_id
                .ok_or_else(|| format!("field `{rust_name}` is missing #[tpack(field_id = N)]"))?;
            let wire_name = pending_attr
                .rename
                .take()
                .unwrap_or_else(|| rust_name.clone());
            if fields
                .iter()
                .any(|field: &Field| field.field_id == field_id)
            {
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
            if self.current_is_comma() {
                self.index += 1;
            }
        }
        Ok(fields)
    }

    fn parse_variants_stream(input: TokenStream) -> Result<Vec<EnumVariant>, String> {
        let tokens: Vec<_> = input.into_iter().collect();
        Cursor::new(&tokens).parse_variants()
    }

    fn parse_variants(mut self) -> Result<Vec<EnumVariant>, String> {
        let mut variants = Vec::new();
        while self.index < self.tokens.len() {
            if self.current_is_comma() {
                self.index += 1;
                continue;
            }
            if self.parse_tpack_attr()?.is_some() {
                continue;
            }
            let Some(name) = self.maybe_ident() else {
                break;
            };
            let payload_ty = match self.tokens.get(self.index) {
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                    self.index += 1;
                    let pieces = Self::split_top_level_commas(&group.stream().to_string());
                    if pieces.len() != 1 {
                        return Err(format!(
                            "enum variant `{name}` must have zero fields or exactly one unnamed field"
                        ));
                    }
                    Some(pieces[0].trim().to_string())
                }
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
                    return Err(format!(
                        "enum variant `{name}` uses struct fields, which this derive does not support"
                    ));
                }
                _ => None,
            };
            variants.push(EnumVariant { name, payload_ty });
            while self.index < self.tokens.len() && !self.current_is_comma() {
                self.index += 1;
            }
            if self.current_is_comma() {
                self.index += 1;
            }
        }
        Ok(variants)
    }

    fn parse_tpack_attr(&mut self) -> Result<Option<Attr>, String> {
        if self.index + 1 >= self.tokens.len() || !self.current_is_punct('#') {
            return Ok(None);
        }
        let TokenTree::Group(group) = &self.tokens[self.index + 1] else {
            return Ok(None);
        };
        if group.delimiter() != Delimiter::Bracket {
            return Ok(None);
        }
        self.index += 2;
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
        let mut cursor = Cursor::new(&args);
        while cursor.index < cursor.tokens.len() {
            if cursor.current_is_comma() {
                cursor.index += 1;
                continue;
            }
            let key = cursor.token_text(
                cursor
                    .tokens
                    .get(cursor.index)
                    .ok_or("expected attribute key")?,
            );
            cursor.index += 1;
            cursor.expect_punct('=')?;
            let Some(value) = cursor.tokens.get(cursor.index) else {
                return Err(format!("missing value for tpack attribute `{key}`"));
            };
            match key.as_str() {
                "field_id" => {
                    attr.field_id = Some(
                        cursor
                            .token_text(value)
                            .parse()
                            .map_err(|_| "field_id must be an integer literal".to_string())?,
                    );
                }
                "rename" => attr.rename = Some(cursor.unquote_literal(value)?),
                "type" | "ty" => attr.tpack_ty = Some(cursor.unquote_literal(value)?),
                _ => return Err(format!("unsupported tpack attribute `{key}`")),
            }
            cursor.index += 1;
        }
        Ok(Some(attr))
    }

    fn next_ident(&mut self) -> Result<String, String> {
        while let Some(token) = self.tokens.get(self.index) {
            self.index += 1;
            if let TokenTree::Ident(ident) = token {
                return Ok(ident.to_string());
            }
        }
        Err("expected identifier".into())
    }

    fn maybe_ident(&mut self) -> Option<String> {
        match self.tokens.get(self.index) {
            Some(TokenTree::Ident(ident)) => {
                self.index += 1;
                Some(ident.to_string())
            }
            _ => None,
        }
    }

    fn skip_visibility(&mut self) {
        if matches!(self.tokens.get(self.index), Some(TokenTree::Ident(ident)) if ident.to_string() == "pub")
        {
            self.index += 1;
            if matches!(self.tokens.get(self.index), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
            {
                self.index += 1;
            }
        }
    }

    fn expect_punct(&mut self, ch: char) -> Result<(), String> {
        if self.current_is_punct(ch) {
            self.index += 1;
            Ok(())
        } else {
            Err(format!("expected `{ch}`"))
        }
    }

    fn collect_until_comma(&mut self) -> String {
        let mut parts = Vec::new();
        while self.index < self.tokens.len() && !self.current_is_comma() {
            parts.push(self.tokens[self.index].to_string());
            self.index += 1;
        }
        Self::join_type_tokens(&parts)
    }

    fn current_is_comma(&self) -> bool {
        self.tokens
            .get(self.index)
            .is_some_and(|token| Self::is_punct(token, ','))
    }

    fn current_is_punct(&self, ch: char) -> bool {
        self.tokens
            .get(self.index)
            .is_some_and(|token| Self::is_punct(token, ch))
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

    fn join_type_tokens(parts: &[String]) -> String {
        let mut out = String::new();
        for part in parts {
            if Self::should_join_without_space(out.chars().last(), part.chars().next()) {
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

    fn is_punct(token: &TokenTree, ch: char) -> bool {
        matches!(token, TokenTree::Punct(punct) if punct.as_char() == ch)
    }

    fn token_text(&self, token: &TokenTree) -> String {
        token.to_string()
    }

    fn unquote_literal(&self, token: &TokenTree) -> Result<String, String> {
        let text = self.token_text(token);
        if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
            Ok(text[1..text.len() - 1].to_string())
        } else {
            Err("expected string literal".into())
        }
    }
}
