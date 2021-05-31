use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use cgraph::mpmc::{ChannelReceiver, Receiver};
use cgraph::nodes::ComputeNode;

use crate::{LITTLE_ENDIAN, PACKET_SIZE};

/// Write a stream of PCM data to a file.
pub struct WritePcmFile<T: Copy> {
    path: PathBuf,
    channel: Receiver<Vec<T>>,
}

impl ComputeNode for WritePcmFile<i16> {
    fn name(&self) -> &str {
        "Write PCM File i16"
    }

    fn run(&self) {
        let mut file = File::create(&self.path).expect("Unable to open file for writing");
        let mut buffer = [0u8; PACKET_SIZE];
        let mut cursor = 0usize;
        while let Ok(vals) = self.channel.recv() {
            for v in vals {
                if cursor + 2 > PACKET_SIZE {
                    // if we will overflow the buffer time, flush now
                    file.write_all(&buffer[0..cursor])
                        .expect("Error writing to output file.");
                    cursor = 0;
                }

                let bytes: [u8; 2] = if LITTLE_ENDIAN {
                    v.to_le_bytes()
                } else {
                    v.to_be_bytes()
                };
                buffer[cursor] = bytes[0];
                buffer[cursor + 1] = bytes[1];
                cursor += 2;
            }
        }
        if cursor > 0 {
            // flush anything that remains
            file.write_all(&buffer[0..cursor])
                .expect("Error writing to output file.");
        }
    }
}

impl ComputeNode for WritePcmFile<f32> {
    fn name(&self) -> &str {
        "Write PCM File f32"
    }

    fn run(&self) {
        let mut file = File::create(&self.path).expect("Unable to open file for writing");
        let mut buffer = [0u8; PACKET_SIZE];
        let mut cursor = 0usize;
        while let Ok(vals) = self.channel.recv() {
            for v in vals {
                if cursor + 4 > PACKET_SIZE {
                    // if we will overflow the buffer time, flush now
                    file.write_all(&buffer[0..cursor])
                        .expect("Error writing to output file.");
                    cursor = 0;
                }

                let bytes: [u8; 4] = if LITTLE_ENDIAN {
                    v.to_le_bytes()
                } else {
                    v.to_be_bytes()
                };
                buffer[cursor] = bytes[0];
                buffer[cursor + 1] = bytes[1];
                buffer[cursor + 2] = bytes[2];
                buffer[cursor + 3] = bytes[3];
                cursor += 4;
            }
        }
        if cursor > 0 {
            // flush anything that remains
            file.write_all(&buffer[0..cursor])
                .expect("Error writing to output file.");
        }
    }
}

impl<T: Copy> WritePcmFile<T> {
    pub fn new(path: PathBuf, channel: Receiver<Vec<T>>) -> Self {
        Self { path, channel }
    }
}