# TezEdge Debugger

## Memory Profiler  

This tool monitors the memory usage of each function of the TezEdge Light Node.

The profiler can track both the memory used by the node itself and by the kernel
for the caching IO of the node.

### How it works

The tool consists of two parts.

#### 1. EBPF loader

The first part is `bpf-memprof-user` binary which has an embedded ebpf module.
It requires superuser permission. When launched, this binary loads the ebpf
module into the kernel and creates the `/tmp/bpf-memprof.sock` socket. The ebpf
module tracks the `exec` syscall to determine which process is the TezEdge node.
That is why `bpf-memprof-user` should be running before the TezEdge node
is launched. If `bpf-memprof-user` is launched when the node is already running,
it will not be able to find the node.

The ebpf module is tracking physical (residential) page allocation and
deallocation, either removing or adding such pages to the IO cache.
Additionally, the ebpf module unwinds the stack during each allocation event
so that the profiler has call-stack virtual addresses.

#### 2. TezEdge memprof binary

The second part is the `tezedge-memprof` binary.
It performs the following tasks:

* Connects to the socket and receives a stream of kernel events.

* Monitors the `/proc/<pid>/maps` file. This file contains descriptions of
each memory area on the address space of the TezEdge node. Among others,
it contains the descriptions of memory areas of the executable code
`light-node` binary and shared libraries used by the node. It allows
translation from the virtual address of the function into filename and offset
in the file where the function is.

* Loads `.symtab` and `.strtab` sections from `light-node` binary and from
shared libraries. It enables the profiler to resolve function names.

* Counts allocated memory and memory used for cache at each function.

* Serves http requests.

### How to run

#### Using docker-compose

* `git clone https://github.com/tezedge/tezedge-debugger.git`

* `cd tezedge-debugger`

* `docker-compose pull && docker-compose up -d`

First two steps are clone source code from github and move to the directory.
The full source core is unneeded. You can take only `docker-compose.yml` file.

Third step is running the TezEdge node along with the memory profiler and
frontend.

Now you can see the result at http://localhost/#/resources/memory in your
browser.

#### Without docker-compose

The application is distributed as a docker image
`simplestakingcom/tezedge-memprof`. The image needs to have privileged
permissions. It also needs `/sys/kernel/debug` and `/proc` directories mapped
from the host system. The application is serving http requests on port `17832`.

For example:

```
docker run --rm --privileged -it -p 17832:17832 -v /proc:/proc:rw -v /sys/kernel/debug:/sys/kernel/debug:rw simplestakingcom/tezedge-memprof:latest
```

In order to determine function names, the memory profiler needs access
to `light-node`

and system shared libraries. The files to which the memory profiler has access
to should be the same files that the Tezedge node is using. That is why
the docker image

`simplestakingcom/tezedge-memprof:latest` is inherited from the `simplestakingcom/tezedge:latest` image.

However, if `tezedge` is updated, but the `tezedge-memprof` image is still old,
it can lead to problems. To avoid such situations, `tezedge-memprof` image has
a docker client inside, and copies the `light-node` binary from the current
`tezedge` container.

Set the `TEZEDGE_NODE_NAME` environment variable into the TezEdge node container
name and map `/var/run/docker.sock` file from host to enable such behavior.

See `docker-compose.yml` and `memprof.sh` for details.

### HTTP API

### `/v1/tree`

Return a tree-like object. Each node of the tree represents a function in some
executable file.

The tree has the following structure:

* `name`

* `executable` - name of the binary file (ELF), for example `libc-2.31.so`

* `offset` - offset of the function call in the binary file

* `functionName` - demangled name of the function, for example
`<networking::p2p::peer::Peer as riker::actor::Receive<networking::p2p::peer::SendMessage>>::receive::hfe17b4d497a1a6cb`,
note: rust function name is ending with hash, for example `hfe17b4d497a1a6cb`

* `functionCategory` - indicates the origin of the function, can be one of
the following:

* `nodeRust` is a function of the TezEdge node written in Rust

