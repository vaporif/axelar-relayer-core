//! Utilities for deserializing some common structures
use std::io::ErrorKind;

use borsh::io::{Read, Result, Write};
use borsh::{BorshDeserialize, BorshSerialize as _};
use chrono::{DateTime, Utc};

/// Serialize [`DateTime<Utc>`]
///
/// # Errors
/// Infallible
pub fn serialize_utc<W: Write>(value: &DateTime<Utc>, writer: &mut W) -> Result<()> {
    let secs = value.timestamp();
    let nsecs = value.timestamp_subsec_nanos();

    // Serialize as a tuple of (seconds, nanoseconds)
    (secs, nsecs).serialize(writer)
}

/// Deserialize [`DateTime<Utc>`]
///
/// # Errors
/// wrong input
pub fn deserialize_utc<R: Read>(reader: &mut R) -> Result<DateTime<Utc>> {
    let (secs, nsecs): (i64, u32) = BorshDeserialize::deserialize_reader(reader)?;
    let datetime = DateTime::from_timestamp(secs, nsecs);
    match datetime {
        Some(datetime) => Ok(datetime),
        None => Err(borsh::io::Error::new(
            ErrorKind::InvalidData,
            "Invalid DateTime timestamp_micros",
        )),
    }
}

/// Serialize [`Option<DateTime<Utc>>`] as `timestamp_micros`
/// use first byte as an Option
///
/// # Errors
/// Infallible
#[allow(clippy::ref_option, reason = "serde requires otherwise")]
pub fn serialize_option_utc<W: Write>(value: &Option<DateTime<Utc>>, writer: &mut W) -> Result<()> {
    match *value {
        Some(dt) => {
            1_u8.serialize(writer)?;
            serialize_utc(&dt, writer)
        }
        None => 0_u8.serialize(writer),
    }
}

/// Serialize [`Option<DateTime<Utc>>`] as `timestamp_micros`
/// use first byte as an Option
///
/// # Errors
/// wrong input: i.e. first byte not 0 or 1
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

#[cfg(test)]
mod tests {
    use borsh::{BorshDeserialize, BorshSerialize};
    use chrono::{DateTime, Utc};

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
}
