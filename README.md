# CGraph

A pure-std rust compute graph implementation to solve the problem presented [here](https://github.com/RainwayApp/low-level-homework/blob/909d25160f8d03e82e74744ff1823fd81d56a841/README.md).

## Building and Running
This project is built using `cargo`, full documentation may be found [here](https://doc.rust-lang.org/cargo/).

- Generate docs with `cargo doc` which can then be found in `target/doc/cgraph/index.html`
- Run tests using `cargo test`
- Run `amplify-pcm` with `cargo run --release --example amplify-pcm -- <input dir> <channel count> <"int"/"float" input> <dB amplification> <"int"/"float" output>` (or other application parameters as appropriate)

### Assumptions
When run, integers are assumed to be signed 16-bit values, and floats 32-bit signed values. Signals are assumed to be stored in linear form. Files are assumed to be encoded in little-endian format (though this is easily changed in the code by toggling a flag). Channels are assumed to be provided in the order the interleaving should occur.

## Design Decisions
- It should be fast, but it should prioritize "infinite" scaling over vertical performance within reason since data pipelines can get large and often need to take advantage of many cores and possibly multiple computers to run at scale.
- Every node has uniquely typed inputs and outputs. I made this decision very early on for a couple of reasons: First it is very useful to allow the shape of data to change through a pipeline, second I have seen some libraries in Rust for data processing pipelines but few support any type at each node. The major downside to this approach is that it means it will be messier to create the graph since there is no good way, that I know of at least, to have a higher-order structure managing this with so many types as inputs and outputs at compile time without making it all still be done by hand.
- `mpmc` is a lower-level abstraction which is generic and could be used elsewhere and was created as a response to a few decisions. The end result is a thread-safe, many-to-many communication channel that supports both sharing data between receivers and duplicating it between receivers. This allows it to be a very powerful edge abstraction where you may want to have multiple copies of one node running in parallel (sharing messages) and other cases where you may want to feed the same data into different processing steps such as writing two different audio file formats in the same pipeline (duplicating messages). I was a little concerned about performance early on as I expected that the signaling for other threads to start would yield the current thread, but actually in testing I discovered that they will keep going after sending the signal and will probably use the entire buffer if it is not large enough which is a good optimization for reducing context switching.
    - Channels should be efficient with memory utilization. Since multiple nodes may consume the same data, a common pattern is to simply make a copy for each one. The goal here is to share a buffer so even if there are hundreds of unique readers, you are able to have a very small memory footprint and do not have run away memory use if one consumer gets behind.
    - Channels must support many-to-many communication.
    - Channels need to support a competing consumers pattern. Without this the only way to scale up would be to create an entire copy of the pipeline, however, introducing this does introduce the notion of unordered messages as competing consumers do not guarantee sending messages in order (at this time).
    - Backpressure to slow down producers if consumers get behind. Would not want to crash in production because of a run away memory issue.
- Every node is computed in one or more threads of its own. This allows treating the pipeline like a microservice cluster and in-fact makes this pipeline capable of spanning multiple computers or processes on one system using an alternative form of `ChannelSender` and `ChannelReceiver`.
- Avoid excessive generics in the example implementation. For an example of a very generic compute node, checkout GenericCompute_1_1. In the example, there is little value in introducing so many generic parameters such as for `Sender` and `Receiver` types since the types are well-defined and if they need to change, it is not the type of application where other code will directly depend on it being generic but just its output. Further, supporting end to end i16 and f32 permutations is painful, I foolishly tried only to realize that I wanted to do the amplification process in f32 anyway.

## Future Work

- The next steps for this project would be to make a macro to generate more generic compute nodes so it is really easy to just plug a custom function in and then get up and going without as much boilerplate.
- Python bindings and a couple helpful wrappers which would allow rapid construction of compute graphs for one-off tasks.
- Support more read and write modes to improve efficiency of smaller data types (required if we want to stop allocating and de-allocating a bunch of Vecs as we go)
- Create additional pre-defined compute nodes for common tasks
- Pipeline debugging and analytic tools for identifying choke points and seeing throughput
- Support mpmc over different backends such as IPC and Redis/RabbitMQ/Nats.
- Message re-ordering within channels to support competing consumers pattern automatically
- Reduce duplication between data storage types using something like num-traits

## Inspiration and Resources
- Official Rust Docs were a constant source of information for this project
- [Tensorflow](https://www.tensorflow.org) which gave me a lot of inspiration for the compute graph design
- [Library of Congress](https://www.loc.gov/preservation/digital/formats/fdd/fdd000016.shtml) page on PCM formats
- [PySDR](https://pysdr.org/index.html) which provides lots of useful reference information from a programmer's perspective on sound
