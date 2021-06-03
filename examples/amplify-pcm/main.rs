use std::str::FromStr;
use std::{env, thread, io};

use crate::interleave_channels::InterleaveChannels;
use crate::read_pcm_directory::ReadPcmDirectory;
use crate::write_pcm_stdout::WritePcmStdout;
use cgraph::mpmc::*;
use cgraph::nodes::{ComputeNode, GenericComputeNode_1_1};

// A couple of easily-changeable configs in case my assumptions are incorrect.
const LITTLE_ENDIAN: bool = true;
/// Size of vecs passed along the buffer (in bytes).
const PACKET_SIZE: usize = 4 * 1024;
/// Number of pending vecs that can be waiting.
const BUFFER_SIZE: usize = 128;

mod interleave_channels;
mod read_pcm_directory;
mod write_pcm_stdout;

#[derive(Copy, Clone, Debug)]
pub enum EncodingType {
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

/// f(x) = x*10^(dB/10)
fn amplify_linear_signal(data: Vec<f32>, db: f32) -> Vec<f32> {
    let factor = f32::powf(10.0, db / 10.0);
    data.into_iter().map(|x| x * factor).collect()
}

pub fn main() {
    let mut args = env::args().skip(1);
    let input_path = args
        .next()
        .expect("Expected input path to be specified")
        .into();
    let channel_count = args
        .next()
        .expect("Expected a channel count to be specified")
        .parse::<usize>()
        .expect("Unable to parse number of expected channels");
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

    // Construct the compute graph
    let mut nodes: Vec<Box<dyn ComputeNode>> = Vec::new();
    let (reader, channels) = ReadPcmDirectory::new(input_path, channel_count, input_type);
    nodes.push(Box::new(reader));
    let amplified_channels: Vec<_> = channels
        .into_iter()
        .map(|channel| {
            let (amp_tx, amp_rx) = sync_channel(BUFFER_SIZE);
            let amplifier =
                GenericComputeNode_1_1::new("Amplifier".into(), channel, amp_tx, move |v| {
                    Some(amplify_linear_signal(v.unwrap(), amplification))
                });
            nodes.push(Box::new(amplifier));
            amp_rx
        })
        .collect();
    let (interleaved_tx, interleaved_rx) = sync_channel(BUFFER_SIZE);
    nodes.push(Box::new(InterleaveChannels::new(
        amplified_channels,
        interleaved_tx,
    )));
    nodes.push(Box::new(WritePcmStdout::new(
        interleaved_rx,
        output_type,
    )));

    // And the fun part... Run it!
    let handles: Vec<_> = nodes
        .into_iter()
        .map(|node| thread::spawn(move || node.run()))
        .collect();
    for handle in handles {
        handle.join().unwrap()
    }
}
