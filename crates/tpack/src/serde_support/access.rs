use std::collections::HashMap;

use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use tpack_core::{Field, TpackValue, TypeDescriptor, ValueMapEntry};

use super::error::Error;
use super::value::ValueDeserializer;

pub(super) struct SeqValueAccess<'a, 'de> {
    element_ty: &'a TypeDescriptor,
    values: std::vec::IntoIter<TpackValue<'de>>,
    index: usize,
    remaining_depth: usize,
}

impl<'a, 'de> SeqValueAccess<'a, 'de> {
    pub(super) fn from_values(
        element_ty: &'a TypeDescriptor,
        values: Vec<TpackValue<'de>>,
        remaining_depth: usize,
    ) -> Self {
        Self {
            element_ty,
            values: values.into_iter(),
            index: 0,
            remaining_depth,
        }
    }

    pub(super) fn from_typed_values(
        values: Vec<(TypeDescriptor, TpackValue<'de>)>,
        remaining_depth: usize,
    ) -> SyntheticSeqAccess<'de> {
        SyntheticSeqAccess {
            values: values.into_iter(),
            index: 0,
            remaining_depth,
        }
    }
}

pub(super) struct StructTupleAccess<'a, 'de> {
    fields: &'a [Field],
    values: Vec<Option<TpackValue<'de>>>,
    index: usize,
    remaining_depth: usize,
}

