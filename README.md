# Ockham: Simplex Consensus Implementation

> A high-performance, partially synchronous blockchain consensus engine built in Rust.

**Ockham** is a "from scratch" implementation of the **Simplex Consensus** protocol. It is designed to prioritize simplicity and latency, achieving optimal confirmation times without the complexity of traditional View Change protocols.

## Key Features

*   **Optimal Optimistic Confirmation**: $3\delta$ (three network hops to finalize).
*   **Optimal Block Time**: $2\delta$.
*   **Simplex Liveness**: Uses a unique "Dummy Block" mechanism.
*   **BLS Signature Aggregation**: Uses `blst` for efficient signature verification.
*   **JSON-RPC API**: Standard interface for external clients.
*   **Graceful Shutdown**: Ensures data integrity upon termination.

## Architecture

The project is structured into modular components:

*   **`consensus`**: The core State Machine. Handles proposals, vote aggregation, and the $3\Delta$ timeout logic.
*   **`types`**: Core data structures including `Block`, `Vote`, and `QuorumCertificate` (QC).
*   **`crypto`**: BLS12-381 cryptography using `blst`. Supports signature aggregation and VRFs.
*   **`network`**: `libp2p` implementation using Gossipsub/Noise.
*   **`storage`**: Persistent storage using `Redb`.
*   **`rpc`**: JSON-RPC server implementation.

## Getting Started

### Prerequisites

*   [Rust Toolchain](https://rustup.rs/) (stable)

### Building

```bash
cargo build --release
```

### Running the Cluster

We provide a script to spin up a local 4-node cluster for demonstration:

```bash
./scripts/test_cluster.sh
```

### JSON-RPC API

Each node exposes a JSON-RPC server.
- Node 0: `http://127.0.0.1:8545`
- Node 1: `http://127.0.0.1:8546`
- ...

Example Query:
```bash
curl -H "Content-Type: application/json" -d '{"jsonrpc":"2.0", "method":"get_status", "params":[], "id":1}' http://127.0.0.1:8545
```

### Running Tests

Run the simulation tests to verify the consensus logic:

```bash
cargo test
```

## Roadmap

This project is being developed in 4 phases:

- [x] **Phase 1: The Core Library**
    - Core data structures (`Block`, `Vote`, `QC`).
    - Consensus State Machine (`SimplexState`).
    - Simulation Tests.

- [x] **Phase 2: The Networked Prototype**
    - `libp2p` integration.
    - Gossipsub configuration.
    - Network-based consensus tests.

- [x] **Phase 3: Cryptography and Storage**
    - [x] Replace mock crypto with `blst` (BLS12-381).
    - [x] `Redb` integration for persistence.
    - [x] Sync protocol.

- [x] **Phase 4: Optimization and Tooling**
    - [x] Signature Aggregation.
    - [x] JSON-RPC API.
    - [x] Graceful Shutdown.
    - [ ] Block Explorer (Moved to separate repo).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
