use std::sync::{Arc, RwLock};
use std::collections::{HashMap, VecDeque};
use std::iter::FusedIterator;
use std::sync::mpsc::Receiver;

/// Internal structure
struct Buffer<T> {
    data: VecDeque<T>,
    /// Offset is how many bits have items have been dropped from the buffer
    offset: u64,
    /// Positions of all of the read pipes
    cursors: HashMap<u16, u64>,
}

pub struct DataNode<T: Clone> {
    name: String,
    max_buffer_size: usize,
    buffer: RwLock<Buffer<T>>,
    rx: Receiver<T>
}

impl<T: Clone> DataNode<T> {
    pub fn estimate_current_size(&self) -> usize {
        // use a read lock so it is easy to do this without causing
        self.buffer.read().expect("Not poisoned").data.len()
    }

    pub fn new(name: String, max_buffer_size: usize) -> Arc<Self> {
        Arc::new(Self { name, max_buffer_size, buffer: VecDeque::new(), cursors: HashMap::new(), offset: 0 })
    }

    pub fn reader(self: &Arc<Self>) -> ReadPipe<T> {
        // we can do this id == length trick since it's append only
        let id = self.cursors.len();
        debug_assert!(id <= u16::MAX as usize);
        ReadPipe { id: id as u16, node: self.clone() }
    }

    pub fn writer(self: &Arc<Self>) -> WritePipe<T> {
        WritePipe { node: self.clone() }
    }
}

pub struct ReadPipe<T> {
    id: u16,
    node: Arc<data_node<T>>,
}

impl<T> ReadPipe<T> {
    pub fn read(&mut self) -> T { todo!() }
    pub fn read_stream(&mut self) -> impl FusedIterator<Item=T> { todo!() }
}

pub struct WritePipe<T> {
    node: Arc<data_node<T>>,
}

impl<T> WritePipe<T> {
    pub fn write(&mut self, v: T) {
        // need to make sure we yield this thread if we find out the buffer is full and wait until it isn't
        todo!()
    }
    pub fn write_stream(&mut self, itr: impl IntoIterator<Item=T>) { todo!() }
}
