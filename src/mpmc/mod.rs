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
    fn non_blocking_one_to_one() {
        let (tx, rx) = sync_channel::<u8>(2);
        assert_eq!(tx.id(), rx.id().0);

        tx.try_send(23).unwrap();
        tx.try_send(81).unwrap();
        assert_eq!(tx.try_send(12).unwrap(), Some(12));
        assert_eq!(rx.try_recv().unwrap(), Some(23));

        // make sure the window is moving since we are going past initial window
        tx.try_send(42).unwrap();
        assert_eq!(tx.try_send(58).unwrap(), Some(58));
        assert_eq!(rx.try_recv().unwrap(), Some(81));
        assert_eq!(rx.try_recv().unwrap(), Some(42));
        assert_eq!(rx.try_recv().unwrap(), None);
    }

    #[test]
    fn non_blocking_many_tx() {
        let (tx1, rx) = sync_channel::<u8>(2);
        let tx2 = tx1.clone();
        assert_eq!(tx1.id(), tx2.id());
        assert_eq!(tx1.id(), rx.id().0);

        tx1.try_send(1).unwrap();
        tx2.try_send(2).unwrap();

        // both senders recognize the buffer is full
        assert_eq!(tx1.try_send(3).unwrap(), Some(3));
        assert_eq!(tx2.try_send(4).unwrap(), Some(4));

        // messages from both senders were received
        assert_eq!(rx.try_recv().unwrap(), Some(1));
        assert_eq!(rx.try_recv().unwrap(), Some(2));
    }

    #[test]
    fn non_blocking_many_rx() {
        let (tx, rx1) = sync_channel::<u8>(2);
        let rx2 = rx1.clone();
        assert_eq!(tx.id(), rx1.id().0);
        assert_eq!(tx.id(), rx2.id().0);
        assert_ne!(rx1.id().1, rx2.id().1);

        tx.try_send(1).unwrap();
        tx.try_send(2).unwrap();

        // both read same value
        assert_eq!(rx1.try_recv().unwrap(), Some(1));
        assert_eq!(rx2.try_recv().unwrap(), Some(1));
        // moved the window after both have read it
        tx.try_send(3).unwrap();
        // allows one to get ahead
        assert_eq!(rx1.try_recv().unwrap(), Some(2));
        assert_eq!(rx1.try_recv().unwrap(), Some(3));
        assert_eq!(rx1.try_recv().unwrap(), None);
        // did not move the window yet
        assert_eq!(tx.try_send(4).unwrap(), Some(4));
        // other can now read values
        assert_eq!(rx2.try_recv().unwrap(), Some(2));
        assert_eq!(rx2.try_recv().unwrap(), Some(3));
    }

    #[test]
    fn non_blocking_shared_rx() {
        let (tx, rx1) = sync_channel::<u8>(2);
        let rx2 = SharedReceiver::from(rx1.clone());
        let rx3 = rx2.clone();
        // make sure cloning worked correctly
        assert_eq!(tx.id(), rx1.id().0);
        assert_eq!(tx.id(), rx2.id().0);
        assert_eq!(tx.id(), rx3.id().0);
        assert_ne!(rx1.id().1, rx2.id().1);
        assert_eq!(rx2.id().1, rx3.id().1);

        tx.try_send(1).unwrap();
        tx.try_send(2).unwrap();

        // both read same value
        assert_eq!(rx1.try_recv().unwrap(), Some(1));
        assert_eq!(rx2.try_recv().unwrap(), Some(1));
        // moved the window after both have read it
        tx.try_send(3).unwrap();

        // allows one to get ahead
        assert_eq!(rx1.try_recv().unwrap(), Some(2));
        assert_eq!(rx1.try_recv().unwrap(), Some(3));
        assert_eq!(rx1.try_recv().unwrap(), None);
        // did not move the window yet
        assert_eq!(tx.try_send(4).unwrap(), Some(4));
        // rx2 and rx3 share the same cursor
        assert_eq!(rx3.try_recv().unwrap(), Some(2));
        assert_eq!(rx2.try_recv().unwrap(), Some(3));
    }
}
