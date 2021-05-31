use std::fmt::{self, Debug, Formatter};

use crate::mpmc::{ChannelError, ChannelReceiver, ChannelSender};

use super::ComputeNode;

/// At this time, this is intended to serve as a possible template for a macro to create a series
/// of generic compute nodes that need only take a function and the appropriate channel connections
/// and then can handle the rest of the boilerplate.
#[derive(Clone)]
pub struct GenericComputeNode_1_1<I1, O1, R1, S1> {
    /// f will always be called with at least one `Some` value and will only start passing `None`
    /// values once that input is exhausted.
    f: fn(Option<I1>) -> (Option<O1>),
    rx1: R1,
    tx1: S1,
    name: String,
}

impl<I1, O1, S1, R1> Debug for GenericComputeNode_1_1<I1, O1, S1, R1> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl<I1, O1, S1, R1> ComputeNode for GenericComputeNode_1_1<I1, O1, R1, S1>
where
    I1: Clone,
    O1: Clone,
    S1: ChannelSender<Item = O1>,
    R1: ChannelReceiver<Item = I1>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn start(&self) {
        loop {
            let i1 = match self.rx1.recv() {
                Ok(i1) => Some(i1),
                Err(ChannelError::IsCorked) => None,
                Err(ChannelError::Poisoned) => panic!("Thread was poisoned"),
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

impl<I1, O1, S1, R1> GenericComputeNode_1_1<I1, O1, R1, S1>
where
    I1: Clone,
    O1: Clone,
    S1: ChannelSender<Item = O1>,
    R1: ChannelReceiver<Item = I1>,
{
    pub fn new(name: String, rx: (R1), tx: (S1), f: fn(Option<I1>) -> (Option<O1>)) -> Self {
        let (rx1) = rx;
        let (tx1) = tx;
        Self { f, tx1, rx1, name }
    }
}
