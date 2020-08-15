//! A Serde serializer/deserializer for [Histogram] instances that uses the histogram library's
//! "HistoBlob" serialization format.

use hdrhistogram::serialization::{
    Deserializer as HistogramDeserializer, Serializer as HistogramSerializer,
    V2DeflateSerializer as HistogramSerializerImpl,
};
use hdrhistogram::Histogram;
use serde::{self, Deserialize, Deserializer, Serializer};

/// Converts [Histogram] instances to the histogram library's "HistoBlob" serialization format,
/// for use in JSON.
///
/// Parameters:
/// * `histogram`: the [Histogram] instance to be serialized
/// * `serializer`: the Serde [Serializer] to use
///
/// Returns the [Serializer] result.
pub fn serialize<S>(histogram: &Histogram<u64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut bytes = Vec::new();
    HistogramSerializerImpl::new()
        .serialize(histogram, &mut bytes)
        .map_err(|err| serde::ser::Error::custom(format!("{}", err)))?;
    let bytes_base64 = base64::encode(bytes);

    serializer.serialize_str(&bytes_base64)
}

/// Converts "HistoBlob" JSON strings back to [Histogram] instances.
///
/// Parameters:
/// * `deserializer`: the Serde [Deserializer] to use
///
/// Returns the deserialized [Histogram].
pub fn deserialize<'de, D>(deserializer: D) -> Result<Histogram<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes_base64 = String::deserialize(deserializer)?;
    let bytes: Vec<u8> = match base64::decode(bytes_base64) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err(serde::de::Error::custom(format!("{}", err)));
        }
    };
    let mut bytes_reader = std::io::Cursor::new(&bytes);

    let mut deserializer = HistogramDeserializer::new();
    match deserializer.deserialize(&mut bytes_reader) {
        Ok(histogram) => Ok(histogram),
        Err(err) => Err(serde::de::Error::custom(format!("{}", err))),
    }
}

/// Unit tests for the [Duration] serializer & deserialzer.
#[cfg(test)]
mod tests {
    use anyhow::Result;
    use hdrhistogram::Histogram;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    /// Just used to test Serde against.
    #[derive(Deserialize, Serialize)]
    struct DurationStruct {
        #[serde(with = "super")]
        histogram: Histogram<u64>,
    }

    /// Verifies that [Duration] values serialize as expected.
    #[test]
    fn serialize() -> Result<()> {
        let expected = json!({
            "histogram": "HISTFAAAABx4nJNpmSzMwMDAxAABzFCaEUoz2X+AsQA/awKA",
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let mut actual = DurationStruct {
            histogram: Histogram::<u64>::new(3)?,
        };
        actual.histogram.record(1)?;
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);

        Ok(())
    }

    /// Verifies that [Duration] values deserialize as expected.
    #[test]
    fn deserialize() -> Result<()> {
        let mut expected = DurationStruct {
            histogram: Histogram::<u64>::new(3)?,
        };
        expected.histogram.record(1)?;
        let actual = json!({
            "histogram": "HISTFAAAABx4nJNpmSzMwMDAxAABzFCaEUoz2X+AsQA/awKA",
        });
        let actual = serde_json::to_string(&actual).unwrap();
        let actual: DurationStruct = serde_json::from_str(&actual).unwrap();
        assert_eq!(expected.histogram, actual.histogram);

        Ok(())
    }
}
