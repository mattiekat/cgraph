use std::fmt::{self, Debug, Formatter};
use crate::mpmc::*;

pub mod mpmc;

/// Primary building block of a compute graph. Compute nodes are run in their own threads and pull
/// data in from channels and publish to other channels. They can also interact with the console,
/// files, the network, or any other source or sink of data.
///
/// There are two reccomended ways of making a given compute node run in parallel.
/// 1. Spin up multiple threads within the `start` function
/// 2. the other is to implement `Clone` and to create multiple instances. This has the downside of
///    making data no longer progress though the pipeline in a guaranteed order.
trait ComputeNode {
    /// Get the name of this node for debugging.
    fn name(&self) -> &str;

    /// Start processing input and keep going until all of the input data has been consumed or the
    /// output has been corked. This will be called from a separate thread.
    fn start(&self);
}

// TODO: generate with a macro? It would allow making a compute node with a function really quickly.
struct GenericComputeNode_1_1<I1: Clone, O1: Clone> {
    name: String,
    /// f will always be called with at least one `Some` value and will only start passing `None`
    /// values once that input is exhausted.
    f: fn(Option<I1>) -> (Option<O1>),
    rx1: Receiver<I1>,
    tx1: Sender<O1>,
}

impl<I1: Clone, O1: Clone> Clone for GenericComputeNode_1_1<I1, O1> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            f: self.f,
            rx1: self.rx1.clone(),
            tx1: self.tx1.clone(),
        }
    }
}

impl<I1: Clone, O1: Clone> Debug for GenericComputeNode_1_1<I1, O1> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl<I1: Clone, O1: Clone> ComputeNode for GenericComputeNode_1_1<I1, O1> {
    fn name(&self) -> &str {
        &self.name
    }

    fn start(&self) {
        loop {
            let i1 = match self.rx1.recv() {
                Ok(i1) => Some(i1),
                Err(ChannelError::IsCorked) => None,
                Err(ChannelError::Poisoned) => panic!("Thread was poisoned"),
                Err(ChannelError::BufferFull) => unreachable!(),
            };
            if i1.is_none() {
                // all inputs have been exhausted
                break;
            }
            let (o1) = (self.f)(i1);
            if let Some(o1) = o1 {
                self.tx1.send(o1);
            }
        }
    }
}

impl<I1: Clone, O1: Clone> GenericComputeNode_1_1<I1, O1> {
    fn new(
        name: String,
        rx: (Receiver<I1>),
        tx: (Sender<O1>),
        f: fn(Option<I1>) -> (Option<O1>),
    ) -> Self {
        let (rx1) = rx;
        let (tx1) = tx;
        Self { f, tx1, rx1, name }
    }
}

// struct StreamFromDirectory {
//     name: String,
//     tx: Sender<Vec<u8>>,
//     path: PathBuf
// }
//
// impl StreamFromDirectory {
//     fn new(path: &Path, tx: Sender<Vec<u8>>) -> Self {
//         let path_name = path.display();
//         Self {
//             name: format!("File Reader {}", path_name),
//             tx,
//
//         }
//     }
// }
//
// impl ComputeNode for StreamFromDirectory {
//     type Err = &'static str;
//
//     fn name(&self) -> &str {
//         &self.name
//     }
//
//     fn process(&mut self) -> Result<(), &'static str> {
//         // let buf: Vec<u8> = todo!();
//         // self.out.write_stream(buf);
//         // Ok(())
//         todo!()
//     }
// }

#[cfg(test)]
mod test {
    use super::GenericComputeNode_1_1;
    use super::mpmc::sync_channel;

    #[test]
    fn make_pipeline() {
        let (tx1, rx1) = sync_channel::<Vec<u8>>(16);
        let (tx2, rx2) = sync_channel::<Vec<u16>>(16);
        let n = GenericComputeNode_1_1::new("Test".into(), (rx1), (tx2), |i1| {
            (Some(Vec::new()))
        });

        let n2 = n.clone();
    }


}