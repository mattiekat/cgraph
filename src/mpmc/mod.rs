//! Multiple producer multiple consumer channels.
//! These channels serve the following purposes:
//!  - Reduce memory duplication by using a single buffer from which all consumers read and only
//!    removes the data once it has been read by all consumers
//!  - Makes consumer threads wait for new data if none is ready
//!  - Makes producer threads wait (backpressure) if any one consumer is getting behind.
//!
//! At this time an unbounded channel is not implemented, but could be added as well.

use std::sync::{Arc, PoisonError};

use buffer::Buffer;
pub use receiver::*;
pub use sender::*;

mod buffer;
mod receiver;
mod sender;

#[derive(Debug)]
pub enum ChannelError {
    IsCorked,
    Poisoned,
    BufferFull,
}

impl<T> From<PoisonError<T>> for ChannelError {
    fn from(_: PoisonError<T>) -> Self {
        Self::Poisoned
    }
}

/// Create a new multiple-producer, multiple-consumer channel. It highly recommended that `T` is a
/// suitably large data packet for efficiency.
pub fn sync_channel<T: Clone>(bound: usize) -> (Sender<T>, Receiver<T>) {
    let buffer = Arc::new(Buffer::new(bound));
    (Sender::new(buffer.clone()), Receiver::new(buffer))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn make_sync_channel() {
        let (tx, rx) = sync_channel::<u8>(4);
        tx.try_send(23).unwrap();
        tx.try_send(81).unwrap();
        tx.try_send(12).unwrap();
        assert_eq!(rx.try_recv().unwrap().unwrap(), 23);
        assert_eq!(rx.try_recv().unwrap().unwrap(), 81);
        assert_eq!(rx.try_recv().unwrap().unwrap(), 12);

        // make sure the window is moving since we are going past initial window
        tx.try_send(42).unwrap();
        tx.try_send(58).unwrap();
        tx.try_send(36).unwrap();
        assert_eq!(rx.try_recv().unwrap().unwrap(), 42);
        assert_eq!(rx.try_recv().unwrap().unwrap(), 58);
        assert_eq!(rx.try_recv().unwrap().unwrap(), 36);
    }
}
