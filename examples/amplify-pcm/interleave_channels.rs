use cgraph::mpmc::{ChannelError, ChannelReceiver, ChannelSender, Receiver, Sender};
use cgraph::nodes::ComputeNode;

struct InterleaveChannels<T: Copy> {
    pub channels: Vec<Receiver<Vec<T>>>,
    pub output: Sender<Vec<T>>,
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
