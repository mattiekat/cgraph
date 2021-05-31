use std::path::PathBuf;

use cgraph::mpmc::*;
use cgraph::nodes::ComputeNode;
use std::env;
use std::str::FromStr;

// A couple of easily-changeable configs in case my assumptions are incorrect.
const LITTLE_ENDIAN: bool = true;
type PcmInt = i16;
type PcmFloat = f32;

enum EncodingType {
    Float,
    Int,
}

impl FromStr for EncodingType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "float" => Ok(EncodingType::Float),
            "int" => Ok(EncodingType::Int),
            _ => Err("Could not parse data type"),
        }
    }
}

#[derive(Clone)]
struct PcmData<T> {
    /// Raw data
    raw: Vec<T>,
    /// Source file id
    source_id: u16,
    /// Offset of the first T (by sizeof T) in the input data.
    offset: u64,
}

/// Read files in a directory in order (e.g. 0.pcm, 1.pcm, ...) and create new streams for each
/// file so we can interleave the results.
struct ReadPcmDirectory {
    /// The bit alignment we want to maintain (e.g. 16 for 16-bit integers).
    alignment: usize,
    /// Path to the directory being read from
    path: PathBuf,
}

struct ConvertIntToFloat {
    input: Receiver<PcmData<PcmInt>>,
    output: Sender<PcmData<PcmFloat>>,
}

struct ConvertFloatToInt {
    input: Receiver<PcmData<PcmFloat>>,
    output: Sender<PcmData<PcmInt>>,
}

/// f(x) = x*10^(dB/10)
struct AmplifyLinearSignal<T: Clone> {
    input: Receiver<PcmData<T>>,
    output: Sender<PcmData<T>>,
    decibels: f32,
}

pub fn main() {
    let mut args = env::args().skip(1);
    let input_path = args.next().expect("Expected input path to be specified");
    let channel_count = args
        .next()
        .expect("Expected a channel count to be specified");
    let input_type = args
        .next()
        .map(|v| v.parse::<EncodingType>().unwrap())
        .expect("Expected input data type to be specified as 'float' or 'int'");
    let amplification = args
        .next()
        .map(|v| {
            v.parse::<f32>()
                .expect("Amplification factor could not be parsed")
        })
        .expect("Amplification factor in dB to be specified");
    let output_type = args
        .next()
        .map(|v| v.parse::<EncodingType>().unwrap())
        .expect("Output data type to be specified as 'float' or 'int'");
}
