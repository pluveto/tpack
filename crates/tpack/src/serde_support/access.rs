use std::collections::HashMap;

use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use tpack_core::{Field, TpackValue, TypeDescriptor, ValueMapEntry};

use super::error::Error;
use super::value::ValueDeserializer;

pub(super) struct SeqValueAccess<'a, 'de> {
    element_ty: &'a TypeDescriptor,
    values: std::vec::IntoIter<TpackValue<'de>>,
}

impl<'a, 'de> SeqValueAccess<'a, 'de> {
    pub(super) fn new(element_ty: &'a TypeDescriptor, values: Vec<TpackValue<'de>>) -> Self {
        Self {
            element_ty,
            values: values.into_iter(),
        }
    }

    pub(super) fn new_synthetic(
        values: Vec<(TypeDescriptor, TpackValue<'de>)>,
    ) -> SyntheticSeqAccess<'de> {
        SyntheticSeqAccess {
            values: values.into_iter(),
        }
    }
}

pub(super) struct StructTupleAccess<'a, 'de> {
    fields: &'a [Field],
    values: Vec<Option<TpackValue<'de>>>,
    index: usize,
}

impl<'a, 'de> StructTupleAccess<'a, 'de> {
    pub(super) fn new(
        fields: &'a [Field],
        values: Vec<(u64, TpackValue<'de>)>,
    ) -> Result<Self, Error> {
        Ok(Self {
            fields,
            values: struct_values_by_schema(fields, values)?,
            index: 0,
        })
    }
}

pub(super) struct SyntheticSeqAccess<'de> {
    values: std::vec::IntoIter<(TypeDescriptor, TpackValue<'de>)>,
}

pub(super) struct StructAccess<'a, 'de> {
    fields: &'a [Field],
    entries: std::vec::IntoIter<(usize, TpackValue<'de>)>,
    pending_value: Option<(usize, TpackValue<'de>)>,
}

impl<'a, 'de> StructAccess<'a, 'de> {
    pub(super) fn new(
        fields: &'a [Field],
        values: Vec<(u64, TpackValue<'de>)>,
    ) -> Result<Self, Error> {
        Ok(Self {
            fields,
            entries: struct_entries_by_id(fields, values)?.into_iter(),
            pending_value: None,
        })
    }
}

pub(super) struct MapValueAccess<'a, 'de> {
    key_ty: &'a TypeDescriptor,
    value_ty: &'a TypeDescriptor,
    entries: std::vec::IntoIter<ValueMapEntry<'de>>,
    pending_value: Option<TpackValue<'de>>,
}

impl<'a, 'de> MapValueAccess<'a, 'de> {
    pub(super) fn new(
        key_ty: &'a TypeDescriptor,
        value_ty: &'a TypeDescriptor,
        entries: Vec<ValueMapEntry<'de>>,
    ) -> Self {
        Self {
            key_ty,
            value_ty,
            entries: entries.into_iter(),
            pending_value: None,
        }
    }
}

pub(super) struct EnumValueAccess<'a, 'de> {
    variant_name: &'a str,
    payload_ty: Option<&'a TypeDescriptor>,
    payload: Option<TpackValue<'de>>,
}

impl<'a, 'de> EnumValueAccess<'a, 'de> {
    pub(super) fn unit(variant_name: &'a str) -> Self {
        Self {
            variant_name,
            payload_ty: None,
            payload: None,
        }
    }

    pub(super) fn with_payload(
        variant_name: &'a str,
        payload_ty: &'a TypeDescriptor,
        payload: TpackValue<'de>,
    ) -> Self {
        Self {
            variant_name,
            payload_ty: Some(payload_ty),
            payload: Some(payload),
        }
    }
}

pub(super) struct VariantValueAccess<'a, 'de> {
    payload_ty: Option<&'a TypeDescriptor>,
    payload: Option<TpackValue<'de>>,
}

fn struct_entries_by_id<'de>(
    fields: &[Field],
    values: Vec<(u64, TpackValue<'de>)>,
) -> Result<Vec<(usize, TpackValue<'de>)>, Error> {
    let field_indices = fields
        .iter()
        .enumerate()
        .map(|(index, field)| (field.id, index))
        .collect::<HashMap<_, _>>();
    let mut seen = vec![false; fields.len()];
    let mut entries = Vec::with_capacity(values.len().min(fields.len()));
    for (id, value) in values {
        let Some(&index) = field_indices.get(&id) else {
            continue;
        };
        if seen[index] {
            return Err(Error::invalid("duplicate struct field value"));
        }
        seen[index] = true;
        entries.push((index, value));
    }
    Ok(entries)
}

fn struct_values_by_schema<'de>(
    fields: &[Field],
    values: Vec<(u64, TpackValue<'de>)>,
) -> Result<Vec<Option<TpackValue<'de>>>, Error> {
    let mut by_schema = (0..fields.len()).map(|_| None).collect::<Vec<_>>();
    for (index, value) in struct_entries_by_id(fields, values)? {
        by_schema[index] = Some(value);
    }
    Ok(by_schema)
}

impl<'de, 'a> SeqAccess<'de> for SeqValueAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(value) = self.values.next() else {
            return Ok(None);
        };
        seed.deserialize(ValueDeserializer::new(self.element_ty, value))
            .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

impl<'de, 'a> SeqAccess<'de> for StructTupleAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(field) = self.fields.get(self.index) else {
            return Ok(None);
        };
        let value = self.values[self.index]
            .take()
            .ok_or(Error::invalid("missing struct field value"))?;
        self.index += 1;
        seed.deserialize(ValueDeserializer::new(&field.ty, value))
            .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len().saturating_sub(self.index))
    }
}

