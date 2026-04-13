<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/logo-jsconic-horizontal-reverse.png">
    <img src="assets/brand/logo-jsconic-horizontal-color.svg" height="60">
  </picture>
</p>

<h3 align="center">Layer 1 Blockchain That Tokenizes Real-World Manufacturing</h3>

<p align="center">
  <a href="#how-it-works">How It Works</a> &middot;
  <a href="#architecture">Architecture</a> &middot;
  <a href="#run-a-node">Run a Node</a> &middot;
  <a href="#technical-paper">Paper</a> &middot;
  <a href="#community">Community</a>
</p>

<p align="center">
  <a href="https://github.com/protosphinx/jsonic/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-CC--BY--SA--4.0-blue" alt="License">
  </a>
</p>

---

## What is Jsonic?

Jsonic is a **Layer 1 blockchain** — like Ethereum or Solana — but instead of rewarding computation or capital, it rewards **real-world manufacturing and commerce**.

Tokens are minted when manufacturers produce goods, sell them, and have those transactions verified on-chain by both parties. The more real business you do with reputable counterparties, the more you earn.

**The core insight**: Jsonic uses **PageRank mathematics** (the same algorithm that powers Google Search) to build a recursive reputation system. Your reputation depends on who you trade with, and their reputation depends on who *they* trade with, all the way down. This makes it mathematically impossible to game the system with fake transactions.

## How It Works

```
1. Manufacturer registers as a DAO (on-chain business identity)
2. Manufacturer produces goods → logs inventory on their side-chain
3. Sells to a buyer (another DAO or consumer) → both sign the transaction
4. POT (Proof of Transaction) validates the deal on both side-chains
5. At Solstice: main-chain collects all verified activity
6. Tokens minted and distributed based on PageRank-weighted reputation
```

**Who gets rewarded and how much?**

| What you do | Impact on reward |
|---|---|
| Sell to 1,000 unique buyers vs. 1 bulk buyer | 1,000 buyers = **much higher** reward (diversity) |
| Your buyers have high reputation | **Higher** reward (PageRank propagation) |
| You trade with established DAOs vs. unknown entities | Established = **higher** reward |
| You create fake DAOs to buy from yourself | **Near-zero** reward (Sybil resistance) |

## Architecture

Jsonic is a **P2P network** — permissionless, anyone can run a node.

```
                    ┌───────────────────────┐
                    │   Heartbeat Service    │  Central clock signal
                    │   (network pulse)      │
                    └──────────┬────────────┘
                               │
           ┌───────────────────┼───────────────────┐
           │                   │                   │
     ┌─────▼─────┐      ┌─────▼─────┐      ┌─────▼─────┐
     │   Node A   │◄────►│   Node B   │◄────►│   Node C   │  P2P mesh
     │  (Miner/   │      │  (Mfg DAO) │      │ (Consumer) │
     │   Trader)  │      │            │      │            │
     └─────┬─────┘      └─────┬─────┘      └───────────┘
           │                   │
    ┌──────┴──────┐     ┌──────┴──────┐
    │ Main-Chain  │     │ Side-Chain  │   Per-DAO ledger
    │ (global     │     │ (DAO B's    │   with inventory,
    │  state)     │     │  balance    │   transactions,
    │             │     │  sheet &    │   and balance sheet
    │             │     │  inventory) │
    └─────────────┘     └─────────────┘
```

**Who runs a node?**

- **Manufacturers** — to log production, track inventory, and earn tokens
- **Traders/Retailers** — to transact with manufacturers and earn reputation
- **Miners** — to validate transactions and earn a share of minted tokens
- **Anyone** — permissionless; install the node and start participating

### Protocol Stack

| Layer | Component | Purpose |
|-------|-----------|---------|
| **Consensus** | Proof of Transaction (POT) | Validates B2B transactions via dual-DAO signature matching |
| **Reputation** | PageRank Engine | Recursive trust propagation across the transaction graph |
| **Ledger** | Side-chains (per-DAO) | Individual balance sheets, inventory tracking, block generation |
| **State** | Main-chain | Global snapshot at Solstice, token minting, network metrics |
| **Network** | Heartbeat | Central clock + P2P node discovery and liveness |
| **Token** | Jsonic Token | Minted proportionally to verified production & sales |

## Key Concepts

| Concept | Description |
|---------|-------------|
| **DAO** | On-chain identity for a real-world business (manufacturer, retailer, etc.) |
| **POT** | Proof of Transaction — validates that transactions are signed by both parties |
| **PageRank** | Reputation algorithm — your score depends recursively on your trading partners' scores |
| **Solstice** | Periodic sync (like end of financial year) when side-chains sync to main-chain and tokens are minted |
| **Materiality** | Threshold for block creation — blocks appear when accumulated transaction value is significant |
| **Heartbeat** | Network clock pulse from central service; nodes confirm liveness each tick |
| **Adrenaline** | Dynamic scaling of Heartbeat frequency based on network load |
| **Anxiety** | Health metric — ratio of invalid to total transactions (lower = healthier) |

## Run a Node

**Requirements**: Rust 1.75+

```bash
# Clone the repository
git clone https://github.com/protosphinx/jsonic.git
cd jsonic

# Build the node
cargo build --release

# Run the demo (simulates full lifecycle: DAO registration → transactions → Solstice → token minting)
cargo run

# Run tests (52 tests covering crypto, POT, PageRank, side-chain, main-chain)
cargo test
```

## Codebase

```
src/
├── core/
│   ├── types.rs        # DAO, Transaction, Block, BalanceSheet, NetworkMetrics
│   ├── crypto.rs       # SHA-256, Ed25519 signing, Merkle trees
│   ├── dao.rs          # DAO registration, invoice/payment creation
│   ├── pot.rs          # Proof of Transaction validation engine
│   ├── reputation.rs   # PageRank reputation graph + Sybil resistance
│   ├── sidechain.rs    # Per-DAO ledger with Materiality block generation
│   ├── mainchain.rs    # Solstice sync, token minting, network metrics
│   └── heartbeat.rs    # Network node: heartbeat loop, POT matching
├── lib.rs
└── main.rs             # Demo: full DAO-to-DAO lifecycle
```

## Technical Paper

The full mathematical specification is in **[paper.md](paper.md)**, covering:

- PageRank reputation algorithm with weighted edges and convergence proofs
- Token minting function and Solstice distribution
- Sybil resistance analysis with formal bounds
- Network dynamics (Anxiety, Adrenaline, Heartbeat)
- Consumer integration via tokenized payment credentials

The original ecosystem whitepaper is also available: **[whitepaper.md](whitepaper.md)**

## Roadmap

- **Inventory tracking** — On-chain production logging and supply chain visibility
- **eInvoicing & Payment Processing** — Invoice generation, payment tracking, and settlement
- **Consumer identity** — Tokenized credit card / UPI / wallet integration
- **P2P networking** — Node discovery, gossip protocol, distributed Heartbeat
- **Smart Contract Expansion** — Extended JVM for custom business logic
- **Cross-chain Interoperability** — Bridges to Ethereum, Solana, and other L1s

## Contributing

We welcome contributions. See the [Contributing Guide](CONTRIBUTING.md).

## Community

- [Substack](https://protosphinx.substack.com/)
- [Twitter](https://twitter.com/protosphinx)
- [Discord](https://discord.gg/EjPJNwNA)

## Security

See our [Security Policy](SECURITY.md) for responsible disclosure guidelines.

## License

[Creative Commons Attribution-ShareAlike 4.0 International](LICENSE)

Copyright 2023–2024 [@protosphinx](https://github.com/protosphinx)
