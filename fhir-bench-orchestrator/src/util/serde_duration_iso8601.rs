//! A Serde serializer/deserializer for chrono [Duration] instances that uses ISO-8601 formatting.

use chrono::Duration;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{self, Deserialize, Deserializer, Serializer};

pub const NANOS_PER_SEC: i64 = 1_000_000_000;

/// Converts [Duration] instances to ISO-8601 string values, for use in JSON.
///
/// Parameters:
/// * `duration`: the [Duration] instance to be serialized
/// * `serializer`: the Serde [Serializer] to use
///
/// Returns the [Serializer] result.
pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let seconds = duration.num_seconds();
    let nanoseconds = duration.num_nanoseconds().unwrap_or(0) - (seconds * NANOS_PER_SEC);
    let s = format!("PT{}.{}S", seconds, nanoseconds);
    serializer.serialize_str(&s)
}

/// Converts serialized ISO-8601 duration JSON strings back to [Duration] instances.
///
/// Parameters:
/// * `deserializer`: the Serde [Deserializer] to use
///
/// Returns the deserialized [Duration].
pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;

    lazy_static! {
        static ref REGEX_DURATION: Regex = Regex::new("PT(\\d+)\\.(\\d+)S").unwrap();
    }
    match REGEX_DURATION.captures(&text) {
        Some(capture) => {
            let secs = capture[1]
                .parse::<i64>()
                .map_err(serde::de::Error::custom)?;
            let nanos = capture[2]
                .parse::<i64>()
                .map_err(serde::de::Error::custom)?;
            let nanos_total = (secs * NANOS_PER_SEC) + nanos;

            Ok(Duration::nanoseconds(nanos_total))
        }
        None => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Str(&text),
            &"a value in the format: 'PT123.456S'",
        )),
    }
}

/// Unit tests for the [Duration] serializer & deserialzer.
#[cfg(test)]
mod tests {
    use chrono::Duration;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    /// Just used to test Serde against.
    #[derive(Deserialize, Serialize)]
    struct DurationStruct {
        #[serde(with = "super")]
        duration: Duration,
    }

    /// Verifies that [Duration] values serialize as expected.
    #[tracing::instrument(level = "info")]
    #[test_env_log::test(tokio::test)]
    async fn serialize() {
        let expected = json!({
            "duration": "PT1.234S",
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let actual = DurationStruct {
            duration: Duration::nanoseconds(super::NANOS_PER_SEC + 234),
        };
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);
    }

    /// Verifies that [Duration] values deserialize as expected.
    #[tracing::instrument(level = "info")]
    #[test_env_log::test(tokio::test)]
    async fn deserialize() {
        let expected = DurationStruct {
            duration: Duration::nanoseconds(super::NANOS_PER_SEC + 234),
        };
        let actual = json!({
            "duration": "PT1.234S",
        });
        let actual = serde_json::to_string(&actual).unwrap();
        let actual: DurationStruct = serde_json::from_str(&actual).unwrap();
        assert_eq!(expected.duration, actual.duration);
    }
}
