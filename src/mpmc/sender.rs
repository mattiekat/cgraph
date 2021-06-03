use std::sync::Arc;

use crate::mpmc::buffer::Buffer;
use crate::mpmc::ChannelError;

/// A generic sender of packets/data for a `mpmc` channel. This is a producer.
pub trait ChannelSender: Clone {
    type Item: Clone;

    /// The buffer id. All ChannelSender instances with the same id are sending to the same buffer.
    fn id(&self) -> usize;

    /// Write data to the internal buffer for the Receivers to read. This will sleep the current
    /// thread if the internal buffer is full and wait until there is room to write.
    fn send(&self, v: Self::Item) -> Result<(), ChannelError>;

    /// Attempt to write data to the internal buffer for the Receivers to read. This will return
    /// Ok(Some(Item)) if there were no errors but the buffer was full, otherwise it will return
    /// Ok(None) if sent successfully.
    fn try_send(&self, v: Self::Item) -> Result<Option<Self::Item>, ChannelError>;

    /// Cork this channel indicating no new information will be send form this sender or any other
    /// senders to the same buffer.
    fn cork(&self);

    /// Check if the channel has been corked and will not accept any new inputs.
    fn is_corked(&self) -> bool;

    /// The number of items not yet processed by receivers.
    fn pending(&self) -> Result<usize, ChannelError>;
}

/// In-memory `ChannelSender` implementation which uses a locking buffer with multiple cursors.
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

impl<T: Clone> ChannelSender for Sender<T> {
    type Item = T;

    fn id(&self) -> usize {
        self.buffer.id()
    }

    fn send(&self, v: T) -> Result<(), ChannelError> {
        self.buffer.send(v)
    }

    fn try_send(&self, v: T) -> Result<Option<T>, ChannelError> {
        self.buffer.try_send(v)
    }

    fn cork(&self) {
        self.buffer.cork()
    }

    fn is_corked(&self) -> bool {
        self.buffer.is_corked()
    }

    fn pending(&self) -> Result<usize, ChannelError> {
        self.buffer.len()
    }
}

impl<T: Clone> Sender<T> {
    pub(super) fn new(buffer: Arc<Buffer<T>>) -> Self {
        buffer.add_sender();
        Self { buffer }
    }
}
