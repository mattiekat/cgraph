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

// fn int_to_float_pipeline(in_path: PathBuf, out_path: PathBuf, channel_count: usize, amplification: f32) -> Vec<Box<dyn ComputeNode>> {
//
// }
//
// fn float_to_int_pipeline(in_path: PathBuf, out_path: PathBuf, channel_count: usize, amplification: f32) -> Vec<Box<dyn ComputeNode>> {
//
// }
//
// fn int_to_int_pipeline(in_path: PathBuf, out_path: PathBuf, channel_count: usize, amplification: f32) -> Vec<Box<dyn ComputeNode>> {
//
// }

fn float_to_float_pipeline(input_path: PathBuf, channel_count: usize, amplification: f32) -> Vec<Box<dyn ComputeNode>> {
    let mut nodes = Vec::new();

    // read from file
    let (reader, channels) = ReadPcmDirectory::<f32>::new(input_path, channel_count);
    nodes.push(Box::new(reader));
    todo!()

    // //
    //
    // let (file_out_tx, file_out_rx) = sync_channel(BUFFER_SIZE);
    // let mut interleaver = InterleaveChannels::new(Vec::new(), file_out_tx);
    // for rx in channels {
    //     let (interleaver_tx, interleaver_rx) = sync_channel(BUFFER_SIZE);
    //     let processfn = match output_type {
    //         EncodingType::Float => |v| {
    //             // amplify and convert types if needed
    //             let amped = amplify_linear_signal(v.unwrap(), amplification);
    //             match output_type {
    //                 EncodingType::Float => amped,
    //                 EncodingType::Int => convert_float_to_int(amped)
    //             },
    //             EncodingType::Int =>
    //         }
    //         nodes.push(Box::new(GenericComputeNode_1_1::new("Amplify".into(), rx, interleaver_tx, processfn)));
    //     }
    //     nodes.push(Box::new(interleaver));
    // }
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

    // Admittedly this is more complicated than I wanted it to be, but this is partially the
    // consequence of statically typed components which need to get put together in very different
    // ways. One way to keep this from getting worse as more parts get put together is finding
    // common type convergences between alternative branches and merging that way. The other thing
    // that can be done in some cases is to only support one data type though the pipeline itself.
    // To make it simpler we could just make the pipeline only use floats internally and convert at
    // the boundaries.
    let nodes = match input_type {
        EncodingType::Float => {
            match output_type {
                EncodingType::Float => float_to_float_pipeline(input_path, channel_count, amplification),
                EncodingType::Int => todo!()
            }
        },
        EncodingType::Int => {todo!()}
    };
}
