use std::path::PathBuf;

use cgraph::mpmc::Sender;
use cgraph::nodes::ComputeNode;

/// Read files in a directory in order (e.g. 0.pcm, 1.pcm, ...) and create new streams for each
/// file so we can interleave the results.
struct ReadPcmDirectory<T: Copy> {
    /// The bit alignment we want to maintain (e.g. 16 for 16-bit integers).
    pub alignment: usize,
    /// Path to the directory being read from
    pub path: PathBuf,
    /// Split the data by channel, will get merged later
    pub channels: Vec<Sender<Vec<T>>>,
}

impl<T: Copy> ComputeNode for ReadPcmDirectory<T> {
    fn name(&self) -> &str {
        "Read PCM Directory"
    }

    fn run(&self) {}
}
