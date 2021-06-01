use std::env;
use std::str::FromStr;

use cgraph::mpmc::*;
use cgraph::nodes::{ComputeNode, GenericComputeNode_1_1};
use crate::read_pcm_directory::ReadPcmDirectory;
use std::path::PathBuf;
use crate::interleave_channels::InterleaveChannels;

// A couple of easily-changeable configs in case my assumptions are incorrect.
const LITTLE_ENDIAN: bool = true;
/// Size of vecs passed along the buffer (in bytes).
const PACKET_SIZE: usize = 4 * 1024;
/// Number of pending vecs that can be waiting.
const BUFFER_SIZE: usize = 128;

mod interleave_channels;
mod read_pcm_directory;
mod write_pcm_file;

#[derive(Copy, Clone, Debug)]
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

pub fn main() {
    let mut args = env::args().skip(1);
    let input_path = args.next().expect("Expected input path to be specified").into();
    let channel_count = args
        .next()
        .expect("Expected a channel count to be specified")
        .parse::<usize>().expect("Unable to parse number of expected channels");
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
