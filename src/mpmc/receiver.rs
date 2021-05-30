use std::sync::Arc;

use super::{ChannelError, Buffer};
use std::ops::Deref;

pub trait ChannelReceiver {
    type Item: Clone;

    /// Get the (buffer id, cursor id) for this receiver.
    /// 
    /// Receivers which share neither the buffer id nor the cursor id are in no way related to
    /// each other.
    /// 
    /// Any receivers which share the same cursor id  and buffer id will also share the same data
    /// stream, splitting data between recv calls and not having to wait on each other.
    /// 
    /// Any receivers which share the same buffer id but not the same cursor id will both
    /// read the same items in the stream as each other and will have to wait on each other.
    fn id(&self) -> (usize, usize);

    /// Receive the next item from the queue, sleeping this thread until there is data automatically
    /// if no data is present at the time of calling.
    fn recv(&self) -> Result<Self::Item, ChannelError>;

    /// Attempt to retrieve the next item from the queue, if no data is present, return None instead
    /// of sleeping the thread.
    fn try_recv(&self) -> Result<Option<Self::Item>, ChannelError>;
}

pub struct Receiver<T: Clone> {
    buffer: Arc<Buffer<T>>,
    id: usize,
}

/// Make another reader of the same underlying data starting where this reader currently is but
/// allowing both readers to independently read the same data.
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

impl<T: Clone> ChannelReceiver for Receiver<T> {
    type Item = T;
    
    fn id(&self) -> (usize, usize) {
        (self.buffer.id(), self.id)
    }

    fn recv(&self) -> Result<T, ChannelError> {
        self.buffer.recv(self.id)
    }
    
    fn try_recv(&self) -> Result<Option<T>, ChannelError> {
        self.buffer.try_recv(self.id)
    }
}

impl<T: Clone> Receiver<T> {
    pub(super) fn new(buffer: Arc<Buffer<T>>) -> Self {
        let id = buffer.new_receiver().unwrap();
        Self { buffer, id }
    }
}


/// SharedReceivers use the same underlying cursor allowing them to take a single Receiver instance
/// and distribute the data between its instances instead of retuning duplicates for each instances
/// as the underlying receiver does.
#[derive(Clone)]
pub struct SharedReceiver<T: Clone> {
    rx: Arc<Receiver<T>>
}

impl<T: Clone> From<Receiver<T>> for SharedReceiver<T> {
    fn from(rx: Receiver<T>) -> Self {
        Self { rx: Arc::new(rx) }
    }
}

impl<T: Clone> Deref for SharedReceiver<T> {
    type Target = Receiver<T>;

    fn deref(&self) -> &Self::Target {
        &*self.rx
    }
}

impl<T: Clone> SharedReceiver<T> {
    pub fn try_unwrap(self: Self) -> Result<Receiver<T>, Self> {
        match Arc::try_unwrap(self.rx) {
            Ok(rx) => Ok(rx),
            Err(rx) => Err(Self { rx })
        }
    }
}
