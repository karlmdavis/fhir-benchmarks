//! Exports [Histogram]s to the "Histogram Percentiles Text Export" `.hgrm` format, which looks
//! like:
//!
//! ```text
//!     Value   Percentile   TotalCount 1/(1-Percentile)
//!
//!     0.016     0.000000            1         1.00
//!     0.980     0.100000        47530         1.11
//! ...
//!  9428.991     1.000000       475109          inf
//!     #[Mean    =       25.048, StdDeviation   =      120.097]
//!     #[Max     =     9420.800, Total count    =       475109]
//!     #[Buckets =           27, SubBuckets     =         2048]
//! ```

use anyhow::Result;
use hdrhistogram::Histogram;
use std::fmt::Display;

/// The scaling ratio to apply to values on output.
///
/// I'm not really sure what this value is, honestly, but I see this value used for it here:
/// <https://github.com/wayfair-tremor/tremor-runtime/blob/main/src/offramp/blackhole.rs#L105>
/// (as referenced on
/// <https://github.com/HdrHistogram/HdrHistogram/issues/170#issuecomment-673414548>).
const OUTPUT_VALUE_UNIT_SCALING_RATIO: usize = 5;

/// The number of reporting points per exponentially decreasing half-distance.
///
/// This appears to be the default in the Java version of HDR Histogram's command line application:
/// <https://github.com/HdrHistogram/HdrHistogram/blob/0cedc733914117da7ad803e03254b9183e7a740e/src/main/java/org/HdrHistogram/HistogramLogProcessor.java#L77>.
const PERCENTILE_TICKS_PER_HALF_DISTANCE: u32 = 5;

/// Output histogram data in a format similar to the Java impl's
/// `AbstractHistogram#outputPercentileDistribution`, but gzip'd and Base64-encoded.
pub fn export_to_hgrm_gzip(histogram: &Histogram<u64>) -> Result<String> {
    let mut export: String = String::new();
    export.push_str(&format!(
        "{:>12} {:>OUTPUT_VALUE_UNIT_SCALING_RATIO$} {:>10} {:>14}\n\n",
        "Value",
        "Percentile",
        "TotalCount",
        "1/(1-Percentile)",
        OUTPUT_VALUE_UNIT_SCALING_RATIO = OUTPUT_VALUE_UNIT_SCALING_RATIO + 2 // + 2 from leading "0." for numbers
    ));
    let mut sum = 0;
    for v in histogram.iter_quantiles(PERCENTILE_TICKS_PER_HALF_DISTANCE) {
        sum += v.count_since_last_iteration();
        if v.quantile_iterated_to() < 1.0 {
            export.push_str(&format!(
                "{:12} {:1.*} {:10} {:14.2}\n",
                v.value_iterated_to(),
                OUTPUT_VALUE_UNIT_SCALING_RATIO,
                v.quantile_iterated_to(),
                sum,
                1_f64 / (1_f64 - v.quantile_iterated_to())
            ));
        } else {
            export.push_str(&format!(
                "{:12} {:1.*} {:10} {:>14}\n",
                v.value_iterated_to(),
                OUTPUT_VALUE_UNIT_SCALING_RATIO,
                v.quantile_iterated_to(),
                sum,
                "∞"
            ));
        }
    }

    fn format_extra_data<T1: Display, T2: Display>(
        label1: &str,
        data1: T1,
        label2: &str,
        data2: T2,
    ) -> String {
        format!(
            "#[{:10} = {:12.2}, {:14} = {:12.2}]\n",
            label1, data1, label2, data2
        )
    }

    export.push_str(&format_extra_data(
        "Mean",
        histogram.mean(),
        "StdDeviation",
        histogram.stdev(),
    ));
    export.push_str(&format_extra_data(
        "Max",
        histogram.max(),
        "Total count",
        histogram.len(),
    ));
    export.push_str(&format_extra_data(
        "Buckets",
        histogram.buckets(),
        "SubBuckets",
        histogram.distinct_values(),
    ));

    /*
     * Aesthetically, having a giant blob of escaped text in the output annoys me (which is silly,
     * I know, but it does). Let's clean that up a bit by compressing it and Base64 encoding it.
     */
    let export = {
        use std::io::Write;
        let mut zipper = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        zipper.write_all(export.as_bytes())?;
        let zipped_export: Vec<u8> = zipper.finish()?;
        base64::encode(&zipped_export)
    };

    Ok(export)
}