impl<'a, 'de> StructTupleAccess<'a, 'de> {
    pub(super) fn from_field_values(
        fields: &'a [Field],
        values: Vec<(u64, TpackValue<'de>)>,
        remaining_depth: usize,
    ) -> Result<Self, Error> {
        let matcher = StructFieldMatcher::from_fields(fields);
        Ok(Self {
            fields,
            values: matcher.values_by_schema(values)?,
            index: 0,
            remaining_depth,
        })
    }
}

pub(super) struct SyntheticSeqAccess<'de> {
    values: std::vec::IntoIter<(TypeDescriptor, TpackValue<'de>)>,
    index: usize,
    remaining_depth: usize,
}

pub(super) struct StructAccess<'a, 'de> {
    fields: &'a [Field],
    entries: std::vec::IntoIter<(usize, TpackValue<'de>)>,
    pending_value: Option<(usize, TpackValue<'de>)>,
    remaining_depth: usize,
}

impl<'a, 'de> StructAccess<'a, 'de> {
    pub(super) fn from_field_values(
        fields: &'a [Field],
        values: Vec<(u64, TpackValue<'de>)>,
        remaining_depth: usize,
    ) -> Result<Self, Error> {
        let matcher = StructFieldMatcher::from_fields(fields);
        Ok(Self {
            fields,
            entries: matcher.entries_by_id(values)?.into_iter(),
            pending_value: None,
            remaining_depth,
        })
    }
}

pub(super) struct MapValueAccess<'a, 'de> {
    key_ty: &'a TypeDescriptor,
    value_ty: &'a TypeDescriptor,
    entries: std::vec::IntoIter<ValueMapEntry<'de>>,
    pending_value: Option<TpackValue<'de>>,
    pending_index: Option<usize>,
    next_index: usize,
    remaining_depth: usize,
}

impl<'a, 'de> MapValueAccess<'a, 'de> {
    pub(super) fn from_entries(
        key_ty: &'a TypeDescriptor,
        value_ty: &'a TypeDescriptor,
        entries: Vec<ValueMapEntry<'de>>,
        remaining_depth: usize,
    ) -> Self {
        Self {
            key_ty,
            value_ty,
            entries: entries.into_iter(),
            pending_value: None,
            pending_index: None,
            next_index: 0,
            remaining_depth,
        }
    }
}

pub(super) struct EnumValueAccess<'a, 'de> {
    variant_name: &'a str,
    payload_ty: Option<&'a TypeDescriptor>,
    payload: Option<TpackValue<'de>>,
    remaining_depth: usize,
}

impl<'a, 'de> EnumValueAccess<'a, 'de> {
    pub(super) fn unit(variant_name: &'a str) -> Self {
        Self {
            variant_name,
            payload_ty: None,
            payload: None,
            remaining_depth: 0,
        }
    }

    pub(super) fn with_payload(
        variant_name: &'a str,
        payload_ty: &'a TypeDescriptor,
        payload: TpackValue<'de>,
        remaining_depth: usize,
    ) -> Self {
        Self {
            variant_name,
            payload_ty: Some(payload_ty),
            payload: Some(payload),
            remaining_depth,
        }
    }
}

pub(super) struct VariantValueAccess<'a, 'de> {
    payload_ty: Option<&'a TypeDescriptor>,
    payload: Option<TpackValue<'de>>,
    remaining_depth: usize,
}

struct StructFieldMatcher<'a> {
    fields: &'a [Field],
    field_indices: HashMap<u64, usize>,
}

impl<'a> StructFieldMatcher<'a> {
    fn from_fields(fields: &'a [Field]) -> Self {
        let field_indices = fields
            .iter()
            .enumerate()
            .map(|(index, field)| (field.id, index))
            .collect();
        Self {
            fields,
            field_indices,
        }
    }

    fn entries_by_id<'de>(
        &self,
        values: Vec<(u64, TpackValue<'de>)>,
    ) -> Result<Vec<(usize, TpackValue<'de>)>, Error> {
        let mut seen = vec![false; self.fields.len()];
        let mut entries = Vec::with_capacity(values.len().min(self.fields.len()));
        for (id, value) in values {
            let Some(&index) = self.field_indices.get(&id) else {
                continue;
            };
            if seen[index] {
                return Err(Error::duplicate_struct_field_value());
            }
            seen[index] = true;
            entries.push((index, value));
        }
        Ok(entries)
    }

    fn values_by_schema<'de>(
        &self,
        values: Vec<(u64, TpackValue<'de>)>,
    ) -> Result<Vec<Option<TpackValue<'de>>>, Error> {
        let mut by_schema = (0..self.fields.len()).map(|_| None).collect::<Vec<_>>();
        for (index, value) in self.entries_by_id(values)? {
            by_schema[index] = Some(value);
        }
        Ok(by_schema)
    }
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
        let index = self.index;
        self.index += 1;
        let deserializer = ValueDeserializer::child(self.element_ty, value, self.remaining_depth)
            .map_err(|err| err.at_index(index))?;
        seed.deserialize(deserializer)
            .map_err(|err| err.at_index(index))
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
            .ok_or_else(Error::missing_struct_field_value)?;
        self.index += 1;
        let deserializer = ValueDeserializer::child(&field.ty, value, self.remaining_depth)
            .map_err(|err| err.at_field(field.name.clone()))?;
        seed.deserialize(deserializer)
            .map_err(|err| err.at_field(field.name.clone()))
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
        let index = self.index;
        self.index += 1;
        seed.deserialize(ValueDeserializer::new(&ty, value, self.remaining_depth))
            .map_err(|err| err.at_index(index))
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
            return Err(Error::struct_key_before_value());
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
            .ok_or_else(Error::struct_value_before_key)?;
        let deserializer =
            ValueDeserializer::child(&self.fields[index].ty, value, self.remaining_depth)
                .map_err(|err| err.at_field(self.fields[index].name.clone()))?;
        seed.deserialize(deserializer)
            .map_err(|err| err.at_field(self.fields[index].name.clone()))
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
        let index = self.next_index;
        self.next_index += 1;
        self.pending_index = Some(index);
        self.pending_value = Some(entry.value);
        let deserializer = ValueDeserializer::child(self.key_ty, entry.key, self.remaining_depth)
            .map_err(|err| err.at_index(index))?;
        seed.deserialize(deserializer)
            .map_err(|err| err.at_index(index))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self
            .pending_value
            .take()
            .ok_or_else(Error::map_value_before_key)?;
        let index = self
            .pending_index
            .take()
            .expect("pending map value index tracks pending map value");
        let deserializer = ValueDeserializer::child(self.value_ty, value, self.remaining_depth)
            .map_err(|err| err.at_index(index))?;
        seed.deserialize(deserializer)
            .map_err(|err| err.at_index(index))
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
                remaining_depth: self.remaining_depth,
            },
        ))
    }
}

impl<'de, 'a> VariantAccess<'de> for VariantValueAccess<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        if self.payload.is_some() {
            return Err(Error::expected_unit_enum_variant());
        }
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let ty = self
            .payload_ty
            .ok_or_else(Error::expected_data_enum_variant)?;
        let value = self
            .payload
            .ok_or_else(Error::missing_union_variant_payload)?;
        let deserializer = ValueDeserializer::child(ty, value, self.remaining_depth)?;
        seed.deserialize(deserializer)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let ty = self
            .payload_ty
            .ok_or_else(Error::expected_tuple_enum_variant)?;
        let value = self
            .payload
            .ok_or_else(Error::missing_union_variant_payload)?;
        let deserializer = ValueDeserializer::child(ty, value, self.remaining_depth)?;
        de::Deserializer::deserialize_seq(deserializer, visitor)
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
            .ok_or_else(Error::expected_struct_enum_variant)?;
        let value = self
            .payload
            .ok_or_else(Error::missing_union_variant_payload)?;
        let deserializer = ValueDeserializer::child(ty, value, self.remaining_depth)?;
        de::Deserializer::deserialize_map(deserializer, visitor)
    }
}
