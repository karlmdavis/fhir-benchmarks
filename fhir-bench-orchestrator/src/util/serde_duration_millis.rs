//! A Serde serializer/deserializer for chrono [Duration] instances that uses ISO-8601 formatting.

use chrono::Duration;
use serde::{self, Deserialize, Deserializer, Serializer};

/// Converts [Duration] instances to millisecond numeric values, for use in JSON. This conversion
/// is lossy: any fractional milliseconds in the [Duration] (i.e. extra nanoseconds) will be
/// discarded.
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
    let milliseconds = duration.num_milliseconds();
    serializer.serialize_i64(milliseconds)
}

/// Converts serialized JSON milliseconds back to [Duration] instances.
///
/// Parameters:
/// * `deserializer`: the Serde [Deserializer] to use
///
/// Returns the deserialized [Duration].
pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let milliseconds = i64::deserialize(deserializer)?;
    Ok(Duration::milliseconds(milliseconds))
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
            "duration": 1000,
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let actual = DurationStruct {
            duration: Duration::milliseconds(1000),
        };
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);
    }

    /// Verifies that [Duration] values deserialize as expected.
    #[tracing::instrument(level = "info")]
    #[test_env_log::test(tokio::test)]
    async fn deserialize() {
        let expected = DurationStruct {
            duration: Duration::milliseconds(1000),
        };
        let actual = json!({
            "duration": 1000,
        });
        let actual = serde_json::to_string(&actual).unwrap();
        let actual: DurationStruct = serde_json::from_str(&actual).unwrap();
        assert_eq!(expected.duration, actual.duration);
    }
}
