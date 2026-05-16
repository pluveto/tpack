use tpack::{Message, TpackValue, TypeDescriptor, ValueMapEntry};

use crate::cli::InspectSection;

use super::shared;

pub(super) struct TreeFormatter<'a> {
    out: &'a mut String,
}

impl<'a> TreeFormatter<'a> {
    pub(super) fn write(message: &Message<'_>, section: InspectSection, out: &'a mut String) {
        Self { out }.write_message(message, section);
    }

    fn write_message(&mut self, message: &Message<'_>, section: InspectSection) {
        match section {
            InspectSection::All => {
                shared::line(self.out, 0, "message");
                self.write_envelope(message, 1);

                shared::line(self.out, 1, "schema");
                self.write_type(&message.schema.root, 2);

                shared::line(self.out, 1, "value");
                self.write_value(&message.value, Some(&message.schema.root), 2);
            }
            InspectSection::Envelope => self.write_envelope(message, 0),
            InspectSection::Schema => self.write_type(&message.schema.root, 0),
            InspectSection::Value => {
                self.write_value(&message.value, Some(&message.schema.root), 0)
            }
        }
    }

    fn write_envelope(&mut self, message: &Message<'_>, indent: usize) {
        shared::line(self.out, indent, "envelope");
        shared::line(
            self.out,
            indent + 1,
            &format!(
                "mode: {}",
                shared::envelope_mode_name(message.envelope.mode)
            ),
        );
        match &message.envelope.schema_id {
            Some(schema_id) => shared::line(
                self.out,
                indent + 1,
                &format!("schema_id: {}", shared::bytes_hex(schema_id.as_bytes())),
            ),
            None => shared::line(self.out, indent + 1, "schema_id: null"),
        }
        shared::line(
            self.out,
            indent + 1,
            &format!(
                "used_cached_schema: {}",
                message.envelope.used_cached_schema
            ),
        );
    }

    fn write_type(&mut self, ty: &TypeDescriptor, indent: usize) {
        match ty {
            TypeDescriptor::Struct(fields) => {
                shared::line(self.out, indent, "struct");
                for field in fields {
                    shared::line(
                        self.out,
                        indent + 1,
                        &format!(
                            "{}#{}: {}",
                            field.name,
                            field.id,
                            shared::type_inline(&field.ty)
                        ),
                    );
                    if shared::type_needs_tree(&field.ty) {
                        self.write_type(&field.ty, indent + 2);
                    }
                }
            }
            TypeDescriptor::List { max_count, element } => {
                shared::line(
                    self.out,
                    indent,
                    &format!("list{}", shared::optional_limit("max_count", *max_count)),
                );
                self.write_type(element, indent + 1);
            }
            TypeDescriptor::Map {
                max_count,
                key,
                value,
            } => {
                shared::line(
                    self.out,
                    indent,
                    &format!("map{}", shared::optional_limit("max_count", *max_count)),
                );
                shared::line(
                    self.out,
                    indent + 1,
                    &format!("key: {}", shared::type_inline(key)),
                );
                if shared::type_needs_tree(key) {
                    self.write_type(key, indent + 2);
                }
                shared::line(
                    self.out,
                    indent + 1,
                    &format!("value: {}", shared::type_inline(value)),
                );
                if shared::type_needs_tree(value) {
                    self.write_type(value, indent + 2);
                }
            }
            TypeDescriptor::Union(variants) => {
                shared::line(self.out, indent, "union");
                for (index, variant) in variants.iter().enumerate() {
                    shared::line(
                        self.out,
                        indent + 1,
                        &format!(
                            "#{index} {}: {}",
                            variant.name,
                            shared::type_inline(&variant.ty)
                        ),
                    );
                    if shared::type_needs_tree(&variant.ty) {
                        self.write_type(&variant.ty, indent + 2);
                    }
                }
            }
            TypeDescriptor::Enum(symbols) => {
                shared::line(self.out, indent, "enum");
                for (index, symbol) in symbols.iter().enumerate() {
                    shared::line(self.out, indent + 1, &format!("#{index}: {symbol}"));
                }
            }
            TypeDescriptor::Optional(inner) => {
                shared::line(
                    self.out,
                    indent,
                    &format!("optional: {}", shared::type_inline(inner)),
                );
                if shared::type_needs_tree(inner) {
                    self.write_type(inner, indent + 1);
                }
            }
            _ => shared::line(self.out, indent, &shared::type_inline(ty)),
        }
    }

    fn write_value(&mut self, value: &TpackValue<'_>, ty: Option<&TypeDescriptor>, indent: usize) {
        match value {
            TpackValue::Struct(values) => {
                shared::line(self.out, indent, "struct");
                for (field_id, field_value) in values {
                    let field = shared::find_struct_field(ty, *field_id);
                    let field_ty = field.map(|field| &field.ty);
                    let name = field
                        .map(|field| field.name.as_str())
                        .unwrap_or("<unknown>");
                    shared::line(
                        self.out,
                        indent + 1,
                        &format!("{name}#{field_id}: {}", shared::value_inline(field_value)),
                    );
                    if shared::value_needs_tree(field_value) {
                        self.write_value(field_value, field_ty, indent + 2);
                    }
                }
            }
            TpackValue::List(values) => {
                shared::line(self.out, indent, "list");
                let element_ty = shared::list_element_type(ty);
                for (index, item) in values.iter().enumerate() {
                    shared::line(
                        self.out,
                        indent + 1,
                        &format!("[{index}]: {}", shared::value_inline(item)),
                    );
                    if shared::value_needs_tree(item) {
                        self.write_value(item, element_ty, indent + 2);
                    }
                }
            }
            TpackValue::Map(entries) => {
                shared::line(self.out, indent, "map");
                let (key_ty, value_ty) = shared::map_types(ty);
                for ValueMapEntry { key, value } in entries {
                    shared::line(
                        self.out,
                        indent + 1,
                        &format!("key: {}", shared::value_inline(key)),
                    );
                    if shared::value_needs_tree(key) {
                        self.write_value(key, key_ty, indent + 2);
                    }
                    shared::line(
                        self.out,
                        indent + 1,
                        &format!("value: {}", shared::value_inline(value)),
                    );
                    if shared::value_needs_tree(value) {
                        self.write_value(value, value_ty, indent + 2);
                    }
                }
            }
            TpackValue::Union { index, value } => {
                shared::line(self.out, indent, &format!("union variant #{index}"));
                self.write_value(value, shared::union_variant_type(ty, *index), indent + 1);
            }
            TpackValue::Optional(Some(value)) => {
                shared::line(self.out, indent, "some");
                self.write_value(value, shared::optional_inner_type(ty), indent + 1);
            }
            TpackValue::Optional(None) => shared::line(self.out, indent, "none"),
            _ => shared::line(self.out, indent, &shared::value_inline(value)),
        }
    }
}
