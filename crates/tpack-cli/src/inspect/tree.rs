use tpack::{Message, TpackValue, TypeDescriptor, ValueMapEntry};

use crate::cli::InspectSection;

use super::shared::{
    bytes_hex, envelope_mode_name, find_struct_field, line, list_element_type, map_types,
    optional_inner_type, type_inline, type_needs_tree, union_variant_type, value_inline,
    value_needs_tree,
};

pub(super) fn write(message: &Message<'_>, section: InspectSection, out: &mut String) {
    TreeFormatter { out }.write_message(message, section);
}

struct TreeFormatter<'a> {
    out: &'a mut String,
}

impl TreeFormatter<'_> {
    fn write_message(&mut self, message: &Message<'_>, section: InspectSection) {
        match section {
            InspectSection::All => {
                line(self.out, 0, "message");
                self.write_envelope(message, 1);

                line(self.out, 1, "schema");
                self.write_type(&message.schema.root, 2);

                line(self.out, 1, "value");
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
        line(self.out, indent, "envelope");
        line(
            self.out,
            indent + 1,
            &format!("mode: {}", envelope_mode_name(message.envelope.mode)),
        );
        match &message.envelope.schema_id {
            Some(schema_id) => line(
                self.out,
                indent + 1,
                &format!("schema_id: {}", bytes_hex(schema_id.as_bytes())),
            ),
            None => line(self.out, indent + 1, "schema_id: null"),
        }
        line(
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
                line(self.out, indent, "struct");
                for field in fields {
                    line(
                        self.out,
                        indent + 1,
                        &format!("{}#{}: {}", field.name, field.id, type_inline(&field.ty)),
                    );
                    if type_needs_tree(&field.ty) {
                        self.write_type(&field.ty, indent + 2);
                    }
                }
            }
            TypeDescriptor::List { max_count, element } => {
                line(
                    self.out,
                    indent,
                    &format!(
                        "list{}",
                        super::shared::optional_limit("max_count", *max_count)
                    ),
                );
                self.write_type(element, indent + 1);
            }
            TypeDescriptor::Map {
                max_count,
                key,
                value,
            } => {
                line(
                    self.out,
                    indent,
                    &format!(
                        "map{}",
                        super::shared::optional_limit("max_count", *max_count)
                    ),
                );
                line(self.out, indent + 1, &format!("key: {}", type_inline(key)));
                if type_needs_tree(key) {
                    self.write_type(key, indent + 2);
                }
                line(
                    self.out,
                    indent + 1,
                    &format!("value: {}", type_inline(value)),
                );
                if type_needs_tree(value) {
                    self.write_type(value, indent + 2);
                }
            }
            TypeDescriptor::Union(variants) => {
                line(self.out, indent, "union");
                for (index, variant) in variants.iter().enumerate() {
                    line(
                        self.out,
                        indent + 1,
                        &format!("#{index} {}: {}", variant.name, type_inline(&variant.ty)),
                    );
                    if type_needs_tree(&variant.ty) {
                        self.write_type(&variant.ty, indent + 2);
                    }
                }
            }
            TypeDescriptor::Enum(symbols) => {
                line(self.out, indent, "enum");
                for (index, symbol) in symbols.iter().enumerate() {
                    line(self.out, indent + 1, &format!("#{index}: {symbol}"));
                }
            }
            TypeDescriptor::Optional(inner) => {
                line(
                    self.out,
                    indent,
                    &format!("optional: {}", type_inline(inner)),
                );
                if type_needs_tree(inner) {
                    self.write_type(inner, indent + 1);
                }
            }
            _ => line(self.out, indent, &type_inline(ty)),
        }
    }

    fn write_value(&mut self, value: &TpackValue<'_>, ty: Option<&TypeDescriptor>, indent: usize) {
        match value {
            TpackValue::Struct(values) => {
                line(self.out, indent, "struct");
                for (field_id, field_value) in values {
                    let field = find_struct_field(ty, *field_id);
                    let field_ty = field.map(|field| &field.ty);
                    let name = field
                        .map(|field| field.name.as_str())
                        .unwrap_or("<unknown>");
                    line(
                        self.out,
                        indent + 1,
                        &format!("{name}#{field_id}: {}", value_inline(field_value)),
                    );
                    if value_needs_tree(field_value) {
                        self.write_value(field_value, field_ty, indent + 2);
                    }
                }
            }
            TpackValue::List(values) => {
                line(self.out, indent, "list");
                let element_ty = list_element_type(ty);
                for (index, item) in values.iter().enumerate() {
                    line(
                        self.out,
                        indent + 1,
                        &format!("[{index}]: {}", value_inline(item)),
                    );
                    if value_needs_tree(item) {
                        self.write_value(item, element_ty, indent + 2);
                    }
                }
            }
            TpackValue::Map(entries) => {
                line(self.out, indent, "map");
                let (key_ty, value_ty) = map_types(ty);
                for ValueMapEntry { key, value } in entries {
                    line(self.out, indent + 1, &format!("key: {}", value_inline(key)));
                    if value_needs_tree(key) {
                        self.write_value(key, key_ty, indent + 2);
                    }
                    line(
                        self.out,
                        indent + 1,
                        &format!("value: {}", value_inline(value)),
                    );
                    if value_needs_tree(value) {
                        self.write_value(value, value_ty, indent + 2);
                    }
                }
            }
            TpackValue::Union { index, value } => {
                line(self.out, indent, &format!("union variant #{index}"));
                self.write_value(value, union_variant_type(ty, *index), indent + 1);
            }
            TpackValue::Optional(Some(value)) => {
                line(self.out, indent, "some");
                self.write_value(value, optional_inner_type(ty), indent + 1);
            }
            TpackValue::Optional(None) => line(self.out, indent, "none"),
            _ => line(self.out, indent, &value_inline(value)),
        }
    }
}
