use std::io::{Stdout, Write};

use cgraph::mpmc::{ChannelReceiver, Receiver};
use cgraph::nodes::ComputeNode;

use crate::{EncodingType, LITTLE_ENDIAN, PACKET_SIZE};

/// Write a stream of PCM data to a file.
pub struct WritePcmStdout {
    channel: Receiver<Vec<f32>>,
    write_type: EncodingType,
}

impl ComputeNode for WritePcmStdout {
    fn name(&self) -> &str {
        "Write PCM to Stdout"
    }

    fn run(&self) {
        match self.write_type {
            EncodingType::Float => self.write_i16(),
            EncodingType::Int => self.write_f32(),
        }
    }
}

impl WritePcmStdout {
    pub fn new(channel: Receiver<Vec<f32>>, write_type: EncodingType) -> Self {
        Self {
            channel,
            write_type,
        }
    }

    fn write_i16(&self) {
        let stdout = std::io::stdout();
        let mut file = stdout.lock();
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
                    (v as i16).to_le_bytes()
                } else {
                    (v as i16).to_be_bytes()
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
            file.flush().expect("Error writing to output file.");
        }
    }

    fn write_f32(&self) {
        let stdout = std::io::stdout();
        let mut file = stdout.lock();
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
            file.flush().expect("Error writing to output file.");
        }
    }
}
