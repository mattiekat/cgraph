//! Multiple producer multiple consumer channels.
//! These channels serve the following purposes:
//!  - Reduce memory duplication by using a single buffer from which all consumers read and only
//!    removes the data once it has been read by all consumers
//!  - Makes consumer threads wait for new data if none is ready
//!  - Makes producer threads wait (backpressure) if any one consumer is getting behind.
//!
//! At this time an unbounded channel is not implemented, but could be added as well.

use std::sync::{Arc, PoisonError};

use crate::fmt;
use buffer::Buffer;
pub use receiver::*;
pub use sender::*;
use std::fmt::{Debug};

mod buffer;
mod receiver;
mod sender;

#[derive(Eq, PartialEq, Debug)]
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
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn pseudo_random_duration() -> Duration {
        // semi-random duration between 0 and 1ms
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        Duration::from_micros((epoch.as_nanos() % 1000) as u64)
    }

    #[test]
    fn non_blocking_one_to_one() {
        let (tx, rx) = sync_channel::<u8>(2);
        assert_eq!(tx.id(), rx.id().0);
        assert_eq!(tx.pending().unwrap(), 0);
        assert_eq!(rx.pending().unwrap(), 0);

        tx.try_send(1).unwrap();
        assert_eq!(tx.pending().unwrap(), 1);
        assert_eq!(rx.pending().unwrap(), 1);

        tx.try_send(2).unwrap();
        assert_eq!(rx.pending().unwrap(), 2);
        assert_eq!(tx.pending().unwrap(), 2);

        assert_eq!(tx.try_send(3).unwrap(), Some(3));

        // make sure the window is moving since we are going past initial window
        assert_eq!(rx.try_recv().unwrap(), Some(1));
        assert_eq!(rx.pending().unwrap(), 1);
        assert_eq!(tx.pending().unwrap(), 1);
        tx.try_send(4).unwrap();
        assert_eq!(tx.try_send(5).unwrap(), Some(5));
        assert_eq!(rx.pending().unwrap(), 2);
        assert_eq!(tx.pending().unwrap(), 2);

        // drain all the way
        assert_eq!(rx.try_recv().unwrap(), Some(2));
        assert_eq!(tx.pending().unwrap(), 1);
        assert_eq!(rx.pending().unwrap(), 1);
        assert_eq!(rx.try_recv().unwrap(), Some(4));
        assert_eq!(tx.pending().unwrap(), 0);
        assert_eq!(rx.pending().unwrap(), 0);
        assert_eq!(rx.try_recv().unwrap(), None);
    }

    #[test]
    fn blocking_one_to_one() {
        let (tx, rx) = sync_channel::<u8>(4);
        assert_eq!(tx.id(), rx.id().0);

        let tx_thread = thread::spawn(move || {
            for i in 1..=100 {
                // slow producer
                thread::sleep(pseudo_random_duration());
                tx.send(i).unwrap();
                assert!(tx.pending().unwrap() <= 4);
            }
            for i in 1..=100 {
                // slow consumer
                tx.send(i).unwrap();
                assert!(tx.pending().unwrap() <= 4);
            }
        });
        thread::sleep(Duration::from_millis(1));
        let rx_thread = thread::spawn(move || {
            for i in 1..=100 {
                // slow producer
                assert_eq!(rx.recv().unwrap(), i);
            }
            for i in 1..=100 {
                // slow consumer
                thread::sleep(pseudo_random_duration());
                assert_eq!(rx.recv().unwrap(), i);
            }
            // will be corked because the Drop function was run for all senders
            assert_eq!(rx.recv(), Err(ChannelError::IsCorked));
        });

        tx_thread.join().unwrap();
        rx_thread.join().unwrap();
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
    fn blocking_many_tx() {
        let (tx1, rx) = sync_channel::<u8>(2);
        let tx2 = tx1.clone();

        let tx1_thread = thread::spawn(move || {
            for i in 1..100 {
                tx1.send(i).unwrap();
            }
        });
        let tx2_thread = thread::spawn(move || {
            for i in 100..=200 {
                tx2.send(i).unwrap();
            }
        });
        let rx_thread = thread::spawn(move || {
            for i in 1..=200 {
                rx.recv().unwrap();
            }
            thread::sleep(Duration::from_millis(5));
            // will be corked because the Drop function was run for all senders
            assert_eq!(rx.try_recv(), Err(ChannelError::IsCorked))
        });
        tx1_thread.join().unwrap();
        tx2_thread.join().unwrap();
        rx_thread.join().unwrap();
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
    fn blocking_many_rx() {
        let (tx, rx1) = sync_channel::<u8>(2);
        let rx2 = rx1.clone();

        let tx_thread = thread::spawn(move || {
            for i in 1..=200 {
                tx.send(i).unwrap();
            }
        });
        let rx1_thread = thread::spawn(move || {
            for i in 1..=200u8 {
                assert_eq!(rx1.recv().unwrap(), i);
            }
            thread::yield_now();
            assert_eq!(rx1.recv(), Err(ChannelError::IsCorked))
        });
        let rx2_thread = thread::spawn(move || {
            for i in 1..=200u8 {
                assert_eq!(rx2.recv().unwrap(), i);
            }
            thread::yield_now();
            assert_eq!(rx2.recv(), Err(ChannelError::IsCorked))
        });
        tx_thread.join().unwrap();
        rx1_thread.join().unwrap();
        rx2_thread.join().unwrap();
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

    #[test]
    fn blocking_shared_rx() {
        let (tx, rx1) = sync_channel::<u8>(2);
        let rx1 = SharedReceiver::from(rx1);
        let rx2 = rx1.clone();

        let tx_thread = thread::spawn(move || {
            for i in 1..=200 {
                tx.send(i).unwrap();
            }
            // tx.cork();
        });
        let rx1_thread = thread::spawn(move || {
            let mut count: usize = 0;
            while let Ok(_) = rx1.recv() {
                count += 1;
                thread::yield_now();
            }
            assert_eq!(rx1.recv(), Err(ChannelError::IsCorked));
            count
        });
        let rx2_thread = thread::spawn(move || {
            let mut count: usize = 0;
            while let Ok(_) = rx2.recv() {
                count += 1;
                thread::yield_now();
            }
            assert_eq!(rx2.recv(), Err(ChannelError::IsCorked));
            count
        });
        tx_thread.join().unwrap();
        let c1 = rx1_thread.join().unwrap();
        let c2 = rx2_thread.join().unwrap();
        assert_eq!(c1 + c2, 200);
    }
}