impl<'de> SeqAccess<'de> for SyntheticSeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some((ty, value)) = self.values.next() else {
            return Ok(None);
        };
        seed.deserialize(ValueDeserializer::new(&ty, value))
            .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

impl<'de, 'a> MapAccess<'de> for StructAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.pending_value.is_some() {
            return Err(Error::invalid("struct key requested before value"));
        }
        let Some((index, value)) = self.entries.next() else {
            return Ok(None);
        };
        self.pending_value = Some((index, value));
        let key = serde::de::value::StrDeserializer::<Error>::new(self.fields[index].name.as_str());
        seed.deserialize(key).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let (index, value) = self
            .pending_value
            .take()
            .ok_or(Error::invalid("struct value requested before key"))?;
        seed.deserialize(ValueDeserializer::new(&self.fields[index].ty, value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len() + usize::from(self.pending_value.is_some()))
    }
}

impl<'de, 'a> MapAccess<'de> for MapValueAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        let Some(entry) = self.entries.next() else {
            return Ok(None);
        };
        self.pending_value = Some(entry.value);
        seed.deserialize(ValueDeserializer::new(self.key_ty, entry.key))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self
            .pending_value
            .take()
            .ok_or(Error::invalid("map value requested before key"))?;
        seed.deserialize(ValueDeserializer::new(self.value_ty, value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}

impl<'de, 'a> EnumAccess<'de> for EnumValueAccess<'a, 'de> {
    type Error = Error;
    type Variant = VariantValueAccess<'a, 'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(serde::de::value::StrDeserializer::<Error>::new(
            self.variant_name,
        ))?;
        Ok((
            variant,
            VariantValueAccess {
                payload_ty: self.payload_ty,
                payload: self.payload,
            },
        ))
    }
}

impl<'de, 'a> VariantAccess<'de> for VariantValueAccess<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        if self.payload.is_some() {
            return Err(Error::invalid("expected unit enum variant"));
        }
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let ty = self
            .payload_ty
            .ok_or(Error::invalid("expected data enum variant"))?;
        let value = self
            .payload
            .ok_or(Error::invalid("missing union variant payload"))?;
        seed.deserialize(ValueDeserializer::new(ty, value))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let ty = self
            .payload_ty
            .ok_or(Error::invalid("expected tuple enum variant"))?;
        let value = self
            .payload
            .ok_or(Error::invalid("missing union variant payload"))?;
        de::Deserializer::deserialize_seq(ValueDeserializer::new(ty, value), visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let ty = self
            .payload_ty
            .ok_or(Error::invalid("expected struct enum variant"))?;
        let value = self
            .payload
            .ok_or(Error::invalid("missing union variant payload"))?;
        de::Deserializer::deserialize_map(ValueDeserializer::new(ty, value), visitor)
    }
}
