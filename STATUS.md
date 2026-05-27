# Jsonic status

## Current state

Reference implementation of the Jsonic Layer 1 protocol in Rust: ~3500 lines
across 9 core modules + an HTTP API + 2 binaries. 68 tests passing in <0.3s.
Persistent full-node state via sled, JSON-RPC server over HTTP, TypeScript SDK,
and MCP stdio server. Demo and RPC server ship as separate binaries
(`jsonic-demo`, `jsonic-rpc`).

## Recently shipped

- **PageRank wired into Solstice token distribution.** `MainChain` now owns a
  cumulative `ReputationGraph`. Each settled invoice adds a `buyer -> seller`
  edge weighted by value. At every Solstice the chain runs PageRank and
  overrides each snapshot's `relevance_score` with `compute_dao_reward`.
- **Trust-floor reward formula.** `compute_dao_reward` uses
  `trust(u) = max(0, PR(u) - baseline)` with `baseline = (1 - d) / N`. Drops
  the count-based diversity multiplier that admitted a Sybil inflation hole.
- **End-to-end Sybil test.** 100 fake DAOs claiming $10M of fabricated volume
  lose to an honest cluster doing $900k of real volume.
- **Whitepaper consolidated.** `paper.md` merged into `whitepaper.md`. Math
  sections updated to the trust-floor formulation; stronger Sybil bound
  proved (constant in k, not O(k/N)).
- **Persistence layer.** `ChainStore` trait with `MemoryStore` and
  `SledStore` impls. Full `JsonicNode` state is now `Serialize/Deserialize`:
  main-chain, registry, side-chains, pending transactions, sequence counters,
  heartbeat tick, and reputation graph. Round-trip tests include sled reopen.
- **JSON-RPC server.** `jsonic-rpc` binary serves an Axum router exposing
  `/health`, `/daos`, `/transactions`, `/heartbeats`, `/blocks/:height`,
  `/metrics`, `/balance/:dao_id`, `/reputation/:dao_id`. Restores full node
  state from sled on startup, persists it on graceful shutdown. Integration
  tests drive the full lifecycle through the HTTP surface.
- **Marketing homepage.** The root route now serves a public Jsonic L1 landing
  page while the protocol API stays available on explicit RPC endpoints.
- **SDK and MCP surface.** Added `@protosphinx/jsonic-sdk`, a TypeScript
  JSON-RPC client, and `@protosphinx/jsonic-mcp`, an MCP stdio server exposing
  health, heartbeat, block, metrics, balance, and reputation tools.
- **Transaction admission hardening.** Nodes now reject unregistered
  counterparties, forged signatures, invalid amounts, and replayed or
  out-of-order sequence numbers before transactions reach side-chains.
- **Repo hygiene.** `jsonic_activity.json` removed; `.gitignore` updated for
  it and for the local sled directory. All em-dashes scrubbed from the docs.
- **Hex encoding cleanup.** Removed the embedded `crypto.rs` helper in favor of
  the standard `hex` crate.

## Open issues

- **Daily-update commit spam still in `git log`.** Scrub via `git filter-repo`
  (destructive, force-push to main, breaks any forks). Pending the
  history-cleanup pass.
- **External activity-log generator still running somewhere.** The
  cron / launchd / external service that produces `jsonic_activity.json`
  lives outside this repo. Ignored by `.gitignore`, but needs to be turned
  off at the source.

## Next up (ranked)

1. **P2P / libp2p layer.** RPC is in; gossip and node discovery are next.
   Currently a single-node HTTP node, not yet a multi-node network.
2. **Adversarial test suite for POT.** Replay attacks, sequence-gap attacks,
   double-spend, forged signatures, side-chain forks, partial-eclipse.
3. **Property tests + fuzz targets.** `proptest` over PageRank invariants,
   `cargo-fuzz` on transaction deserialization and signature verification.
4. **Parametric tokenomics.** Move `BASE_MINT_PER_SOLSTICE`,
   `SOLSTICE_INTERVAL`, `damping`, `materiality` into a `Genesis` /
   `ChainParams` struct loaded from `genesis.toml`.
5. **`.github/` scaffold.** CI (fmt, clippy -D warnings, test, audit),
   dependabot, PR template, issue templates, CODEOWNERS.
6. **Reputation graph pruning.** Cumulative graph grows without bound.
   Sliding window or eigenvector incremental update.

## Reference

- Whitepaper: [`whitepaper.md`](whitepaper.md) (consolidated, v0.3).
- Demo: `cargo run --bin jsonic-demo`.
- RPC server: `cargo run --bin jsonic-rpc`.
- Tests: `cargo test`.
