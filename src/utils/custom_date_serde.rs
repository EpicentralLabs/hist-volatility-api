use chrono::{DateTime, NaiveDate, Utc};
use serde::{self, Deserialize, Deserializer, Serializer};

const FORMAT: &'static str = "%Y-%m-%d";

/// Serialize `DateTime<Utc>` into "YYYY-MM-DD" format.
pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = date.format(FORMAT).to_string();
    serializer.serialize_str(&s)
}

/// Deserialize a "YYYY-MM-DD" string into `DateTime<Utc>`, assuming 00:00:00 time.
pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let date = NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;

    let datetime = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| serde::de::Error::custom("Invalid hour/minute/second"))?;

    Ok(DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct TestStruct {
        #[serde(with = "super")]
        date: DateTime<Utc>,
    }

    #[test]
    fn test_serialize_date() {
        let date = Utc.with_ymd_and_hms(2024, 4, 5, 0, 0, 0).unwrap();
        let serialized = serialize(&date, serde_json::value::Serializer).unwrap();
        assert_eq!(serialized, "2024-04-05");
    }

    #[test]
    fn test_deserialize_valid_date() {
        let json = r#"{ "date": "2024-04-05" }"#;
        let result: TestStruct =
            serde_json::from_str(json).expect("deserialization should have succeeded");

        let expected = Utc.with_ymd_and_hms(2024, 4, 5, 0, 0, 0).unwrap();
        assert_eq!(result.date, expected);
    }

    #[test]
    fn test_deserialize_invalid_date_format() {
        let json = r#"{ "date": "20240405" }"#; // wrong format
        let result: Result<TestStruct, _> = serde_json::from_str(json);

        assert!(result.is_err(), "Expected error for invalid date format");
    }

    #[test]
    fn test_deserialize_invalid_date_value() {
        let json = r#"{ "date": "2024-13-40" }"#; // impossible month and day
        let result: Result<TestStruct, _> = serde_json::from_str(json);

        assert!(result.is_err(), "Expected error for invalid date value");
    }
}
