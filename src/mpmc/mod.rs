//! Multiple producer multiple consumer channels.
//! These channels serve the following purposes:
//!  - Reduce memory duplication by using a single buffer from which all consumers read and only
//!    removes the data once it has been read by all consumers
//!  - Makes consumer threads wait for new data if none is ready
//!  - Makes producer threads wait (backpressure) if any one consumer is getting behind.
//!
//! At this time an unbounded channel is not implemented, but could be added as well.

use std::sync::Arc;
mod buffer;
use buffer::Buffer;

pub enum ChannelError {
    IsCorked,
}

pub struct Sender<T> {
    buffer: Arc<Buffer<T>>,
}

/// Create a new sender which will append to the same buffer.
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.buffer.add_sender();
        Self {
            buffer: self.buffer.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.buffer.remove_sender() == 0 {
            // since the buffer is only held within the senders and this was the last sender, it is
            // time to cork it off.
            self.buffer.cork()
        }
    }
}

impl<T> Sender<T> {
    fn new(buffer: Arc<Buffer<T>>) -> Self {
        Self { buffer }
    }

    pub fn send(&self, v: T) -> Result<(), ChannelError> {
        self.buffer.send(v)
    }
}

pub struct Receiver<T> {
    buffer: Arc<Buffer<T>>,
    id: usize,
}

/// Make another reader of the same underlying data starting where this reader currently is.
impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self::new(self.buffer.clone())
    }
}

/// No longer wait for this receiver to consume data
impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.buffer.drop_receiver(self.id)
    }
}

impl<T> Receiver<T> {
    fn new(buffer: Arc<Buffer<T>>) -> Self {
        let id = buffer.new_receiver();
        Self { buffer, id }
    }

    pub fn recv(&self) -> Result<T, ChannelError> {
        self.buffer.recv(self.id)
    }
}

/// Create a new multiple-producer, multiple-consumer channel. It highly recommended that `T` is a
/// suitably large data packet for efficiency.
pub fn sync_channel<T>(bound: usize) -> (Sender<T>, Receiver<T>) {
    todo!()
}
