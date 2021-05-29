use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};

use crate::mpmc::ChannelError;
use crate::mpmc::ChannelError::IsCorked;

/// Lockable inner working components of the buffer
struct BufferInner<T> {
    data: VecDeque<T>,
    /// The "true" first index of data since we will use it as a sliding window and don't want the
    /// cursor positions to become invalid when we move it.
    offset: u64,
    /// Ideally we would use a Priority queue which allows updating "priority" based on current
    /// position so we can quickly find the lowest cursor, but since we probably won't have many
    /// cases where there are more than a couple receivers, it would probably be overkill anyway.
    cursors: HashMap<usize, u64>,
    next_cursor_id: usize,
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
    bound: usize,
}

impl<T: Clone> Buffer<T> {
    pub fn new(bound: usize) -> Self {
        Buffer {
            inner: Mutex::new(BufferInner {
                data: VecDeque::with_capacity(bound),
                offset: 0,
                cursors: HashMap::new(),
                next_cursor_id: 0,
            }),
            bound,
            on_new_data: Condvar::new(),
            on_data_consumed: Condvar::new(),
            corked: AtomicBool::new(false),
            sender_count: AtomicUsize::new(0),
        }
    }

    /// Write data
    pub fn send(&self, v: T) -> Result<(), ChannelError> {
        if self.is_corked() {
            return Err(ChannelError::IsCorked);
        }
        {
            // lock scope
            let mut inner = self.inner.lock()?;
            if inner.data.len() < self.bound {
                inner.data.push_back(v);
            } else {
                // we need to unlock this mutex and wait for consumed data before pushing
                let mut zelf = self.on_data_consumed.wait(inner)?;
                if self.is_corked() {
                    return Err(ChannelError::IsCorked);
                }
                zelf.data.push_back(v);
            }
        }

        // we pushed the data so it is time to send an update
        self.on_new_data.notify_all();
        Ok(())
    }

    /// Attempt to write the data, will return the data if the buffer was full instead of waiting.
    pub fn try_send(&self, v: T) -> Result<Option<T>, ChannelError> {
        if self.is_corked() {
            return Err(ChannelError::IsCorked);
        }
        {
            // Lock Scope
            let mut inner = self.inner.lock()?;
            if inner.data.len() > self.bound {
                return Ok(Some(v));
            } else {
                inner.data.push_back(v);
            }
        }
        // we pushed the data so it is time to send an update
        self.on_new_data.notify_all();
        Ok(None)
    }

    /// Read a value from the buffer, waiting for new data if this the cursor is already caught up.
    /// Returns None if there is no new data and the buffer has been corked.
    pub fn recv(&self, cursor_id: usize) -> Result<T, ChannelError> {
        let inner = self.inner.lock()?;
        let cursor = *inner.cursors.get(&cursor_id).expect("Cursor id is invalid");
        let offset = inner.offset;
        let length = inner.data.len() as u64;
        let mut inner = if cursor >= length + offset {
            // no data left to read
            if self.is_corked() {
                return Err(IsCorked);
            }
            let inner = self.on_new_data.wait(inner)?;
            if inner.data.is_empty() {
                debug_assert!(self.is_corked());
                return Err(IsCorked);
            }
            inner
        } else {
            inner
        };
        let v = inner
            .data
            .get((cursor - offset) as usize)
            .expect("Error in cursor arithmetic")
            .clone();
        inner.cursors.insert(cursor_id, cursor + 1);
        self.clean_after_read(inner, cursor_id);
        Ok(v)
    }

    /// Attempt to read data but do not wait on more data if it has not been generated yet.
    pub fn try_recv(&self, cursor_id: usize) -> Result<Option<T>, ChannelError> {
        let mut inner = self.inner.lock()?;
        let cursor = *inner.cursors.get(&cursor_id).expect("Cursor id is invalid");
        let offset = inner.offset;
        let length = inner.data.len() as u64;
        if cursor >= length + offset {
            // no data left to read
            if self.is_corked() {
                Err(IsCorked)
            } else {
                Ok(None)
            }
        } else {
            let v = inner
                .data
                .get((cursor - offset) as usize)
                .expect("Error in cursor arithmetic")
                .clone();
            inner.cursors.insert(cursor_id, cursor + 1);
            self.clean_after_read(inner, cursor_id);
            Ok(Some(v))
        }
    }

    fn clean_after_read(&self, mut inner: MutexGuard<BufferInner<T>>, cursor_id: usize) {
        let cursor = *inner.cursors.get(&cursor_id).unwrap();
        if cursor != inner.offset + 1 {
            // if the cursor (which was just incremented hence offset + 1) is not at the head of
            // the list, we can skip iterating to find anyone else that is
            return;
        }
        if inner.cursors.values().all(|&c| c > inner.offset) {
            // we were the only cursor which was at the beginning of the buffer, so slide
            // the window and notify that there is room for more data.
            inner.data.pop_front();
            inner.offset += 1;
            std::mem::drop(inner);
            // only notify one since otherwise we will will get one new submission from
            // each producer that was waiting
            self.on_data_consumed.notify_one()
        }
    }

    pub fn is_corked(&self) -> bool {
        return self.corked.load(Ordering::Acquire);
    }

    /// Indicate no new data will come into this buffer.
    pub fn cork(&self) {
        self.corked.store(true, Ordering::Release);
        self.on_data_consumed.notify_all();
        self.on_new_data.notify_all();
    }

    pub fn add_sender(&self) {
        println!("Add sender {}", self.sender_count.load(Ordering::Relaxed) + 1);
        self.sender_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Remove a sender and return the new number of senders.
    pub fn remove_sender(&self) -> usize {
        println!("Remove sender {}", self.sender_count.load(Ordering::Relaxed));
        let prev = self.sender_count.fetch_sub(1, Ordering::AcqRel);
        prev - 1
    }

    pub fn senders(&self) -> usize {
        self.sender_count.load(Ordering::Acquire)
    }

    /// Create a new receiver id and cursor at the beginning of the buffer.
    pub fn new_receiver(&self) -> Result<usize, ChannelError> {
        let mut inner = self.inner.lock()?;
        let id = inner.next_cursor_id;
        inner.next_cursor_id += 1;
        let offset = inner.offset;
        inner.cursors.insert(id, offset);
        Ok(id)
    }

    pub fn drop_receiver(&self, id: usize) -> Result<(), ChannelError> {
        self.inner.lock()?.cursors.remove(&id);
        Ok(())
    }
}
