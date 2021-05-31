use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::thread;
use std::thread::JoinHandle;

use cgraph::mpmc::{sync_channel, ChannelSender, Receiver, Sender};
use cgraph::nodes::ComputeNode;

use crate::{PcmInt, BUFFER_SIZE, LITTLE_ENDIAN, PACKET_SIZE};

/// Read files in a directory in order (e.g. 0.pcm, 1.pcm, ...) and create new streams for each
/// file so we can interleave the results.
struct ReadPcmDirectory<T: Copy> {
    /// Path to the directory being read from
    path: PathBuf,
    /// Split the data by channel, will get merged later
    channels: Vec<Sender<Vec<T>>>,
}

impl ComputeNode for ReadPcmDirectory<i16> {
    fn name(&self) -> &str {
        "Read PCM Directory i16"
    }

    fn run(&self) {
        (0..self.channels.len())
            .map(|i| self.read_channel(i))
            .for_each(|thread| thread.join().unwrap());
    }
}

impl ComputeNode for ReadPcmDirectory<f32> {
    fn name(&self) -> &str {
        "Read PCM Directory f32"
    }

    fn run(&self) {
        (0..self.channels.len())
            .map(|i| self.read_channel(i))
            .for_each(|thread| thread.join().unwrap());
    }
}

impl<T: Copy> ReadPcmDirectory<T> {
    pub fn new(path: PathBuf, channels: usize) -> (Self, Vec<Receiver<Vec<T>>>) {
        let (senders, receivers) = (0..channels).map(|_| sync_channel(BUFFER_SIZE)).unzip();
        (
            Self {
                channels: senders,
                path,
            },
            receivers,
        )
    }
}

impl ReadPcmDirectory<i16> {
    fn read_channel(&self, i: usize) -> JoinHandle<()> {
        let channel = self.channels[i].clone();
        let file_path = self.path.join(format!("{}.pcm", i.to_string()));
        thread::spawn(move || {
            let mut file = File::open(&file_path).expect("Unable to open file");
            let mut buf = [0u8; PACKET_SIZE];
            loop {
                let mut packet = Vec::with_capacity(PACKET_SIZE / 2);
                let bytes_read = file.read(&mut buf).expect("Error reading from file");
                if bytes_read == 0 {
                    break;
                }
                if bytes_read % 2 > 0 {
                    // we did not get a clean break off
                    panic!("File must be aligned to 16-bit integer sizes")
                }
                for i in 0..(bytes_read / 2) {
                    let stage = [buf[i * 2], buf[i * 2 + 1]];
                    let v = if LITTLE_ENDIAN {
                        i16::from_le_bytes(stage)
                    } else {
                        i16::from_be_bytes(stage)
                    };
                    packet.push(v);
                }
                channel.send(packet).unwrap();
            }
            channel.cork();
        })
    }
}

impl ReadPcmDirectory<f32> {
    fn read_channel(&self, i: usize) -> JoinHandle<()> {
        let channel = self.channels[i].clone();
        let file_path = self.path.join(format!("{}.pcm", i.to_string()));
        thread::spawn(move || {
            let mut file = File::open(&file_path).expect("Unable to open file");
            let mut buf = [0u8; PACKET_SIZE];
            loop {
                let mut packet = Vec::with_capacity(PACKET_SIZE / 4);
                let bytes_read = file.read(&mut buf).expect("Error reading from file");
                if bytes_read == 0 {
                    break;
                }
                if bytes_read % 4 > 0 {
                    // we did not get a clean break off
                    panic!("File must be aligned to 32-bit float sizes")
                }
                for i in 0..(bytes_read / 4) {
                    let stage = [buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]];
                    let v = if LITTLE_ENDIAN {
                        f32::from_le_bytes(stage)
                    } else {
                        f32::from_be_bytes(stage)
                    };
                    packet.push(v);
                }
                channel.send(packet).unwrap();
            }
            channel.cork();
        })
    }
}
