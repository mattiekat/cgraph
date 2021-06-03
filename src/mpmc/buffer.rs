use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};

use crate::mpmc::ChannelError;
use crate::mpmc::ChannelError::IsCorked;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

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
    id: usize,
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
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Get the unique id of this buffer.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Write data to the internal buffer for the Receivers to read. This will sleep the current
    /// thread if the internal buffer is full and wait until there is room to write.
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

    /// Attempt to write data to the internal buffer for the Receivers to read. This will return
    /// Ok(Some(Item)) if there were no errors but the buffer was full, otherwise it will return
    /// Ok(None) if sent successfully.
    pub fn try_send(&self, v: T) -> Result<Option<T>, ChannelError> {
        if self.is_corked() {
            return Err(ChannelError::IsCorked);
        }
        {
            // Lock Scope
            let mut inner = self.inner.lock()?;
            if inner.data.len() < self.bound {
                inner.data.push_back(v);
            } else {
                return Ok(Some(v));
            }
        }
        // we pushed the data so it is time to send an update
        self.on_new_data.notify_all();
        Ok(None)
    }

    /// Receive the next item from the queue, sleeping this thread until there is data automatically
    /// if no data is present at the time of calling.
    pub fn recv(&self, cursor_id: usize) -> Result<T, ChannelError> {
        let mut inner = loop {
            // making this a scope because it will pause during this and vars will change and need
            // to be re-set after
            let inner = self.inner.lock()?;
            let cursor = *inner.cursors.get(&cursor_id).expect("Cursor id is invalid");
            let offset = inner.offset;
            let length = inner.data.len() as u64;
            if cursor >= length + offset {
                // no data left to read
                if self.is_corked() {
                    return Err(IsCorked);
                }
                let inner = self.on_new_data.wait(inner)?;
                if !inner.data.is_empty() {
                    break inner;
                } else if self.is_corked() {
                    return Err(IsCorked);
                } else {
                    // shared cursor situation where we are the looser of the race
                }
            } else {
                break inner;
            }
        };
        // re-set values because they may have changed after waiting
        let offset = inner.offset;
        let cursor = *inner.cursors.get(&cursor_id).expect("Cursor id is invalid");
        let v = inner
            .data
            .get((cursor - offset) as usize)
            .expect("Error in cursor arithmetic")
            .clone();
        inner.cursors.insert(cursor_id, cursor + 1);
        if cursor == offset {
            // if this cursor was at the head of the list it may be time to move the window
            self.move_buffer_window(inner);
        }
        Ok(v)
    }

    /// Attempt to retrieve the next item from the queue, if no data is present, return None instead
    /// of sleeping the thread.
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
            if cursor == offset {
                // if this cursor was at the head of the list it may be time to move the window
                self.move_buffer_window(inner);
            }
            Ok(Some(v))
        }
    }

    /// Move sliding window if possible
    fn move_buffer_window(&self, mut inner: MutexGuard<BufferInner<T>>) {
        if inner.cursors.values().any(|&c| c <= inner.offset) {
            // there is at least one cursor still at the beginning of the buffer so we can't move
            // forward yet.
            return;
        }

        inner.data.pop_front();
        inner.offset += 1;
        std::mem::drop(inner);
        // only notify one since otherwise we will will get one new submission from
        // each producer that was waiting
        self.on_data_consumed.notify_one()
    }

    /// Check if this buffer is no longer accepting new inputs.
    pub fn is_corked(&self) -> bool {
        return self.corked.load(Ordering::Acquire);
    }

    /// Indicate no new data will come into this buffer.
    pub fn cork(&self) {
        self.corked.store(true, Ordering::Release);
        self.on_data_consumed.notify_all();
        self.on_new_data.notify_all();
    }

    /// Register that a new sender exists by incrementing an internal count.
    pub fn add_sender(&self) {
        self.sender_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Remove a sender and return the new number of senders.
    pub fn remove_sender(&self) -> usize {
        let prev = self.sender_count.fetch_sub(1, Ordering::AcqRel);
        prev - 1
    }

    /// Get the current number of registered senders.
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
    
    /// Remove the cursor for a receiver and perform any other necessary cleanup. This buffer will
    /// no longer wait on the provided receiver.
    pub fn drop_receiver(&self, id: usize) -> Result<(), ChannelError> {
        self.inner.lock()?.cursors.remove(&id);
        Ok(())
    }

    /// Current number of pending elements in the buffer.
    pub fn len(&self) -> Result<usize, ChannelError> {
        // TODO: we could store this outside the mutex with an atomic usize
        Ok(self.inner.lock()?.data.len())
    }
}
