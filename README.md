<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/logo-jsconic-horizontal-reverse.png">
    <img src="assets/brand/logo-jsconic-horizontal-color.svg" height="60">
  </picture>
</p>

<h3 align="center">A B2B Blockchain Protocol for Decentralized Autonomous Organizations</h3>

<p align="center">
  <a href="#architecture">Architecture</a> &middot;
  <a href="#key-concepts">Key Concepts</a> &middot;
  <a href="#whitepaper">Whitepaper</a> &middot;
  <a href="#contributing">Contributing</a> &middot;
  <a href="#community">Community</a>
</p>

<p align="center">
  <a href="https://github.com/protosphinx/jsonic/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-CC--BY--SA--4.0-blue" alt="License">
  </a>
</p>

---

## About

Jsonic is a blockchain platform designed for business-to-business (B2B) transactions in the Web3 era. It introduces a novel **Proof of Transaction (POT)** consensus mechanism specifically built for enterprise use cases, enabling businesses to record, validate, and incentivize real-world transactions on-chain.

The platform is built around **Decentralized Autonomous Organizations (DAOs)** — on-chain representations of real-world businesses — each with its own dedicated side-chain that functions as an individual balance sheet and ledger.

## Architecture

Jsonic's architecture consists of three core layers:

```
┌─────────────────────────────────────────────┐
│               Main-Chain                     │
│  Global financial state, periodic Solstice   │
│  snapshots, token minting & distribution     │
├──────────┬──────────┬──────────┬────────────┤
│ DAO-1    │ DAO-2    │ DAO-3    │  DAO-N     │
│ Side-    │ Side-    │ Side-    │  Side-     │
│ Chain    │ Chain    │ Chain    │  Chain     │
│          │          │          │            │
│ Ledger   │ Ledger   │ Ledger   │  Ledger   │
│ Balance  │ Balance  │ Balance  │  Balance  │
│ Sheet    │ Sheet    │ Sheet    │  Sheet    │
└──────────┴──────────┴──────────┴────────────┘
         Jsonic Virtual Machine (JVM)
```

- **Main-chain** — Aggregates and reconciles balances from all DAO side-chains at each Solstice (periodic sync point). Provides a global snapshot of the ecosystem's financial state.
- **Side-chains** — Each DAO maintains its own side-chain for transaction processing, balance sheet management, and ledger keeping. New blocks are generated when accumulated transaction value reaches a Materiality threshold.
- **JVM (Jsonic Virtual Machine)** — Runtime environment on each node for executing smart contracts, processing data, and maintaining consensus.

## Key Concepts

| Concept | Description |
|---------|-------------|
| **DAO** | On-chain equivalent of a real-world business entity |
| **POT** | Proof of Transaction — validates that B2B transactions are well-formed and complete |
| **Solstice** | Periodic sync point (analogous to end of financial year) when side-chain data is consolidated to the main-chain |
| **Materiality** | Threshold that determines when a new block is added to a side-chain based on accumulated transaction value |
| **Heartbeat** | Fixed interval at which each node confirms its liveness to the network |
| **Adrenaline** | Dynamic adjustment of Heartbeat speed based on network transaction volume |

## Whitepaper

The full technical whitepaper is available in this repository:

**[Read the Jsonic Whitepaper](whitepaper.md)**

It covers the complete protocol specification including blockchain architecture, consensus mechanism, tokenomics, DAO valuation methods, ecosystem integration, and the development roadmap.

## Roadmap

- **eInvoicing & Payment Processing** — On-chain invoice generation, payment tracking, and settlement
- **Project Management & Billing** — Decentralized project management with integrated billing
- **Timesheets & Payroll** — On-chain workforce management and payroll processing
- **Cross-chain Interoperability** — Communication with other blockchain networks
- **Smart Contract Expansion** — Extended JVM capabilities for diverse business applications

## Contributing

We welcome contributions to Jsonic. Please see our [Contributing Guide](CONTRIBUTING.md) for details on how to get involved.

## Community

- [Substack](https://protosphinx.substack.com/)
- [Twitter](https://twitter.com/protosphinx)
- [Discord](https://discord.gg/EjPJNwNA)

## Security

If you discover a security vulnerability, please review our [Security Policy](SECURITY.md) for responsible disclosure guidelines.

## License

This work is licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](LICENSE).

Copyright 2023 [@protosphinx](https://github.com/protosphinx)
