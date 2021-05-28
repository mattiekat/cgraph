use crate::mpmc::ChannelError;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering, AtomicUsize};
use std::sync::{Condvar, Mutex};

/// Lockable inner working components of the buffer
struct BufferInner<T> {
    data: VecDeque<T>,
    bound: usize,
    offset: u64,
    cursors: HashMap<usize, u64>,
}

/// A buffer of data for multiple consumers and producers to work with.
///
/// Corking the buffer means no new data may be added.
/// The buffer should be corked once there are no more senders or when there is no more data.
pub(super) struct Buffer<T> {
    inner: Mutex<BufferInner<T>>,
    on_new_data: Condvar,
    on_data_consumed: Condvar,
    corked: AtomicBool,
    sender_count: AtomicUsize,
}

impl<T> Buffer<T> {
    pub fn new(bound: usize) -> Self {
        Buffer {
            inner: Mutex::new(BufferInner {
                data: VecDeque::with_capacity(bound),
                bound,
                offset: 0,
                cursors: HashMap::new(),
            }),
            on_new_data: Condvar::new(),
            on_data_consumed: Condvar::new(),
            corked: AtomicBool::new(false),
            sender_count: AtomicUsize::new(0)
        }
    }

    /// Write data
    pub fn send(&self, v: T) -> Result<(), ChannelError> {
        todo!()
    }

    pub fn try_send(&self, v: T) -> Result<bool, ChannelError> {
        todo!()
    }

    /// Read a value from the buffer, waiting for new data if this the cursor is already caught up.
    /// Returns None if there is no new data and the buffer has been corked.
    pub fn recv(&self, cursor_id: usize) -> Result<T, ChannelError> {
        todo!()
    }

    /// Attempt to read data but do not wait on more data if it has not been generated yet.
    pub fn try_recv(&self, cursor_id: usize) -> Result<Option<T>, ChannelError> {
        todo!()
    }

    pub fn is_corked(&self) -> bool {
        return self.corked.load(Ordering::Acquire);
    }

    /// Indicate no new data will come into this buffer.
    pub fn cork(&self) {
        self.corked.store(true, Ordering::Release);
        self.on_data_consumed.notify_all()
    }

    pub fn add_sender(&self) {
        self.sender_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Remove a sender and return the new number of senders.
    pub fn remove_sender(&self) -> usize {
        let prev = self.sender_count.fetch_sub(1, Ordering::AcqRel);
        prev - 1
    }

    pub fn senders(&self) -> usize {
        self.sender_count.load(Ordering::Acquire)
    }

    /// Create a new receiver id
    pub fn new_receiver(&self) -> usize {
        todo!()
    }

    pub fn drop_receiver(&self, id: usize) {
        todo!()
    }
}
