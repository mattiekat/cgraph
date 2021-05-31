use std::env;
use std::str::FromStr;

use cgraph::mpmc::*;
use cgraph::nodes::ComputeNode;

// A couple of easily-changeable configs in case my assumptions are incorrect.
const LITTLE_ENDIAN: bool = true;
/// Size of vecs passed along the buffer (in bytes).
const PACKET_SIZE: usize = 1024;
/// Number of pending vecs that can be waiting.
const BUFFER_SIZE: usize = 128;

mod interleave_channels;
mod read_pcm_directory;
mod write_pcm_file;

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

fn convert_int_to_float(data: Vec<i16>) -> Vec<f32> {
    data.into_iter().map(|x| x as f32).collect()
}

fn convert_float_to_int(data: Vec<f32>) -> Vec<i16> {
    data.into_iter().map(|x| x as i16).collect()
}

/// f(x) = x*10^(dB/10)
fn amplify_linear_signal(data: Vec<f32>, db: f32) -> Vec<f32> {
    let factor = f32::powf(10.0, db / 10.0);
    data.into_iter().map(|x| x * factor).collect()
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