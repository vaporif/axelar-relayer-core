use std::io::ErrorKind;

use borsh::io::{Read, Result, Write};
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, Utc};

use crate::types::BigInt;

pub fn serialize_utc<W: Write>(value: &DateTime<Utc>, writer: &mut W) -> Result<()> {
    value.timestamp_micros().serialize(writer)
}

pub fn deserialize_utc<R: Read>(reader: &mut R) -> Result<DateTime<Utc>> {
    let timestamp: i64 = BorshDeserialize::deserialize_reader(reader)?;
    let datetime = DateTime::from_timestamp_micros(timestamp);
    match datetime {
        Some(datetime) => Ok(datetime),
        None => Err(borsh::io::Error::new(
            ErrorKind::InvalidData,
            "Invalid DateTime timestamp_micros",
        )),
    }
}

pub fn serialize_option_utc<W: Write>(value: &Option<DateTime<Utc>>, writer: &mut W) -> Result<()> {
    match value {
        Some(dt) => {
            1_u8.serialize(writer)?;
            serialize_utc(dt, writer)
        }
        None => 0_u8.serialize(writer),
    }
}

pub fn deserialize_option_utc<R: Read>(reader: &mut R) -> Result<Option<DateTime<Utc>>> {
    let flag: u8 = BorshDeserialize::deserialize_reader(reader)?;

    match flag {
        0 => Ok(None),
        1 => {
            let datetime = deserialize_utc(reader)?;
            Ok(Some(datetime))
        }
        _ => Err(borsh::io::Error::new(
            ErrorKind::InvalidData,
            "Invalid Option flag byte for Option<DateTime<Utc>>",
        )),
    }
}

pub fn serialize_bigint<W: Write>(value: &BigInt, writer: &mut W) -> Result<()> {
    value.0.to_string().serialize(writer)
}
pub fn deserialize_bigint<R: Read>(reader: &mut R) -> Result<BigInt> {
    let value: String = BorshDeserialize::deserialize_reader(reader)?;
    let number = bnum::types::I512::parse_str_radix(&value, 10);
    Ok(BigInt(number))
}

#[cfg(test)]
mod tests {
    use borsh::{BorshDeserialize, BorshSerialize};
    use chrono::{DateTime, Utc};

    use crate::types::BigInt;

    #[derive(BorshSerialize, BorshDeserialize)]
    struct DateTimeContainer {
        #[borsh(
            serialize_with = "crate::util::serialize_utc",
            deserialize_with = "crate::util::deserialize_utc"
        )]
        pub timestamp: DateTime<Utc>,
    }

    #[derive(BorshSerialize, BorshDeserialize)]
    struct DateTimeOptionContainer {
        #[borsh(
            serialize_with = "crate::util::serialize_option_utc",
            deserialize_with = "crate::util::deserialize_option_utc"
        )]
        pub timestamp: Option<DateTime<Utc>>,
    }

    #[derive(BorshSerialize, BorshDeserialize)]
    struct BigIntContainer {
        #[borsh(
            serialize_with = "crate::util::serialize_bigint",
            deserialize_with = "crate::util::deserialize_bigint"
        )]
        pub value: BigInt,
    }

    #[test]
    fn test_datetime_utc_borsh_serialize_and_deserialize() {
        let now = Utc::now();
        let container = DateTimeContainer { timestamp: now };
        let serialized = borsh::to_vec(&container).expect("serialize utc succeeds");
        let deserialized =
            DateTimeContainer::deserialize(&mut serialized.as_slice()).expect("deserize suceeds");

        assert_eq!(now, deserialized.timestamp);
    }

    #[test]
    fn test_datetime_option_utc_borsh_serialize_and_deserialize() {
        let now = Some(Utc::now());
        let container = DateTimeOptionContainer { timestamp: now };
        let serialized = borsh::to_vec(&container).expect("serialize utc succeeds");
        let deserialized = DateTimeOptionContainer::deserialize(&mut serialized.as_slice())
            .expect("deserize suceeds");

        assert_eq!(now, deserialized.timestamp);

        let container = DateTimeOptionContainer { timestamp: None };
        let serialized = borsh::to_vec(&container).expect("serialize utc succeeds");
        let deserialized = DateTimeOptionContainer::deserialize(&mut serialized.as_slice())
            .expect("deserize suceeds");

        assert_eq!(None, deserialized.timestamp);
    }

    #[test]
    fn test_bigint_borsh_serialize_and_deserialize() {
        let value = BigInt::new(bnum::types::I512::parse_str_radix(
            "423423413123813194728478923748923748923748923749872984732",
            10,
        ));
        let container = BigIntContainer {
            value: value.clone(),
        };
        let serialized = borsh::to_vec(&container).expect("serialize bigint succeeds");
        let deserialized =
            BigIntContainer::deserialize(&mut serialized.as_slice()).expect("deserize suceeds");

        assert_eq!(value, deserialized.value);
    }
}
