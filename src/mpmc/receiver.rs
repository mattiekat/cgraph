use std::sync::Arc;

use crate::mpmc::buffer::Buffer;
use crate::mpmc::ChannelError;

pub struct Receiver<T: Clone> {
    buffer: Arc<Buffer<T>>,
    id: usize,
}

/// Make another reader of the same underlying data starting where this reader currently is.
impl<T: Clone> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self::new(self.buffer.clone())
    }
}

/// No longer wait for this receiver to consume data
impl<T: Clone> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.buffer.drop_receiver(self.id).unwrap();
    }
}

impl<T: Clone> Receiver<T> {
    pub(super) fn new(buffer: Arc<Buffer<T>>) -> Self {
        let id = buffer.new_receiver().unwrap();
        Self { buffer, id }
    }

    pub fn recv(&self) -> Result<T, ChannelError> {
        self.buffer.recv(self.id)
    }

    pub fn try_recv(&self) -> Result<Option<T>, ChannelError> {
        self.buffer.try_recv(self.id)
    }
}
