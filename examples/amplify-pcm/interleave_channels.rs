use crate::PACKET_SIZE;
use cgraph::mpmc::{ChannelError, ChannelReceiver, ChannelSender, Receiver, Sender};
use cgraph::nodes::ComputeNode;
use std::mem;

/// Take 1 or more input channels and round-robin their values into a single output. It will read
/// the channels in order and round-robin at the `T` level and not the packet level.
///
/// If packets are of different sizes it will wait for more data from the one which runs out, and if
/// a channel runs out before another, it will drop any data after the last complete round-robining.
pub struct InterleaveChannels<T: Copy> {
    pub channels: Vec<Receiver<Vec<T>>>,
    pub output: Sender<Vec<T>>,
}

impl<T: Copy + Send> ComputeNode for InterleaveChannels<T> {
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
                        if i > 0 {
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

#[cfg(test)]
mod test {
    use super::*;
    use cgraph::mpmc::sync_channel;
    use std::thread;

    #[test]
    fn interleaving() {
        let (ch0_tx, ch0_rx) = sync_channel(1);
        let (ch1_tx, ch1_rx) = sync_channel(1);
        let (out_tx, out_rx) = sync_channel(1);

        let handle = thread::spawn(move || {
            InterleaveChannels::new(vec![ch0_rx, ch1_rx], out_tx).run();
        });

        // end result should be 0 to 200 in combined channel in order.
        for i in 0..10 {
            ch0_tx.send(((i * 10)..((i + 1) * 10)).map(|v| v * 2).collect());
            ch1_tx.send(((i * 10)..((i + 1) * 10)).map(|v| v * 2 + 1).collect());
        }
        ch0_tx.cork();
        ch1_tx.cork();
        handle.join().unwrap();
        let mut next = 0;
        while let Ok(packet) = out_rx.recv() {
            for i in packet {
                assert_eq!(i, next);
                next += 1;
            }
        }
        assert_eq!(next, 200);
    }

    #[test]
    fn interleaving_uneven() {
        let (ch0_tx, ch0_rx) = sync_channel(1);
        let (ch1_tx, ch1_rx) = sync_channel(1);
        let (out_tx, out_rx) = sync_channel(1);

        let handle = thread::spawn(move || {
            InterleaveChannels::new(vec![ch0_rx, ch1_rx], out_tx).run();
        });

        // end result should be 0 to 200 in combined channel in order.
        ch0_tx.send(vec![0]);
        ch1_tx.send(vec![1, 3]);
        ch0_tx.send(vec![2, 4, 6]);
        ch1_tx.send(vec![5]);
        ch0_tx.send(vec![8, 10]);
        ch0_tx.cork();
        ch1_tx.cork();
        handle.join().unwrap();
        let mut next = 0;
        while let Ok(packet) = out_rx.recv() {
            for i in packet {
                assert_eq!(i, next);
                next += 1;
            }
        }
        assert_eq!(next, 6);
    }
}
