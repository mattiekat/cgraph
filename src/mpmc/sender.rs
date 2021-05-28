use std::sync::Arc;

use crate::mpmc::buffer::Buffer;
use crate::mpmc::ChannelError;

pub struct Sender<T: Clone> {
    buffer: Arc<Buffer<T>>,
}

/// Create a new sender which will append to the same buffer.
impl<T: Clone> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.buffer.add_sender();
        Self {
            buffer: self.buffer.clone(),
        }
    }
}

impl<T: Clone> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.buffer.remove_sender() == 0 {
            // since the buffer is only held within the senders and this was the last sender, it is
            // time to cork it off.
            self.buffer.cork()
        }
    }
}

impl<T: Clone> Sender<T> {
    fn new(buffer: Arc<Buffer<T>>) -> Self {
        Self { buffer }
    }

    pub fn send(&self, v: T) -> Result<(), ChannelError> {
        self.buffer.send(v)
    }
}
