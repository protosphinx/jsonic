<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/logo-jsconic-horizontal-reverse.png">
    <img src="assets/brand/logo-jsconic-horizontal-color.svg" height="60">
  </picture>
</p>

<h1 align="center">Jsonic Whitepaper</h1>
<p align="center"><em>A PageRank-Weighted Proof of Transaction Protocol for B2B and Manufacturing Economies</em></p>

<p align="center">
<strong>Version:</strong> 0.3.0 (consolidated)<br>
<strong>Authors:</strong> <a href="https://github.com/protosphinx">@protosphinx</a><br>
<strong>License:</strong> CC-BY-SA-4.0
</p>

---

## Abstract

Jsonic is a Layer 1 blockchain protocol designed to tokenize real-world business and manufacturing activity. Unlike proof-of-work or proof-of-stake systems that reward computational or capital commitment, Jsonic introduces **Proof of Transaction (POT)**, a consensus mechanism that rewards verified production, sale, and settlement of goods and services between businesses. Token minting is proportional to economically meaningful work: producing inventory, selling it, and having both counterparties acknowledge the transaction on-chain.

Central to Jsonic's economics is a **PageRank-based reputation system** that propagates trust through the cumulative transaction graph. Combined with a trust-floor reward formulation, this defends against Sybil attacks: tokens flow to participants whose buyers have themselves earned trust from other earned-trust participants, rather than to actors who fabricate self-referential transactions.

This document consolidates the original ecosystem whitepaper with the formal mathematical specification of the protocol. It covers the architecture, the PageRank reputation algorithm, the token minting and Sybil-resistance bounds, the network health dynamics (Anxiety, Adrenaline, Heartbeat, Materiality, Solstice), DAO valuation, consumer integration, and the integration roadmap.

A reference implementation in Rust accompanies this paper at [`src/core/`](src/core), and a runnable demo of the full lifecycle is available via `cargo run`.

---

## Table of Contents

