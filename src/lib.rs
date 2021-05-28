use std::collections::{HashMap, VecDeque};
use std::iter::FusedIterator;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use crate::data_node::WritePipe;

struct ComputeGraphTemplate {}

pub mod data_node;



trait ComputeNode {
    type Err = &'static str;

    fn name(&self) -> &str;
    fn process(&mut self) -> Result<(), Self::Err>;
}


struct StreamFromFile {
    name: String,
    out: WritePipe<u8>,
}

impl StreamFromFile {
    fn new(path: Path) -> (Self, Arc<data_node<u8>>) {
        let path_name = path.display();
        let data = data_node::new(format!("File Stream {}", path_name), 1024);
        (StreamFromFile { name: format!("File Reader {}", path_name), out: data.writer() }, data)
    }
}

impl ComputeNode for StreamFromFile {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&mut self) -> Result<(), &'static str> {
        let buf: Vec<u8> = todo!();
        self.out.write_stream(buf);
        Ok(())
    }
}


fn make_pipeline() {
    let d1 = data_node::<u8>::new("Raw input", 1024);
    let d1_i = d1.reader();
    let d1_o1 = d1.writer();
    let d1_o2 = d1.writer();
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
