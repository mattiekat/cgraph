use std::borrow::{Borrow, BorrowMut};
use std::collections::{HashMap, VecDeque};
use std::iter::{FusedIterator, Iterator};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::collections::hash_map::Entry::Occupied;

/// Internal structure to handle data delivery to multiple consumers which each get copies of the
/// same data and may not consume it at the same rate.
struct Buffer<T> {
    /// Store recently fetched items until all readers have caught up
    data: VecDeque<T>,
    /// Offset is how many bits have items have been dropped from the buffer
    offset: u64,
    /// Positions of all of the read pipes
    cursors: HashMap<u16, u64>,
    inbound_rx: Receiver<T>,
}

pub struct DataNode<T: Clone> {
    name: String,
    buffer: Mutex<Buffer<T>>,
    inbound_tx: Mutex<SyncSender<T>>,
}

impl<T: Clone> DataNode<T> {
    pub fn new(name: String, max_buffer_size: usize) -> Arc<Self> {
        let (tx, rx) = sync_channel(max_buffer_size);
        Arc::new(Self {
            inbound_tx: Mutex::new(tx),
            name,
            buffer: Mutex::new(Buffer {
                data: VecDeque::new(),
                offset: 0,
                cursors: HashMap::new(),
                inbound_rx: rx,
            }),
        })
    }

    pub fn reader(self: &Arc<Self>) -> ReadPipe<T> {
        // we can do this id == length trick since it's append only
        let mut lock = self.buffer.lock().expect("poisoned thread");

        let id = lock.cursors.len();
        debug_assert!(id <= u16::MAX as usize);
        let offset = lock.offset;
        lock.cursors.insert(id as u16, offset);
        ReadPipe {
            id: id as u16,
            node: self.clone(),
        }
    }

    pub fn writer(self: &Arc<Self>) -> WritePipe<T> {
        WritePipe { node: self.clone() }
    }
}

pub struct ReadPipe<T: Clone> {
    id: u16,
    node: Arc<DataNode<T>>,
}

impl<T: Clone> ReadPipe<T> {
    pub fn read(&self) -> T {
        let mut lock = self.node.buffer.lock().expect("poisoned thread");
        if let Occupied(mut cursor_entry) = lock.cursors.entry(self.id) {
            let cursor = *cursor_entry.get();
            if cursor < lock.cursors.len() as u64 + lock.offset {
                // read from pending buffer
                let next = lock.data.get((cursor - lock.offset) as usize).unwrap().clone();
                cursor_entry.insert(cursor + 1);
                return next
            } else {
                // TODO: wait around if the buffer is already quite large
                let next = lock.inbound_rx.recv().expect("Failed to receive from channel");
                lock.data.push_back(next);
                if let Occupied(mut cursor_entry) = lock.cursors.entry(self.id) {
                    cursor_entry.insert(cursor_entry.get() + 1)
                } else {
                    unreachable!()
                }
                return next
            }
        } else {
            unreachable!()
        }
        // TODO: cleanup buffer if there is no one awaiting the data
    }

    // pub fn read_stream(&mut self) -> impl FusedIterator<Item = T> {
    //     todo!()
    // }
}

pub struct WritePipe<T: Clone> {
    node: Arc<DataNode<T>>,
}

impl<T: Clone> WritePipe<T> {
    pub fn write(&self, v: T) {
        // need to make sure we yield this thread if we find out the buffer is full and wait until it isn't
        self.node
            .inbound_tx
            .lock()
            .expect("poisoned thread")
            .send(v)
            .expect("Failed to send");
    }
    pub fn write_stream(&self, itr: impl IntoIterator<Item = T>) {
        todo!()
    }
}
