use std::{fmt::Display, str::FromStr};

use alloy_primitives::Bytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

pub(crate) fn serialize_as_string<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer,
{
    serializer.collect_str(value)
}

#[allow(clippy::ref_option)]
pub(crate) fn serialize_optional_as_string<T, S>(
    value: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer,
{
    value
        .as_ref()
        .map(ToString::to_string)
        .serialize(serializer)
}

pub(crate) fn serialize_all_as_strings<T, S>(values: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    T: Display,
    S: Serializer,
{
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .serialize(serializer)
}

pub(crate) fn serialize_bytes<S>(value: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{value:#x}"))
}

pub(crate) fn deserialize_from_string<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(de::Error::custom)
}
