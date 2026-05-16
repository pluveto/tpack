use proc_macro::{Delimiter, TokenStream, TokenTree};

use crate::ast::{EnumVariant, Field, Item, ItemKind, TypeKind, TypePath, TypeRef};

pub(crate) fn parse_item(input: TokenStream) -> Result<Item, String> {
    let tokens: Vec<_> = input.into_iter().collect();
    Cursor::new(&tokens).parse_item()
}

#[derive(Debug, Clone, Default)]
struct Attr {
    auto: bool,
    field_id: Option<u64>,
    rename: Option<String>,
    tpack_ty: Option<String>,
}

impl Attr {
    fn merge(&mut self, other: Attr) {
        self.auto |= other.auto;
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
        let mut auto_field_id = false;
        while let Some(token) = self.tokens.get(self.index) {
            if let Some(attr) = self.parse_tpack_attr()? {
                auto_field_id |= attr.auto;
                continue;
            }
            if self.parse_tpack_container_attr(&mut auto_field_id)? {
                continue;
            }
            match token {
                TokenTree::Ident(ident) if ident.to_string() == "struct" => {
                    self.index += 1;
                    return self.parse_struct(auto_field_id);
                }
                TokenTree::Ident(ident) if ident.to_string() == "enum" => {
                    self.index += 1;
                    return self.parse_enum(auto_field_id);
                }
                _ => self.index += 1,
            }
        }
        Err("Tpack derive supports structs and enums".into())
    }

    fn parse_struct(&mut self, auto_field_id: bool) -> Result<Item, String> {
        let name = self.next_ident()?;
        while let Some(token) = self.tokens.get(self.index) {
            if let TokenTree::Group(group) = token {
                if group.delimiter() == Delimiter::Brace {
                    let fields = Self::parse_named_fields_stream(group.stream(), auto_field_id)?;
                    return Ok(Item {
                        name,
                        kind: ItemKind::Struct(fields),
                    });
                }
            }
            self.index += 1;
        }
        Err("Tpack derive supports only named-field structs".into())
    }

    fn parse_enum(&mut self, _auto_field_id: bool) -> Result<Item, String> {
        let name = self.next_ident()?;
        while let Some(token) = self.tokens.get(self.index) {
            if let TokenTree::Group(group) = token {
                if group.delimiter() == Delimiter::Brace {
                    let variants = Self::parse_variants_stream(group.stream())?;
                    return Ok(Item {
                        name,
                        kind: ItemKind::Enum(variants),
                    });
                }
            }
            self.index += 1;
        }
        Err("malformed enum for Tpack derive".into())
    }

    fn parse_named_fields_stream(
        input: TokenStream,
        auto_field_id: bool,
    ) -> Result<Vec<Field>, String> {
        let tokens: Vec<_> = input.into_iter().collect();
        Cursor::new(&tokens).parse_named_fields(auto_field_id)
    }

