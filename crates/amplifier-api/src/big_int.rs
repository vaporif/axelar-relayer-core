//! `BigInt` type for blockchain-specific token amounts using i128.

use core::fmt::Display;
use std::io::{Read, Result, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Deserializer, Serialize};

/**
Represents a big integer as a string matching the pattern `^(0|[1-9]\d*)$`.
ref: <https://github.com/axelarnetwork/axelar-eds-mirror/blob/main/axelarons-gmp-api/schema/schema.yaml/>
Uses i128 as the underlying type.
*/
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BigInt(i128);

impl BigInt {
    /// Get underlying value
    #[must_use]
    pub const fn inner(self) -> i128 {
        self.0
    }
}

impl From<i128> for BigInt {
    fn from(value: i128) -> Self {
        Self(value)
    }
}

impl Display for BigInt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for BigInt {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string = self.0.to_string();
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for BigInt {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = <String as serde::Deserialize>::deserialize(deserializer)?;
        let number = string
            .parse::<i128>()
            .map_err(|err| serde::de::Error::custom(format!("Failed to parse i128, err: {err}")))?;
        Ok(Self(number))
    }
}

/// Serialize `BigInt` for Borsh
///
/// # Errors
/// Infallible
#[allow(clippy::trivially_copy_pass_by_ref, reason = "needs by ref")]
pub fn serialize<W: Write>(value: &BigInt, writer: &mut W) -> Result<()> {
    <String as BorshSerialize>::serialize(&value.0.to_string(), writer)
}

/// Deserialize `BigInt` for Borsh
///
/// # Errors
/// wrong input
pub fn deserialize<R: Read>(reader: &mut R) -> Result<BigInt> {
    let value: String = BorshDeserialize::deserialize_reader(reader)?;
    let number = value
        .parse::<i128>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(BigInt(number))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(BorshSerialize, BorshDeserialize)]
    struct BigIntContainer {
        #[borsh(serialize_with = "serialize", deserialize_with = "deserialize")]
        pub value: BigInt,
    }

    #[test]
    fn test_bigint_creation() {
        let bigint: BigInt = BigInt::from(42_i128);
        assert_eq!(bigint.0.to_string(), "42");
    }

    #[test]
    fn test_bigint_serialization() {
        let bigint: BigInt = BigInt::from(12345_i128);
        let serialized = serde_json::to_string(&bigint).unwrap();
        assert_eq!(serialized, "\"12345\"");
    }

    #[test]
    fn test_bigint_deserialization() {
        let json = "\"98765\"";
        let bigint: BigInt = serde_json::from_str(json).unwrap();
        assert_eq!(bigint.0.to_string(), "98765");
    }

    #[test]
    fn test_borsh_serialization() {
        let bigint: BigInt = 999_i128.into();
        let mut buffer = Vec::new();
        serialize(&bigint, &mut buffer).unwrap();

        let deserialized = deserialize(&mut buffer.as_slice()).unwrap();
        assert_eq!(bigint, deserialized);

        // Test with i128::MAX
        let max_bigint: BigInt = i128::MAX.into();
        let mut max_buffer = Vec::new();
        serialize(&max_bigint, &mut max_buffer).unwrap();

        let max_deserialized = deserialize(&mut max_buffer.as_slice()).unwrap();
        assert_eq!(max_bigint, max_deserialized);
    }

    #[test]
    fn test_large_numbers() {
        // Test with a large positive number
        let json = "\"170141183460469231731687303715884105727\""; // i128::MAX
        let bigint: BigInt = serde_json::from_str(json).unwrap();
        assert_eq!(
            bigint.0.to_string(),
            "170141183460469231731687303715884105727"
        );
    }

    #[test]
    fn test_zero() {
        let bigint: BigInt = 0_i128.into();
        assert_eq!(bigint.0.to_string(), "0");

        let serialized = serde_json::to_string(&bigint).unwrap();
        assert_eq!(serialized, "\"0\"");
    }

    #[test]
    fn test_negative_numbers() {
        let bigint: BigInt = (-12345_i128).into();
        assert_eq!(bigint.0.to_string(), "-12345");

        let serialized = serde_json::to_string(&bigint).unwrap();
        assert_eq!(serialized, "\"-12345\"");

        // Test deserialization
        let deserialized: BigInt = serde_json::from_str("\"-12345\"").unwrap();
        assert_eq!(deserialized.0, -12345);
    }

    #[test]
    fn test_i128_specific() {
        let max: BigInt = i128::MAX.into();
        assert_eq!(max.0, i128::MAX);

        let min: BigInt = i128::MIN.into();
        assert_eq!(min.0, i128::MIN);
    }

    #[test]
    fn test_bigint_borsh_serialize_and_deserialize() {
        let value = i128::MAX;
        let container = BigIntContainer {
            value: BigInt::from(value),
        };
        let serialized = borsh::to_vec(&container).expect("serialize bigint succeeds");
        let deserialized =
            BigIntContainer::deserialize(&mut serialized.as_slice()).expect("deserialize succeeds");

        assert_eq!(BigInt::from(value), deserialized.value);
    }
}