* `nodeCpp` is a function of the TezEdge node written in C++

* `systemLib` is a function from a system library, usually written in C,
but it can also be an arbitrary language.

* `value` - positive integer, number of kilobytes of total memory
(ordinary + cache) allocated in this function and all functions from which
this function is called

* `cacheValue` - positive integer, number of kilobytes of cache allocated
in this function

* `frames` - list of branches of the tree, containing all functions from which
this function is called, or containing all functions which are called from this
function (if `reverse` is set in true).

### Parameters

`reverse` - boolean parameter, used to request reversed tree, default value
is `false`;

`threshold` - integer parameter, used to filter out functions which allocate
a smaller amount of memory than some threshold value, default value is `256`.

### `/v1/pid`

Returns the process id of the TezEdge Node process.

## Recorder

Network message recorder for applications running on the Tezos protocol.

### Peer to peer messages

First of all, the recorder should get the raw data from the kernel.

#### BPF module

The recorder using BPF module to intercept network related syscalls.
It intercepts `read`, `recvfrom`, `write`, `sendto`, `bind`, `listen`,
`connect`, `accept` and `close` syscalls. Those syscalls can give a full picture
of network activity of the application. The BPF module configured to know where
the application which we want to record listens incoming connection.
That is needed to determine an applications PID. It listen `bind` attempts from
any PID on the given port. And once we have one, we know the PID. After that,
the BPF module intercepting other syscalls made by this PID. The single recorder
can record multiple applications simultaneously.

The most challenging task here is to send dynamically sized and potentially big
amount of data from the BPF module which works in kernel space to the main part
of the application which works in user space. The BPF restrict heap memory
allocations. But there is an API to create a ring buffer in shared memory.
We use this buffer. Next problem is the size of data sent in the buffer should
be statically known. It should be a constant. The kernel reject a BPF module
which try to allocate a variable sized area in the ring buffer. So we are
allocating a 256 bytes if the data size is in (0, 256] interval, a 512 bytes if
the data is in (256, 512] interval and so on, up to 128 megabytes.
With one exception: if the data send by the application has size 148 bytes,
we send 148 bytes, because it is very frequent case during bootstrap.
It is a length of a block header.

#### Packets, Chunks and Messages
Tezos nodes communicate by exchanging chunked P2P messages over the internet. Each part uses its own "blocks" of data.

#### Packet
Packets are used by the higher layers of TCP/IP models to transport application communication over the internet 
(there are more type of data blocks on lower levels of the model, like ethernet frames, but we do not work with those).
The recorder does not care about such low-level details, packets are processed by the kernel.

#### Chunks
A binary chunk is a Tezos construct, which represents some sized binary block. Each chunk is a continuous memory, with the
first two bytes representing the size of the block. Chunks are send over internet in TCP Packets, but not necessarily one
chunk per packet, and not necessarily the end of the packet is the end of the chunk. The TCP segment can contain multiple
chunks and it split into packets by the kernel, or network hardware which does not know nothing about Tezos chunks. So
the single TCP packet can contain multiple chunk, and can contain few last bytes of some chunk and few first bytes of the next chunk.
It is not easy to handle properly. We need to bufferize received data and cut chunks from the buffer.

#### Message
A message is parsed representation of some node command, but to be able to send them over internet, they must first be serialized into binary blocks of data, which are then converted into Binary Chunks and finally split into packets to be sent over internet. Again, it is not necessary, that single message is split into single binary chunk. It is required
to await enough chunks to deserialize message. 

#### Encryption

The primary feature of the recorder is the ability to decrypt all messages while having access only to the single identity of the local
node.

##### Tezos "handshake"
To establish encrypted connection, Tezos nodes exchange `ConnectionMessages` which contain information about the nodes themselves,
including public keys, nonces, proof-of-stake and node running protocol version(s). The public key is static and is part of
a node's identity, as is proof-of-stake. Nonces are generated randomly for each connection message. After the `ConnectionMessage`
exchange, each node remembers the node it received and the nonce it sent, and creates the "precomputed" key (for speedups), which is
calculated from the local node's private key and remote node's public key. The nonce is a number incremented after each use.