    fn parse_named_fields(mut self, auto_field_id: bool) -> Result<Vec<Field>, String> {
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
            let ty = self.collect_type_until_comma();
            let field_id = if auto_field_id {
                pending_attr.field_id.unwrap_or((fields.len() as u64) + 1)
            } else {
                pending_attr.field_id.ok_or_else(|| {
                    format!("field `{rust_name}` is missing #[tpack(field_id = N)]")
                })?
            };
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
                    let tokens: Vec<_> = group.stream().into_iter().collect();
                    let pieces = Self::split_top_level_commas(&tokens);
                    if pieces.len() != 1 {
                        return Err(format!(
                            "enum variant `{name}` must have zero fields or exactly one unnamed field"
                        ));
                    }
                    Some(Self::build_type_ref(&pieces[0]))
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
            match key.as_str() {
                "auto" => attr.auto = true,
                "field_id" => {
                    cursor.expect_punct('=')?;
                    let Some(value) = cursor.tokens.get(cursor.index) else {
                        return Err("missing value for tpack attribute `field_id`".into());
                    };
                    attr.field_id = Some(
                        cursor
                            .token_text(value)
                            .parse()
                            .map_err(|_| "field_id must be an integer literal".to_string())?,
                    );
                }
                "rename" | "type" | "ty" => {
                    cursor.expect_punct('=')?;
                    let Some(value) = cursor.tokens.get(cursor.index) else {
                        return Err(format!("missing value for tpack attribute `{key}`"));
                    };
                    match key.as_str() {
                        "rename" => attr.rename = Some(cursor.unquote_literal(value)?),
                        "type" | "ty" => attr.tpack_ty = Some(cursor.unquote_literal(value)?),
                        _ => unreachable!(),
                    }
                }
                _ => return Err(format!("unsupported tpack attribute `{key}`")),
            }
            cursor.index += 1;
        }
        Ok(Some(attr))
    }

    fn parse_tpack_container_attr(&mut self, auto_field_id: &mut bool) -> Result<bool, String> {
        if self.index + 1 >= self.tokens.len() || !self.current_is_punct('#') {
            return Ok(false);
        }
        let TokenTree::Group(group) = &self.tokens[self.index + 1] else {
            return Ok(false);
        };
        if group.delimiter() != Delimiter::Bracket {
            return Ok(false);
        }
        self.index += 2;
        let mut inner = group.stream().into_iter();
        let Some(TokenTree::Ident(name)) = inner.next() else {
            return Ok(false);
        };
        if name.to_string() != "tpack" {
            return Ok(false);
        }
        let Some(TokenTree::Group(args)) = inner.next() else {
            *auto_field_id = true;
            return Ok(true);
        };
        if args.delimiter() != Delimiter::Parenthesis {
            return Err("malformed tpack container attribute".into());
        }
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
            match key.as_str() {
                "auto" => *auto_field_id = true,
                _ => return Err(format!("unsupported tpack container attribute `{key}`")),
            }
        }
        Ok(true)
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

    fn collect_type_until_comma(&mut self) -> TypeRef {
        let start = self.index;
        while self.index < self.tokens.len() && !self.current_is_comma() {
            self.index += 1;
        }
        Self::build_type_ref(&self.tokens[start..self.index])
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

    fn split_top_level_commas(tokens: &[TokenTree]) -> Vec<Vec<TokenTree>> {
        let mut out = Vec::new();
        let mut current = Vec::new();
        let mut depth = 0i32;
        for token in tokens {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == '<' => {
                    depth += 1;
                    current.push(token.clone());
                }
                TokenTree::Punct(punct) if punct.as_char() == '>' => {
                    depth -= 1;
                    current.push(token.clone());
                }
                TokenTree::Punct(punct) if punct.as_char() == ',' && depth == 0 => {
                    if !current.is_empty() {
                        out.push(current);
                    }
                    current = Vec::new();
                }
                _ => current.push(token.clone()),
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
        out
    }

    fn build_type_ref(tokens: &[TokenTree]) -> TypeRef {
        let parts = tokens.iter().map(TokenTree::to_string).collect::<Vec<_>>();
        let source = Self::join_type_tokens(&parts);
        let kind = Self::parse_type_path(tokens)
            .map(TypeKind::Path)
            .unwrap_or(TypeKind::Other);
        TypeRef::new(source, kind)
    }

    fn parse_type_path(tokens: &[TokenTree]) -> Option<TypePath> {
        let mut index = 0;
        Self::consume_double_colon(tokens, &mut index);
        let mut segments = Vec::new();
        loop {
            let TokenTree::Ident(ident) = tokens.get(index)? else {
                return None;
            };
            segments.push(ident.to_string());
            index += 1;
            if tokens
                .get(index)
                .is_some_and(|token| Self::is_punct(token, '<'))
            {
                Self::consume_angle_args(tokens, &mut index)?;
            }
            if !Self::consume_double_colon(tokens, &mut index) {
                break;
            }
        }
        (index == tokens.len()).then_some(TypePath { segments })
    }

    fn consume_double_colon(tokens: &[TokenTree], index: &mut usize) -> bool {
        match (tokens.get(*index), tokens.get(*index + 1)) {
            (Some(first), Some(second))
                if Self::is_punct(first, ':') && Self::is_punct(second, ':') =>
            {
                *index += 2;
                true
            }
            _ => false,
        }
    }

    fn consume_angle_args(tokens: &[TokenTree], index: &mut usize) -> Option<()> {
        let mut depth = 0i32;
        while let Some(token) = tokens.get(*index) {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == '<' => depth += 1,
                TokenTree::Punct(punct) if punct.as_char() == '>' => {
                    depth -= 1;
                    if depth == 0 {
                        *index += 1;
                        return Some(());
                    }
                }
                _ => {}
            }
            *index += 1;
        }
        None
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