- [1. Introduction](#1-introduction)
- [2. Architecture Overview](#2-architecture-overview)
  - [2.1 Definitions](#21-definitions)
  - [2.2 DAOs and Nodes](#22-daos-and-nodes)
  - [2.3 Side-chains](#23-side-chains)
  - [2.4 Main-chain](#24-main-chain)
  - [2.5 Side-chain block structure](#25-side-chain-block-structure)
  - [2.6 Main-chain block structure](#26-main-chain-block-structure)
  - [2.7 Proof of Transaction (POT) mechanism](#27-proof-of-transaction-pot-mechanism)
  - [2.8 Jsonic Virtual Machine (JVM)](#28-jsonic-virtual-machine-jvm)
- [3. Transaction Graph Model](#3-transaction-graph-model)
- [4. PageRank Reputation Algorithm](#4-pagerank-reputation-algorithm)
  - [4.1 Classical PageRank](#41-classical-pagerank)
  - [4.2 Weighted PageRank for Jsonic](#42-weighted-pagerank-for-jsonic)
  - [4.3 Dangling nodes](#43-dangling-nodes)
  - [4.4 Convergence](#44-convergence)
- [5. Edge Weight Function](#5-edge-weight-function)
- [6. Diversity (analytical metric)](#6-diversity-analytical-metric)
- [7. Token Minting Function](#7-token-minting-function)
  - [7.1 Trust-floor reward](#71-trust-floor-reward)
  - [7.2 Solstice distribution](#72-solstice-distribution)
- [8. Sybil Resistance Analysis](#8-sybil-resistance-analysis)
  - [8.1 Attack model](#81-attack-model)
  - [8.2 Bound on the Sybil reward](#82-bound-on-the-sybil-reward)
  - [8.3 Cost-benefit analysis](#83-cost-benefit-analysis)
- [9. Network Dynamics](#9-network-dynamics)
  - [9.1 Anxiety](#91-anxiety)
  - [9.2 Adrenaline and Heartbeat](#92-adrenaline-and-heartbeat)
  - [9.3 Materiality threshold](#93-materiality-threshold)
- [10. DAO Valuation Method](#10-dao-valuation-method)
- [11. Consumer Integration](#11-consumer-integration)
- [12. Properties and Guarantees](#12-properties-and-guarantees)
- [13. Integration with Existing Business Systems](#13-integration-with-existing-business-systems)
- [14. Roadmap and Long-term Vision](#14-roadmap-and-long-term-vision)
- [References](#references)

---

## 1. Introduction

Jsonic is a B2B blockchain protocol for achieving financial inclusion among businesses. The aim is to bring real-world business transactions on-chain through a mechanism of rewards, and to provide a transparent B2B substrate for transactions and contracts. Traditional B2B systems suffer from inefficiencies, high costs, and security vulnerabilities. As businesses become more interconnected, there is a growing need for a scalable platform that can handle complex inter-business transactions with verifiable reputation.

Existing blockchain platforms have shown the potential of smart contracts and decentralized applications, but high transaction costs, scalability constraints, and consensus mechanisms (PoW, PoS) that reward computation or capital rather than economic activity have limited their adoption for business workflows.

Jsonic introduces **Proof of Transaction (POT)**: a consensus mechanism designed for B2B use cases where a transaction is "proved" by being independently acknowledged by both counterparty businesses on-chain. POT is paired with a PageRank-based reputation system so that the *quality* of one's counterparties (not just the volume of one's own activity) determines token rewards.

Jsonic is built around Decentralized Autonomous Organizations (DAOs), each with its own side-chain. Side-chains serve as individual ledgers, ensuring efficient transaction processing and scalability while keeping the main-chain unburdened. Periodic Solstice events synchronize side-chain state to the main-chain and trigger token minting based on a DAO's reputation-weighted activity.

---

## 2. Architecture Overview

Jsonic is a main-chain plus per-DAO side-chains. Businesses register as DAOs (pseudonymously), record B2B transactions on their side-chains, and at each Solstice the main-chain pulls a snapshot of every side-chain, validates transactions via POT, computes PageRank over the cumulative transaction graph, and mints tokens proportional to each DAO's reputation-weighted contribution.

### 2.1 Definitions

- **DAO**: A Decentralized Autonomous Organization. The on-chain counterpart of a real-world business. Individuals are not DAOs; only businesses are.
- **Node**: An electronic device connected to the Jsonic network running the JVM. A node may be a member of one or more DAOs.
- **User**: An individual who is a member of at least one DAO.
- **Transaction**: A B2B record of a business interaction. Currently invoices and payments are supported; a payment may reference an invoice and "settle" it.
- **Heartbeat**: A fixed time interval at which each JVM instance confirms its liveness to the network and processes pending work.
- **Solstice**: An interval (an integer multiple of the Heartbeat) at which the main-chain produces a new block by synchronizing snapshots from all side-chains. Analogous to the close of a financial period.
- **Materiality**: A side-chain block-creation threshold expressed as a percentage of total existing on-chain value. When pending transaction value exceeds this threshold, a new block is sealed.
- **POT (Proof of Transaction)**: The validation mechanism that checks signature validity, sequence-number contiguity, counterparty acknowledgment, and (for payments) settlement against a referenced invoice.
- **Adrenaline**: A scaling factor on the Heartbeat interval based on observed transaction throughput. Higher load shortens the Heartbeat.
- **Anxiety**: The proportion of invalid or incomplete transactions in the network. Lower is healthier; lies in (0, 1).

### 2.2 DAOs and Nodes

A DAO is a business entity that is part of the Jsonic network. Every device connected to the network is a node, and a node can be part of one or more DAOs. Multiple nodes may be part of a single DAO and record data on-chain on its behalf, so the DAO-to-node relationship is many-to-many. Each DAO has an Ed25519 identity keypair; its on-chain identifier is the first 40 hex characters of `SHA-256(public_key)`.

### 2.3 Side-chains

Each DAO maintains its own side-chain: a per-business ledger that doubles as an on-chain balance sheet. The side-chain accumulates transactions in a pending pool; when the pending value crosses the Materiality threshold, a new block is sealed. Each block carries a Merkle root over its transactions and a `closing_balance` (accounts receivable, accounts payable, revenue, expenses) computed by applying the block's transactions to the previous block's closing balance.

Unsupported transaction types may still be recorded on a side-chain but are not considered for token rewards.

### 2.4 Main-chain

The main-chain is the global Jsonic blockchain, connecting individual DAO side-chains. At every Solstice it collects DAOSnapshots from every side-chain and produces a new main-chain block containing those snapshots, the token distribution computed for this Solstice, and network health metrics (Anxiety, Adrenaline, Heartbeat, total token supply).

### 2.5 Side-chain block structure

A side-chain block contains:

- **Header**
  - Index (height in the side-chain)
  - Previous block hash
  - Timestamp
  - Transaction Merkle root
  - Block hash
- **Transactions**: the list of transactions sealed into this block (each independently signed by its originating DAO).
- **Closing balance**: the DAO's balance sheet (AR / AP / revenue / expenses) after applying this block's transactions.

### 2.6 Main-chain block structure

A main-chain block contains:

- **Header**: index, previous hash, timestamp, Merkle root over included DAO snapshots, block hash.
- **DAO snapshots**: for each side-chain in this Solstice, the closing balance, height, latest block hash, matched-transaction count and value, and the DAO's relevance score for this Solstice.
- **Token distribution**: the per-DAO token award computed by the trust-floor reward function (see §7).
- **Network metrics**: total DAOs, Anxiety, Heartbeat (effective ms), Adrenaline, and the new total token supply.

### 2.7 Proof of Transaction (POT) mechanism

POT is Jsonic's consensus primitive, inspired by real-world financial auditing. A transaction is valid under POT iff:

1. Its Ed25519 signature verifies under the originating DAO's public key.
2. Its sequence number is the next contiguous one expected from that DAO.
3. The counterparty has independently recorded the same transaction on its own side-chain (matching).
4. For Payment transactions, the referenced Invoice exists, the amounts agree, and the from/to are inverted (settlement).

A transaction that passes (1), (2), and (3) is **Matched**. A payment that passes (4) settles its invoice; both transitions go to **Settled**. Transactions that fail any check are **Invalid** and contribute to network Anxiety. Only Matched and Settled transactions contribute to side-chain balance updates and to the reputation graph (§3).

### 2.8 Jsonic Virtual Machine (JVM)

The JVM is the distributed computing instance running on each node. It validates transactions via POT, executes smart contracts, broadcasts side-chain updates to other members of the same DAO, synchronizes with the main-chain, and at Solstice participates in producing the next main-chain block. The JVM is the sole means of interacting with the network: every node must run it to participate.

The Heartbeat is implemented as a state-machine tick driven by the JVM; in production deployments it is bounded by wall-clock time, while the reference implementation drives it synchronously from a test loop.

---

## 3. Transaction Graph Model

The Jsonic network is modeled as a directed weighted graph **G = (V, E, w)** where:

- **V** = set of all nodes (DAOs and verified consumers).
- **E** ⊆ V × V = set of directed edges representing settled transactions.
- **w: E → ℝ⁺** = the edge weight function (defined in §5).

A directed edge **(u, v) ∈ E** exists when node **u** has paid node **v** for goods or services. Each edge carries:

| Symbol | Description |
|--------|-------------|
| **n(u, v)** | Number of individual settled transactions from u to v |
| **s(u, v)** | Total monetary value of those transactions |

The adjacency sets are:

- **In(v)** = { u ∈ V : (u, v) ∈ E }: nodes that bought from v.
- **Out(u)** = { v ∈ V : (u, v) ∈ E }: nodes that u bought from.
- **Partners(v)** = In(v) ∪ Out(v): all unique counterparties of v.

The graph is *cumulative*: edges added by a Solstice persist across Solstices, and reputation propagates over the entire historical record. (Pruning policies are out of scope for v0.3 and are tracked in the roadmap.)

---

## 4. PageRank Reputation Algorithm

### 4.1 Classical PageRank

The original PageRank formula (Brin & Page, 1998) assigns an importance score to each node based on its inbound links:

$$
\text{PR}(v) = \frac{1 - d}{N} + d \sum_{u \in \text{In}(v)} \frac{\text{PR}(u)}{|\text{Out}(u)|}
$$

where **d** ∈ (0, 1) is the **damping factor** (default 0.85), **N** = |V| is the total number of nodes, and **(1 - d) / N** is the probability of a random jump to any node.

### 4.2 Weighted PageRank for Jsonic

Jsonic extends classical PageRank with weighted edges. Each contribution is scaled by the proportion of the edge weight relative to the source node's total outbound weight:

$$
\text{PR}(v) = \frac{1 - d}{N} + d \sum_{u \in \text{In}(v)} \text{PR}(u) \cdot \frac{w(u, v)}{W_{\text{out}}(u)}
$$

where

$$
W_{\text{out}}(u) = \sum_{v \in \text{Out}(u)} w(u, v).
$$

If u buys from v₁ and v₂ and spends 3x more with v₁, then v₁ receives 3x more reputation flow from u.

### 4.3 Dangling nodes

Nodes with no outbound edges (typically pure consumers) are *dangling*. Their rank would be lost without redistribution. Following the standard treatment, we let

$$
D = \sum_{u : |\text{Out}(u)| = 0} \text{PR}(u),
$$

and the complete formula becomes

$$
\text{PR}(v) = \frac{1 - d}{N} + \frac{d \cdot D}{N} + d \sum_{u \in \text{In}(v)} \text{PR}(u) \cdot \frac{w(u, v)}{W_{\text{out}}(u)}.
$$

This guarantees rank conservation: Σ PR(v) = 1.

### 4.4 Convergence

PageRank is computed via the power method. Starting from a uniform distribution PR⁰(v) = 1/N, we iterate:

$$
\text{PR}^{(t+1)}(v) = \frac{1 - d}{N} + \frac{d \cdot D^{(t)}}{N} + d \sum_{u \in \text{In}(v)} \text{PR}^{(t)}(u) \cdot \frac{w(u, v)}{W_{\text{out}}(u)}.
$$

The iteration stops when the L₁ norm of the change falls below a tolerance ε:

$$
\| \text{PR}^{(t+1)} - \text{PR}^{(t)} \|_1 < \varepsilon.
$$

**Theorem** (convergence). For any d ∈ (0, 1), the power iteration converges to a unique stationary distribution with geometric rate d, so

$$
\| \text{PR}^{(t)} - \text{PR}^{*} \|_1 \leq \frac{2 d^t}{1 - d}.
$$

For d = 0.85 and ε = 10⁻⁸, convergence requires at most ~100 iterations. The reference implementation converges in 20-50 iterations on networks of 10⁴ nodes.

---

## 5. Edge Weight Function

The weight of an edge (u, v) is

$$
w(u, v) = n(u, v) \cdot \ln(1 + s(u, v)).
$$

Properties:

1. **Volume sensitivity**: doubling n(u, v) doubles the weight. 1,000 sales of $100 each carry 1,000x the weight of 1 sale of $100.
2. **Value dampening**: the logarithm prevents a single huge transaction from dominating. A $1M transaction has only ~2x the weight of a $1K transaction (ln(10⁶) ≈ 13.8 vs ln(10³) ≈ 6.9).
3. **Combined incentive**: the optimal strategy is many real-valued transactions across many counterparties, which aligns with genuine commercial activity.

| Scenario | n | s | w(n, s) |
|----------|---|---|---------|
| 1 sale of $100 | 1 | 100 | 4.62 |
| 1 sale of $1,000,000 | 1 | 1,000,000 | 13.82 |
| 1,000 sales of $100 each | 1,000 | 100,000 | 11,513 |
| 1,000 sales of $1,000 each | 1,000 | 1,000,000 | 13,816 |

A thousand small real sales dramatically outweigh a single large sale, regardless of total value.

---

## 6. Diversity (analytical metric)

Raw PageRank does not distinguish between a node with many counterparties and one with few. Jsonic computes a **diversity factor** for analytical and display purposes:

$$
\text{div}(v) = \frac{|\text{Partners}(v)|}{\max_{u \in V} |\text{Partners}(u)|}
$$

The composite score `Rep(v) = PR(v) · (1 + div(v))` is exposed by the reputation engine for inspection.

**Design note (v0.3).** Earlier formulations of Jsonic used `(1 + div(v))` as a multiplier on the per-DAO reward (§7). That approach was vulnerable to a *diversity inflation* attack: a Sybil ring with k fake unique buyers gets div = 1 regardless of buyer quality, and the multiplier swamped the buyer-reputation weighting. Since v0.3 the reward function uses the trust-floor formulation below, which captures effective diversity automatically (each unique high-trust buyer contributes independently to the sum) without admitting the inflation attack. The `div` field remains a useful surface metric but is no longer load-bearing for token economics.

---

## 7. Token Minting Function

### 7.1 Trust-floor reward

Define the **baseline rank** as the PageRank a node has purely from existing in the graph:

$$
\text{baseline} = \frac{1 - d}{N}.
$$

Every node's PR(u) is at least this value. Subtracting the baseline before weighting filters out the "everyone-gets-some-rank" signal so that only nodes which have *earned* trust from other earned-trust nodes contribute meaningfully. Define the **trust** of a node as

$$
\tau(u) = \max\big(0, \text{PR}(u) - \text{baseline}\big).
$$

The token reward for a DAO v is the sum of its inbound edge weights, each scaled by its buyer's trust:

$$
R(v) = \sum_{u \in \text{In}(v)} \ln(1 + s(u, v)) \cdot n(u, v) \cdot \tau(u).
$$

Properties of this formulation:

- Selling to a high-reputation buyer yields more tokens (linear in τ).
- Selling to many *independently-trusted* buyers yields more tokens (each contributes its own trust to the sum, so true diversity is rewarded).
- Higher sale values yield more tokens, log-dampened so a single huge transaction cannot dominate.
- A Sybil ring of k fake buyers contributes essentially zero because every fake's PageRank converges to the baseline (it has no inbound edges from earned-trust nodes), so τ(fake) ≈ 0.

### 7.2 Solstice distribution

At each Solstice the protocol mints a fixed budget **M** of tokens (constant `BASE_MINT_PER_SOLSTICE` in the reference implementation, default 10,000) and distributes them in proportion to the per-DAO rewards:

$$
\text{tokens}(v) = M \cdot \frac{R(v)}{\sum_{u \in V_{\text{DAO}}} R(u)}.
$$

V_DAO is the set of DAO nodes; consumers do not directly receive tokens. If Σ R(u) = 0 (no verified transactions in the cumulative graph), no tokens are minted: zero economic activity, zero inflation.

---

## 8. Sybil Resistance Analysis

### 8.1 Attack model

A Sybil attacker controls one attack DAO **a** plus k fake DAOs **{f₁, …, f_k}**. Each fake can be cheaply registered (it is just a keypair and an Ed25519 signature). The attacker has each fake submit a "payment" to **a** and self-acknowledges both sides, generating k settled invoices in a's favor.

Importantly, since payments in Jsonic are signed claims rather than real on-chain currency transfers, the attacker can pick fake transaction values arbitrarily large at no extra cost. Any honest defense must therefore not depend on the assumption that fake values are small.

### 8.2 Bound on the Sybil reward

Each fake fᵢ has exactly one outbound edge (to a) and zero inbound edges. Solving the PageRank fixed-point with N + k + 1 nodes and a as the only dangling node:

$$
\text{PR}(f_i) = \text{baseline} + \frac{d \cdot D'}{N + k + 1}
$$

where D' is the dangling sum (here, dominated by PR(a)). Substituting and simplifying for N >> dk gives

$$
\text{PR}(f_i) - \text{baseline} = O\!\left(\frac{1}{N}\right),
$$

so

$$
\tau(f_i) = O\!\left(\frac{1}{N}\right)
$$

uniformly in k. The attacker's reward is bounded by

$$
R(a)_{\text{sybil}} = \sum_{i=1}^{k} \ln(1 + s(f_i, a)) \cdot n(f_i, a) \cdot \tau(f_i) \leq k \cdot \ln(1 + s_{\max}) \cdot O\!\left(\frac{1}{N}\right).
$$

For k = αN with α a constant fraction of the network, R_sybil = O(ln(1 + s_max)): a constant in N, growing only logarithmically in fake transaction value. By contrast, the legitimate reward of a DAO embedded in the honest cluster grows linearly in the volume of real trade because its buyers have τ ≫ baseline. As the honest network grows, R_legit / R_sybil → ∞.

This is a strictly stronger result than the count-based diversity formulation it replaces, where R_sybil grew as O(k) before normalization. The trust-floor cap saturates at a constant regardless of how many fakes the attacker spins up.

### 8.3 Cost-benefit analysis

Beyond the reward bound, a Sybil ring also incurs registration cost (each fake DAO consumes a network slot and contributes to N), which dilutes everyone's baseline rank including the attacker's. The marginal gain to a from one more fake decreases as k grows. The economically rational equilibrium for a rational actor is to participate honestly: real settled trade with real high-reputation counterparties strictly dominates ring construction at every margin.

---

## 9. Network Dynamics

### 9.1 Anxiety

Anxiety measures the proportion of invalid or incomplete transactions:

$$
\text{Anxiety} = \frac{T_{\text{invalid}}}{T_{\text{total}}}.
$$

Anxiety lies in (0, 1), where lower is healthier. An Anxiety of 0.05 means 5% of submitted transactions failed POT validation.

### 9.2 Adrenaline and Heartbeat

The Heartbeat is the fixed interval at which JVMs confirm liveness and process pending work. As load increases, the Heartbeat must adapt. Adrenaline is the ratio of actual to target throughput:

$$
\text{Adrenaline} = \frac{T_{\text{heartbeat}}}{T_{\text{target}}}.
$$

The effective Heartbeat interval is

$$
H_{\text{effective}} = \max\!\left(\frac{H_{\text{base}}}{\text{Adrenaline}},\ H_{\text{min}}\right),
$$

where H_min (default 100ms) is a floor preventing instability under burst load.

| Load | Adrenaline | H_base (1s) | H_effective |
|------|-----------|-------------|-------------|
| Normal (100 tx/beat) | 1.0 | 1,000 ms | 1,000 ms |
| Double (200 tx/beat) | 2.0 | 1,000 ms | 500 ms |
| 10x (1,000 tx/beat) | 10.0 | 1,000 ms | 100 ms |

### 9.3 Materiality threshold

A side-chain seals a new block when accumulated pending transaction value crosses the **Materiality** threshold:

$$
\text{NewBlock} \iff \sum_{tx \in \text{pending}} \text{amount}(tx) \geq \text{Materiality} \cdot V_{\text{chain}},
$$

where Materiality ∈ (0, 1) (default 0.05) and V_chain is the total value already on the side-chain. For empty chains, an absolute minimum V_min (default 100) is used. Highly active DAOs produce blocks more frequently; quiet DAOs accumulate longer between blocks.

---

## 10. DAO Valuation Method

The on-chain valuation of a DAO combines its reputation, its on-chain transaction volume, and its operational health:

$$
\text{Valuation}(v) = \text{Rep}(v) \cdot V_{\text{chain}}(v) \cdot (1 - \text{Anxiety}(v))
$$

where Rep(v) = PR(v) · (1 + div(v)) (the composite reputation score), V_chain(v) is the total value of matched transactions on v's side-chain, and Anxiety(v) is the proportion of v's own submissions that failed validation. This creates a direct mapping from real-world activity to on-chain valuation:

- More verified transactions → higher V_chain → higher valuation.
- Better trading partners → higher Rep → higher valuation.
- Fewer invalid submissions → lower Anxiety → higher valuation.

In aggregate, the sum of all DAO valuations approximates the on-chain GDP of the network state.

---

## 11. Consumer Integration

End consumers participate as **Consumer nodes**, identified by tokenized payment credentials:

| Credential type | Tokenization |
|-----------------|--------------|
| Credit card | SHA-256(card_number ‖ expiry) |
| UPI ID | SHA-256(upi_id) |
| Wallet address | Direct (already pseudonymous) |

A Consumer participates in PageRank exactly like a DAO, but is a dangling node (consumers buy and do not sell). Their rank flows to the sellers they buy from. A consumer who buys repeatedly from many legitimate DAOs accumulates trust, which in turn makes their future purchases more valuable to sellers.

**Privacy.** Only the hash of the credential is stored on-chain. The underlying credit-card number, UPI ID, or wallet address is never exposed.

**Effect on DAO rewards.** A consumer c purchasing from DAO v contributes

$$
\Delta R(v) = \ln(1 + s(c, v)) \cdot n(c, v) \cdot \tau(c).
$$

A million unique consumers each buying one item from v drives a large reward (each consumer contributes its own trust). One consumer buying a million items from v drives a smaller reward (only one trust value enters the sum, and the value is log-dampened).

---

## 12. Properties and Guarantees

### 12.1 Rank conservation

$$
\sum_{v \in V} \text{PR}(v) = 1.
$$

### 12.2 Non-negativity

$$
\text{PR}(v) \geq \frac{1 - d}{N} > 0 \quad \forall v \in V.
$$

Every node has a strictly positive minimum reputation, but trust τ(v) = max(0, PR(v) - baseline) starts at zero and only grows from earned inbound flow.

### 12.3 Monotonicity

Adding a new edge (u, v) with u of nonzero trust never decreases v's reward:

$$
R'(v) \geq R(v).
$$

### 12.4 Sybil bound

For any number k of Sybil nodes attached to a single attacker a in a network of N legitimate nodes,

$$
R(a)_{\text{sybil}} \leq O\!\left(\ln(1 + s_{\max})\right),
$$

independent of k. The attack reward is constant in the size of the ring; only the legitimate-cluster reward scales with real activity.

### 12.5 Inflation control

$$
\text{Supply}(t) = \text{Supply}(t-1) + M \cdot \mathbb{1}\!\left[\sum_{v} R(v) > 0\right].
$$

Tokens are minted only when there is genuine economic activity. Zero settled volume produces zero new supply.

---

## 13. Integration with Existing Business Systems

### 13.1 Interoperability with existing infrastructure

A key advantage of Jsonic is its ability to integrate alongside existing business systems. The protocol is designed to coexist with traditional ERP, accounting, and invoicing tooling rather than replacing them outright: a DAO can record on-chain only the slice of its activity that benefits from cross-business verification.

### 13.2 Jsonic API and SDK

To enable smooth integration, Jsonic exposes a JSON-RPC API and SDKs (planned for v0.4) that let businesses submit invoices, register payments, query reputation scores, and retrieve their own balance sheet history programmatically.

### 13.3 Cross-chain communication

Cross-chain bridges to settle Jsonic-attested transactions on other Layer 1s (planned), enabling Jsonic-verified reputation to influence credit scoring and DeFi positions on other networks.

### 13.4 Compliance and regulatory considerations

The protocol provides built-in audit trails. Per-DAO side-chains preserve the full settled history, and the main-chain provides cryptographic anchoring of side-chain state at every Solstice. Hooks for KYC, AML, and jurisdiction-specific reporting are part of the SDK roadmap.

---

## 14. Roadmap and Long-term Vision

### 14.1 Ongoing research and development

Active areas of work:

- **Persistence layer.** The reference implementation keeps state in memory. v0.4 adds a `ChainStore` trait with a sled-backed default, so nodes can restart without losing reputation history.
- **P2P / RPC surface.** Adding libp2p-based gossip and a JSON-RPC interface to make Jsonic actually a network rather than a single-process simulator.
- **Reputation graph pruning.** Cumulative graphs grow without bound. Sliding-window or eigenvector-incremental updates are under evaluation.
- **Adversarial test suite.** Property tests over PageRank invariants, replay-attack tests against POT, and end-to-end Sybil simulations beyond the v0.3 case.
- **Parametric tokenomics.** Moving constants like `BASE_MINT_PER_SOLSTICE`, `SOLSTICE_INTERVAL`, `damping`, and `materiality` into a `Genesis` / `ChainParams` struct loaded from `genesis.toml`.

### 14.2 Planned features

- **eInvoicing and payment processing.** Smart-contract templates for invoice generation, payment tracking, and settlement.
- **Project management and billing.** Immutable project milestones, time tracking, and expense ledgers.
- **Timesheets and payroll.** Secure, transparent tracking of employee hours and wages.
- **Privacy enhancements.** Zero-knowledge proofs for amount confidentiality on settled transactions.

### 14.3 Long-term vision

- **Every business uses Jsonic.** A neutral substrate for B2B and B2C transactions across all sectors.
- **All transactions are published on-chain.** Audit-grade transparency across the supply chain.
- **The Jsonic token reflects real economic activity.** Token supply tracks settled volume, not speculation.
- **Aggregate token value approximates network-state GDP.** A new way to measure economic output rooted in verifiable activity rather than self-reported numbers.

---

## References

1. Brin, S. and Page, L. (1998). "The anatomy of a large-scale hypertextual web search engine." *Computer Networks and ISDN Systems*, 30(1-7), 107-117.
2. Langville, A. N. and Meyer, C. D. (2004). "Deeper inside PageRank." *Internet Mathematics*, 1(3), 335-380.
3. Cheng, A. and Friedman, E. (2005). "Sybilproof reputation mechanisms." *Proceedings of the 2005 ACM SIGCOMM Workshop on Economics of Peer-to-Peer Systems*.
4. Gyöngyi, Z., Garcia-Molina, H., and Pedersen, J. (2004). "Combating web spam with TrustRank." *Proceedings of the 30th International Conference on Very Large Data Bases (VLDB)*. (Inspiration for the trust-floor formulation in §7.)

---

<p align="center">
<em>Copyright 2024-2026 <a href="https://github.com/protosphinx">@protosphinx</a>. Licensed under CC-BY-SA-4.0.</em>
</p>
