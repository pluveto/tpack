use proc_macro::TokenStream;

use crate::ast::{EnumVariant, Field, Item, ItemKind};

pub(crate) fn impl_serialize(item: &Item) -> Result<TokenStream, String> {
    match &item.kind {
        ItemKind::Struct(fields) => impl_struct_serialize(&item.name, fields),
        ItemKind::Enum(variants) => impl_enum_serialize(&item.name, variants),
    }
}

pub(crate) fn impl_deserialize(item: &Item) -> Result<TokenStream, String> {
    match &item.kind {
        ItemKind::Struct(fields) => impl_struct_deserialize(&item.name, fields),
        ItemKind::Enum(variants) => impl_enum_deserialize(&item.name, variants),
    }
}

fn impl_struct_serialize(name: &str, fields: &[Field]) -> Result<TokenStream, String> {
    let schema_fields = fields
        .iter()
        .map(|field| {
            let ty_expr = field
                .tpack_ty
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| {
                    format!("<{} as ::tpack::TpackSerialize>::schema().root", field.ty)
                });
            format!(
                "::tpack::Field::new({}, {:?}, {ty_expr})",
                field.field_id, field.wire_name
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let values = fields
        .iter()
        .map(|field| {
            format!(
                "({}, ::tpack::TpackSerialize::to_value(&self.{}))",
                field.field_id, field.rust_name
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    parse_tokens(format!(
        r#"
        impl ::tpack::TpackSerialize for {name} {{
            fn schema() -> ::tpack::Schema {{
                ::tpack::Schema::new(::tpack::TypeDescriptor::Struct(::std::vec![{schema_fields}]))
            }}

            fn to_value(&self) -> ::tpack::TpackValue<'_> {{
                ::tpack::TpackValue::Struct(::std::vec![{values}])
            }}
        }}
        "#
    ))
}

fn impl_struct_deserialize(name: &str, fields: &[Field]) -> Result<TokenStream, String> {
    let schema_fields = fields
        .iter()
        .map(|field| {
            let ty_expr = field
                .tpack_ty
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| {
                    format!("<{} as ::tpack::TpackDeserialize>::schema().root", field.ty)
                });
            format!(
                "::tpack::Field::new({}, {:?}, {ty_expr})",
                field.field_id, field.wire_name
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let declarations = fields
        .iter()
        .enumerate()
        .map(|(index, _)| format!("let mut __tpack_field_{index} = ::std::option::Option::None;"))
        .collect::<Vec<_>>()
        .join("\n");
    let match_arms = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            format!(
                r#"
                {} => {{
                    if __tpack_field_{index}.is_some() {{
                        return Err(::tpack::Error::new(::tpack::ErrorKind::Invalid(::std::borrow::Cow::Borrowed("duplicate struct field"))));
                    }}
                    __tpack_field_{index} = ::std::option::Option::Some(<{} as ::tpack::TpackDeserialize>::from_value(val)?);
                }}
                "#,
                field.field_id, field.ty
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let initializers = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            if field.ty.is_option() {
                return format!(
                    "{}: __tpack_field_{index}.unwrap_or(::std::option::Option::None)",
                    field.rust_name
                );
            }
            let message = format!("missing struct field `{}`", field.rust_name);
            format!(
                "{}: __tpack_field_{index}.ok_or_else(|| ::tpack::Error::new(::tpack::ErrorKind::Invalid(::std::borrow::Cow::Borrowed({message:?}))))?",
                field.rust_name
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    parse_tokens(format!(
        r#"
        impl<'de> ::tpack::TpackDeserialize<'de> for {name} {{
            fn schema() -> ::tpack::Schema {{
                ::tpack::Schema::new(::tpack::TypeDescriptor::Struct(::std::vec![{schema_fields}]))
            }}

            fn from_value(value: ::tpack::TpackValue<'de>) -> ::tpack::Result<Self> {{
                let ::tpack::TpackValue::Struct(fields) = value else {{
                    return Err(::tpack::Error::new(::tpack::ErrorKind::TypeMismatch {{ expected: "Struct" }}));
                }};
                {declarations}
                for (id, val) in fields {{
                    match id {{
                        {match_arms}
                        _ => {{}}
                    }}
                }}
                Ok(Self {{ {initializers} }})
            }}
        }}
        "#
    ))
}

fn impl_enum_serialize(name: &str, variants: &[EnumVariant]) -> Result<TokenStream, String> {
    if variants.iter().all(|variant| variant.payload_ty.is_none()) {
        let symbols = variants
            .iter()
            .map(|variant| format!("::std::string::String::from({:?})", variant.name))
            .collect::<Vec<_>>()
            .join(", ");
        let arms = variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                format!(
                    "Self::{} => ::tpack::TpackValue::Enum({})",
                    variant.name, index
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        return parse_tokens(format!(
            r#"
            impl ::tpack::TpackSerialize for {name} {{
                fn schema() -> ::tpack::Schema {{
                    ::tpack::Schema::new(::tpack::TypeDescriptor::Enum(::std::vec![{symbols}]))
                }}

                fn to_value(&self) -> ::tpack::TpackValue<'_> {{
                    match self {{ {arms} }}
                }}
            }}
            "#
        ));
    }

    let variant_defs = variants
        .iter()
        .map(|variant| {
            let ty = variant.payload_ty.as_ref().ok_or_else(|| {
                "data enums must give every variant exactly one unnamed field".to_string()
            })?;
            Ok(format!(
                "::tpack::Variant::new({:?}, <{} as ::tpack::TpackSerialize>::schema().root)",
                variant.name, ty
            ))
        })
        .collect::<Result<Vec<_>, String>>()?
        .join(", ");
    let arms = variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            format!(
                "Self::{}(value) => ::tpack::TpackValue::Union {{ index: {}, value: ::std::boxed::Box::new(::tpack::TpackSerialize::to_value(value)) }}",
                variant.name, index
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    parse_tokens(format!(
        r#"
        impl ::tpack::TpackSerialize for {name} {{
            fn schema() -> ::tpack::Schema {{
                ::tpack::Schema::new(::tpack::TypeDescriptor::Union(::std::vec![{variant_defs}]))
            }}

            fn to_value(&self) -> ::tpack::TpackValue<'_> {{
                match self {{ {arms} }}
            }}
        }}
        "#
    ))
}

fn impl_enum_deserialize(name: &str, variants: &[EnumVariant]) -> Result<TokenStream, String> {
    if variants.iter().all(|variant| variant.payload_ty.is_none()) {
        let symbols = variants
            .iter()
            .map(|variant| format!("::std::string::String::from({:?})", variant.name))
            .collect::<Vec<_>>()
            .join(", ");
        let arms = variants
            .iter()
            .enumerate()
            .map(|(index, variant)| format!("{index} => Ok(Self::{}),", variant.name))
            .collect::<Vec<_>>()
            .join("");
        return parse_tokens(format!(
            r#"
            impl<'de> ::tpack::TpackDeserialize<'de> for {name} {{
                fn schema() -> ::tpack::Schema {{
                    ::tpack::Schema::new(::tpack::TypeDescriptor::Enum(::std::vec![{symbols}]))
                }}

                fn from_value(value: ::tpack::TpackValue<'de>) -> ::tpack::Result<Self> {{
                    let ::tpack::TpackValue::Enum(index) = value else {{
                        return Err(::tpack::Error::new(::tpack::ErrorKind::TypeMismatch {{ expected: "Enum" }}));
                    }};
                    match index {{ {arms} _ => Err(::tpack::Error::new(::tpack::ErrorKind::Invalid(::std::borrow::Cow::Borrowed("enum index out of range")))) }}
                }}
            }}
            "#
        ));
    }

    let variant_defs = variants
        .iter()
        .map(|variant| {
            let ty = variant.payload_ty.as_ref().ok_or_else(|| {
                "data enums must give every variant exactly one unnamed field".to_string()
            })?;
            Ok(format!(
                "::tpack::Variant::new({:?}, <{} as ::tpack::TpackDeserialize>::schema().root)",
                variant.name, ty
            ))
        })
        .collect::<Result<Vec<_>, String>>()?
        .join(", ");
    let arms = variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let ty = variant.payload_ty.as_ref().ok_or_else(|| {
                "data enums must give every variant exactly one unnamed field".to_string()
            })?;
            Ok(format!(
                "{} => Ok(Self::{}(<{} as ::tpack::TpackDeserialize>::from_value(*value)?)),",
                index, variant.name, ty
            ))
        })
        .collect::<Result<Vec<_>, String>>()?
        .join("");
    parse_tokens(format!(
        r#"
        impl<'de> ::tpack::TpackDeserialize<'de> for {name} {{
            fn schema() -> ::tpack::Schema {{
                ::tpack::Schema::new(::tpack::TypeDescriptor::Union(::std::vec![{variant_defs}]))
            }}

            fn from_value(value: ::tpack::TpackValue<'de>) -> ::tpack::Result<Self> {{
                let ::tpack::TpackValue::Union {{ index, value, .. }} = value else {{
                    return Err(::tpack::Error::new(::tpack::ErrorKind::TypeMismatch {{ expected: "Union" }}));
                }};
                match index {{ {arms} _ => Err(::tpack::Error::new(::tpack::ErrorKind::Invalid(::std::borrow::Cow::Borrowed("union index out of range")))) }}
            }}
        }}
        "#
    ))
}

fn parse_tokens(source: String) -> Result<TokenStream, String> {
    source.parse().map_err(|_| source)
}