* To encrypt a message, the node uses the nonce sent in its own `ConnectionMessage` and a precomputed key.
* To decrypt a message, the node uses the received nonce and a precomputed key.

For the recorder to decrypt a message that is coming from a remote node to the local running node. It needs to know:
* The local node's private key - which is part of its local identity to which the recorder has access.
* The remote node's public key - which is part of the received `ConnectionMessage` and was captured.
* The remote node's nonce - which is part of the received `ConnectionMessage` and was captured.

But to decrypt a message sent by the local node, it would be necessary to know the private key of the remote node, to which it does not have
access. Fortunately, Tezos is internally using the Curve5519 method, which allows to decrypt a message with the same 
keys which were used for encryption, thus the recorder "just" needs the:
* Local node's private key - which is part of its local identity, to which the recorder has access.
* Remote node's public key - which is part of the received `ConnectionMessage` and was captured.
* Local node's nonce - which is part of the sent `ConnectionMessage` and was captured.

### Node Logs
To capture node logs, the recorder utilizes the "syslog" protocol (which can be easily enabled in the Docker), which,
instead of printing the log into the console, wraps them into the UDP packet and sends them to the server. This should
be handled by the application or the administrator of the application. The recorder runs a syslog server inside, to simply process the generated
logs. This system allows to decouple the recorder from the node, which prevents the recorder from failing if the running node fails, 
preserving all of the captured logs, and potentially information about the failure of the node.

### Storage
Storage is based on RocksDB, utilizing custom [indexes](./src/storage/secondary_index.rs), which
allows field filtering and cursor pagination.

### RPC server
RPC server is based on the [warp crate](https://crates.io/crates/warp). All endpoints are based on cursor-pagination, 
meaning it is simple to paginate real-time data. All data are from local storage

### API

#### `/v2/p2p`
##### Description
Endpoint for checking all P2P communication on running node. 
Messages are always sorted from newest to oldest.
##### Query arguments
* `cursor_id : 64bit integer value` - Cursor offset, used for easier navigating in messages. Default is the last message.
* `limit : 64bit integer value` - Maximum number of messages returned by the RPC. Default is 100 messages.
* `remote_addr : String representing socket address in format "<IP>:<PORT>"` - Filter message belonging to communication with given remote node.
* `incoming : Boolean` - Filter messages by their direction
* `types : comma separated list of types` - Filter messages by given types
* `source_type: "local" or "remote"` - Filter messages by source of the message
##### Example
* `/v2/p2p` - Return last 100 P2P messages
* `/v2/p2p?cursor_id=100&types=connection_message,metadata` - Return all connection and metadata messages from first 100 messages.

#### `/v2/log`
##### Description
Endpoint for checking all captured logs on running node
Messages are always sorted from newest to oldest.
##### Query arguments
* `cursor_id : 64bit integer value` - Cursor offset, used for easier navigating in messages. Default is the last message.
* `limit : 64bit integer value` - Maximum number of messages returned by the RPC. Default is 100 messages.
* `level : string` - Log level, should be on of `trace, debug, info, warn, error`
* `timestamp : string` - Unix timestamp representing time from which the logs are shown.
##### Example
* `/v2/log?level=error` - Return all errors in last one hundred logs,

### Requirements
* Linux kernel 5.11 version or higher.
* Docker
* [Docker compose](https://docs.docker.com/compose/install/)
* (**RECOMMENDED**)  Steps described in Docker [Post-Installation](https://docs.docker.com/engine/install/linux-postinstall/). 

### How to run

First, you must clone this repo.
```bash
git clone https://github.com/simplestaking/tezedge-debugger.git
```

Then change into the cloned directory
```bash
cd tezedge-debugger
```

The easiest way to launch the Debugger is by running it with the included docker-compose file.
```bash
docker-compose pull
docker-compose up
```
