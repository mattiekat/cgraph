use cgraph::mpmc::{ChannelError, ChannelReceiver, ChannelSender, Receiver, Sender};
use cgraph::nodes::ComputeNode;
use crate::PACKET_SIZE;
use std::mem;

pub struct InterleaveChannels<T: Copy> {
    pub channels: Vec<Receiver<Vec<T>>>,
    pub output: Sender<Vec<T>>,
}

impl<T: Copy> ComputeNode for InterleaveChannels<T> {
    fn name(&self) -> &str {
        "Interleave Channels"
    }

    fn run(&self) {
        let num_channels = self.channels.len();
        let mut buffers = Vec::with_capacity(num_channels);
        let mut output_buffer: Vec<T> = Vec::with_capacity(PACKET_SIZE);

        // initialize
        for _ in 0..num_channels {
            buffers.push(Vec::new().into_iter());
        }
        // interleave over samples
        'outer: loop {
            if (output_buffer.len() + num_channels) * mem::size_of::<T>() > PACKET_SIZE {
                // flush because it would overflow if we don't
                let mut tbuf = Vec::with_capacity(num_channels);
                std::mem::swap(&mut tbuf, &mut output_buffer);
                self.output.send(tbuf).unwrap();
            }
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
                        debug_assert!(!new_buf.is_empty());
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
        }
        if !output_buffer.is_empty() {
            self.output.send(output_buffer).unwrap();
        }
        self.output.cork();
    }
}

impl<T: Copy> InterleaveChannels<T> {
    pub fn new(channels: Vec<Receiver<Vec<T>>>, output: Sender<Vec<T>>) -> Self {
        Self { channels, output }
    }
    
    pub fn add_input_channel(&mut self, rx: Receiver<Vec<T>>) {
        self.channels.push(rx)
    }
}
