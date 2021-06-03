//! Generic compute nodes which form the building blocks of a compute graph.

/// Primary building block of a compute graph. Compute nodes are run in their own threads and pull
/// data in from channels and publish to other channels. They can also interact with the console,
/// files, the network, or any other source or sink of data.
///
/// There are two recommended ways of making a given compute node run in parallel.
/// 1. Spin up multiple threads within the `start` function
/// 2. the other is to implement `Clone` and to create multiple instances. This has the downside of
///    making data no longer progress though the pipeline in a guaranteed order.
pub trait ComputeNode: Send {
    /// Get the name of this node for debugging.
    fn name(&self) -> &str;

    /// Start processing input and keep going until all of the input data has been consumed or the
    /// output has been corked. This will be called from a separate thread.
    fn run(&self);
}

// TODO: make a macro to generate variously sized generic nodes.
mod generic_compute_1_1;
pub use generic_compute_1_1::GenericComputeNode_1_1;
