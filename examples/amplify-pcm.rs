use std::path::PathBuf;

use cgraph::mpmc::*;
use cgraph::nodes::ComputeNode;
use std::collections::VecDeque;
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

/// Read files in a directory in order (e.g. 0.pcm, 1.pcm, ...) and create new streams for each
/// file so we can interleave the results.
struct ReadPcmDirectory<T: Copy> {
    /// The bit alignment we want to maintain (e.g. 16 for 16-bit integers).
    alignment: usize,
    /// Path to the directory being read from
    path: PathBuf,
    /// Split the data by channel, will get merged later
    channels: Vec<Sender<Vec<T>>>,
}

impl<T: Copy> ComputeNode for ReadPcmDirectory<T> {
    fn name(&self) -> &str {
        "Read PCM Directory"
    }

    fn run(&self) {}
}

struct InterleaveChannels<T: Copy> {
    channels: Vec<Receiver<Vec<T>>>,
    output: Sender<Vec<T>>,
}

impl<T: Copy> ComputeNode for InterleaveChannels<T> {
    fn name(&self) -> &str {
        "Interleave Channels"
    }

    fn run(&self) {
        const TARGET_BUFFER_SIZE: usize = 1024;
        let num_channels = self.channels.len();
        let mut buffers = Vec::with_capacity(num_channels);
        let mut output_buffer: Vec<T> = Vec::with_capacity(TARGET_BUFFER_SIZE + num_channels - 1);

        // initialize
        for _ in 0..num_channels {
            buffers.push(Vec::new().into_iter());
        }
        // interleave over samples
        'outer: loop {
            for i in 0..num_channels {
                // write
                if let Some(v) = buffers[i].next() {
                    // value was at the ready
                    output_buffer.push(v);
                    continue;
                }
                // need to fetch the next data frame
                match self.channels[i].recv() {
                    Ok(new_buf) => {
                        // found the next set of data for this channel
                        debug_assert!(new_buf.len() > 0);
                        buffers[i] = new_buf.into_iter();
                        output_buffer.push(buffers[i].next().unwrap());
                    }
                    Err(ChannelError::IsCorked) => {
                        // found the end of data for this channel
                        if i + 1 < num_channels {
                            // we want 1 record for all channels, so since this one is done
                            // and there are more channels we are not merging, backtrack to the
                            // previous sample which was full.
                            for _ in 0..i {
                                output_buffer.pop();
                            }
                        }
                        break 'outer;
                    }
                    Err(ChannelError::Poisoned) => panic!("Poisoned channel"),
                }
            }
            if output_buffer.len() > TARGET_BUFFER_SIZE {
                // flush
                let mut tbuf = Vec::with_capacity(num_channels);
                std::mem::swap(&mut tbuf, &mut output_buffer);
                self.output.send(tbuf).unwrap();
            }
        }
        if !output_buffer.is_empty() {
            self.output.send(output_buffer).unwrap();
        }
        self.output.cork();
    }
}

fn convert_int_to_float(data: Vec<PcmInt>) -> Vec<PcmFloat> {
    data.into_iter().map(|x| x as PcmFloat).collect()
}

fn convert_float_to_int(data: Vec<PcmFloat>) -> Vec<PcmInt> {
    data.into_iter().map(|x| x as PcmInt).collect()
}

/// f(x) = x*10^(dB/10)
fn amplify_linear_signal(data: Vec<PcmFloat>, db: PcmFloat) -> Vec<PcmFloat> {
    let factor = PcmFloat::powf(10.0, db / 10.0);
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
