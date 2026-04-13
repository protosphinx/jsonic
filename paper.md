<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/logo-jsconic-horizontal-reverse.png">
    <img src="assets/brand/logo-jsconic-horizontal-color.svg" height="60">
  </picture>
</p>

<h1 align="center">Jsonic: A PageRank-Weighted Proof of Transaction Protocol for Manufacturing Economies</h1>

<p align="center"><em>Technical Paper v0.2.0</em></p>

<p align="center">
<strong>Authors:</strong> <a href="https://github.com/protosphinx">@protosphinx</a><br>
<strong>Date:</strong> 2024<br>
<strong>License:</strong> CC-BY-SA-4.0
</p>

---

## Abstract

Jsonic is a Layer 1 blockchain protocol designed to tokenize real-world manufacturing output. Unlike proof-of-work or proof-of-stake systems that reward computational or capital commitment, Jsonic introduces **Proof of Transaction (POT)** — a consensus mechanism that rewards verified production and sale of physical goods. Token minting is proportional to economically meaningful work: manufacturing inventory, selling it, and having the transaction verified by both counterparties on-chain.

Central to Jsonic's economics is a **PageRank-based reputation system** that propagates trust through the transaction graph. This prevents Sybil attacks, rewards diverse trading relationships, and ensures that tokens flow to participants who generate genuine economic activity rather than to those who game the system with self-referential transactions.

This paper formalizes the mathematical foundations of the protocol: the PageRank reputation algorithm, the token minting function, the Sybil resistance properties, and the network health dynamics.

---

## Table of Contents

- [1. Transaction Graph Model](#1-transaction-graph-model)
- [2. PageRank Reputation Algorithm](#2-pagerank-reputation-algorithm)
  - [2.1 Classical PageRank](#21-classical-pagerank)
  - [2.2 Weighted PageRank for Jsonic](#22-weighted-pagerank-for-jsonic)
  - [2.3 Dangling Nodes](#23-dangling-nodes)
  - [2.4 Convergence](#24-convergence)
- [3. Diversity Factor](#3-diversity-factor)
- [4. Edge Weight Function](#4-edge-weight-function)
- [5. Token Minting Function](#5-token-minting-function)
  - [5.1 Per-DAO Reward](#51-per-dao-reward)
  - [5.2 Solstice Distribution](#52-solstice-distribution)
- [6. Sybil Resistance Analysis](#6-sybil-resistance-analysis)
  - [6.1 Attack Model](#61-attack-model)
  - [6.2 Reward Bound for Sybil Nodes](#62-reward-bound-for-sybil-nodes)
  - [6.3 Cost-Benefit Analysis](#63-cost-benefit-analysis)
- [7. Network Dynamics](#7-network-dynamics)
  - [7.1 Anxiety](#71-anxiety)
  - [7.2 Adrenaline and Heartbeat](#72-adrenaline-and-heartbeat)
  - [7.3 Materiality Threshold](#73-materiality-threshold)
- [8. DAO Valuation Method](#8-dao-valuation-method)
- [9. Consumer Integration](#9-consumer-integration)
- [10. Properties and Guarantees](#10-properties-and-guarantees)

---

## 1. Transaction Graph Model

The Jsonic network is modeled as a directed weighted graph **G = (V, E, w)** where:

- **V** = set of all nodes (DAOs and verified consumers)
- **E** ⊆ V × V = set of directed edges representing transaction flows
- **w: E → ℝ⁺** = edge weight function

A directed edge **(u, v) ∈ E** exists when node **u** has purchased goods or services from node **v**. The edge carries two attributes:

| Symbol | Description |
|--------|-------------|
| **n(u,v)** | Number of individual transactions from u to v |
| **s(u,v)** | Total monetary value of transactions from u to v |

The **adjacency sets** are defined as:

- **In(v)** = { u ∈ V : (u, v) ∈ E } — nodes that buy from v
- **Out(u)** = { v ∈ V : (u, v) ∈ E } — nodes that u buys from
- **Partners(v)** = In(v) ∪ Out(v) — all unique counterparties of v

---

## 2. PageRank Reputation Algorithm

### 2.1 Classical PageRank

The original PageRank formula (Brin & Page, 1998) assigns an importance score to each node based on the structure of inbound links:

$$
\text{PR}(v) = \frac{1 - d}{N} + d \sum_{u \in \text{In}(v)} \frac{\text{PR}(u)}{|\text{Out}(u)|}
$$

Where:
- **d** ∈ (0, 1) is the **damping factor** (default: 0.85)
- **N** = |V| is the total number of nodes
- The term **(1 - d) / N** represents the probability of a "random jump" to any node

### 2.2 Weighted PageRank for Jsonic

Jsonic extends classical PageRank with **weighted edges**. Instead of treating all links equally, we weight each contribution by the proportion of the edge weight relative to the source node's total outbound weight:

$$
\text{PR}(v) = \frac{1 - d}{N} + d \sum_{u \in \text{In}(v)} \text{PR}(u) \cdot \frac{w(u, v)}{W_{\text{out}}(u)}
$$

Where **W_out(u)** is the total outbound weight from node u:

$$
W_{\text{out}}(u) = \sum_{v \in \text{Out}(u)} w(u, v)
$$

This means that if node **u** buys from nodes **v₁** and **v₂**, and spends 3× more with **v₁**, then **v₁** receives 3× more reputation flow from **u**.

### 2.3 Dangling Nodes

Nodes with no outbound edges (|Out(u)| = 0) are **dangling nodes** — typically consumers who buy but don't sell. Their rank would be lost without redistribution. Following the standard treatment:

$$
D = \sum_{u : |\text{Out}(u)| = 0} \text{PR}(u)
$$

The complete formula becomes:

$$
\text{PR}(v) = \frac{1 - d}{N} + \frac{d \cdot D}{N} + d \sum_{u \in \text{In}(v)} \text{PR}(u) \cdot \frac{w(u, v)}{W_{\text{out}}(u)}
$$

This ensures that rank is conserved: **Σ PR(v) = 1** for all v ∈ V.

### 2.4 Convergence

PageRank is computed iteratively via the **power method**. Starting from a uniform distribution PR⁰(v) = 1/N, we iterate:

$$
\text{PR}^{(t+1)}(v) = \frac{1 - d}{N} + \frac{d \cdot D^{(t)}}{N} + d \sum_{u \in \text{In}(v)} \text{PR}^{(t)}(u) \cdot \frac{w(u, v)}{W_{\text{out}}(u)}
$$

**Convergence criterion**: The iteration stops when the L₁ norm of the change falls below a tolerance ε:

$$
\| \text{PR}^{(t+1)} - \text{PR}^{(t)} \|_1 = \sum_{v \in V} | \text{PR}^{(t+1)}(v) - \text{PR}^{(t)}(v) | < \varepsilon
$$

**Theorem** (Convergence guarantee): For any damping factor d ∈ (0, 1), the power iteration converges to a unique stationary distribution. The rate of convergence is geometric with factor d, so:

$$
\| \text{PR}^{(t)} - \text{PR}^{*} \|_1 \leq \frac{2 d^t}{1 - d}
$$

For d = 0.85 and ε = 10⁻⁸, convergence is reached in at most ~100 iterations. In practice, the Jsonic reference implementation converges in 20–50 iterations for networks of 10,000+ nodes.

---

## 3. Diversity Factor

Raw PageRank does not distinguish between a node with many counterparties and one with few. Jsonic introduces a **diversity factor** that rewards breadth of trading relationships:

$$
\text{div}(v) = \frac{|\text{Partners}(v)|}{\max_{u \in V} |\text{Partners}(u)|}
$$

This normalizes the diversity to [0, 1], where 1 means the node has the most diverse trading relationships in the network.

The **composite reputation score** combines PageRank and diversity:

$$
\text{Rep}(v) = \text{PR}(v) \cdot (1 + \text{div}(v))
$$

This means:
- A node with maximum diversity gets up to **2× its raw PageRank**
- A node trading with only one counterparty gets approximately **1× its raw PageRank**
- This incentivizes DAOs to trade broadly rather than concentrating all business with a single partner

---

## 4. Edge Weight Function

The weight of an edge (u, v) is defined as:

$$
w(u, v) = n(u, v) \cdot \ln(1 + s(u, v))
$$

Where:
- **n(u,v)** = number of transactions (linear scaling — more transactions = proportionally more weight)
- **ln(1 + s(u,v))** = log-dampened value (prevents a single enormous transaction from dominating)

**Properties of this weight function:**

1. **Volume sensitivity**: Doubling the number of transactions exactly doubles the weight. Selling 1 item to 1,000 buyers is 1,000× the weight of 1 sale to 1 buyer.

2. **Value dampening**: The logarithm ensures that a $1M transaction is not 1,000× more impactful than a $1,000 transaction. Specifically:
   - w(1 tx, $1,000) = 1 × ln(1,001) ≈ 6.91
   - w(1 tx, $1,000,000) = 1 × ln(1,000,001) ≈ 13.82
   - Ratio ≈ 2×, not 1,000×

3. **Combined incentive**: The optimal strategy is many real-valued transactions across diverse counterparties, which aligns with genuine manufacturing commerce.

**Comparison of scenarios:**

| Scenario | n | s | w(n,s) |
|----------|---|---|--------|
| 1 sale of $100 | 1 | 100 | 4.62 |
| 1 sale of $1,000,000 | 1 | 1,000,000 | 13.82 |
| 1,000 sales of $100 each | 1,000 | 100,000 | 11,513 |
| 1,000 sales of $1,000 each | 1,000 | 1,000,000 | 13,816 |

This shows that **1,000 real sales dramatically outweigh a single large sale**, regardless of total value.

---

## 5. Token Minting Function

### 5.1 Per-DAO Reward

The token reward for a DAO is computed from its verified sales, weighted by buyer reputation and diversity:

$$
R(v) = (1 + \text{div}(v)) \cdot \sum_{u \in \text{In}(v)} \ln(1 + s(u,v)) \cdot n(u,v) \cdot \text{PR}(u)
$$

Breaking this down:
- For each buyer **u** of DAO **v**: the contribution is the edge weight × buyer's PageRank
- Buyers with higher PageRank contribute more to the seller's reward
- The whole sum is multiplied by the diversity boost

**Key property**: **R(v) is maximized when v sells to many high-reputation buyers**, not when v self-transacts or sells to fake entities.

### 5.2 Solstice Distribution

At each Solstice (periodic sync point), a fixed number of tokens **M** are minted and distributed proportionally:

$$
\text{tokens}(v) = M \cdot \frac{R(v)}{\sum_{u \in V_{\text{DAO}}} R(u)}
$$

Where V_DAO is the set of all DAO nodes (consumers don't receive tokens directly).

If Σ R(u) = 0 (no verified transactions), no tokens are minted. This prevents inflation without economic activity.

---

## 6. Sybil Resistance Analysis

### 6.1 Attack Model

A **Sybil attacker** creates **k** fake nodes **{f₁, f₂, …, fₖ}** that all "buy" from a single attack node **a**. The goal is to inflate **R(a)** and capture tokens.

### 6.2 Reward Bound for Sybil Nodes

Each fake node fᵢ is a dangling node (no outbound edges to other real nodes except to **a**). Its PageRank is bounded:

$$
\text{PR}(f_i) \leq \frac{1 - d}{N + k} + \frac{d}{N + k} \cdot D'
$$

Where **D'** is the dangling sum after adding k nodes. Since the fake nodes only link to **a**, they don't receive any inbound reputation from legitimate nodes. In a large network (N >> k):

$$
\text{PR}(f_i) \approx \frac{1}{N + k}
$$

The attacker's reward is:

$$
R(a)_{\text{sybil}} = (1 + \text{div}(a)) \cdot \sum_{i=1}^{k} w(f_i, a) \cdot \text{PR}(f_i)
$$

$$
\approx (1 + \text{div}(a)) \cdot k \cdot w_{\text{fake}} \cdot \frac{1}{N + k}
$$

$$
= (1 + \text{div}(a)) \cdot \frac{k}{N + k} \cdot w_{\text{fake}}
$$

As **N grows**, this converges to:

$$
R(a)_{\text{sybil}} \to (1 + \text{div}(a)) \cdot \frac{k}{N} \cdot w_{\text{fake}}
$$

### 6.3 Cost-Benefit Analysis

For a legitimate DAO **v** with m buyers of average reputation **PR_avg** and average edge weight **w_avg**:

$$
R(v)_{\text{legit}} \approx (1 + \text{div}(v)) \cdot m \cdot w_{\text{avg}} \cdot \text{PR}_{\text{avg}}
$$

For the Sybil ratio to favor attacking:

$$
\frac{R(a)_{\text{sybil}}}{R(v)_{\text{legit}}} = \frac{k \cdot w_{\text{fake}} \cdot \frac{1}{N}}{m \cdot w_{\text{avg}} \cdot \text{PR}_{\text{avg}}} \cdot \frac{(1 + \text{div}(a))}{(1 + \text{div}(v))}
$$

Since **PR_avg >> 1/N** for legitimate actors (they have reputation from real trading partners), the attacker needs **k >> m × N × PR_avg** fake nodes to compete — which becomes prohibitively expensive on-chain (each fake DAO must be registered and pay transaction fees).

**Conclusion**: The cost of Sybil attack scales linearly with network size N, while the benefit diminishes as O(1/N). Attacking is economically irrational in any reasonably-sized network.

---

## 7. Network Dynamics

### 7.1 Anxiety

Anxiety measures the proportion of invalid or incomplete transactions:

$$
\text{Anxiety} = \frac{T_{\text{invalid}}}{T_{\text{total}}}
$$

Where:
- **T_invalid** = number of transactions that failed POT validation
- **T_total** = total number of submitted transactions

**Anxiety ∈ (0, 1)** where lower is healthier. An Anxiety of 0.05 means 5% of transactions are invalid.

### 7.2 Adrenaline and Heartbeat

The **Heartbeat** is the fixed interval at which nodes confirm liveness. As network load increases, the Heartbeat must adapt.

**Adrenaline** is the ratio of actual to target transaction throughput:

$$
\text{Adrenaline} = \frac{T_{\text{heartbeat}}}{T_{\text{target}}}
$$

The effective Heartbeat interval is adjusted by Adrenaline:

$$
H_{\text{effective}} = \max\left(\frac{H_{\text{base}}}{\text{Adrenaline}}, H_{\text{min}}\right)
$$

Where **H_min** = 100ms is a floor to prevent instability.

| Load | Adrenaline | H_base (1s) | H_effective |
|------|-----------|-------------|-------------|
| Normal (100 tx/beat) | 1.0 | 1,000ms | 1,000ms |
| Double (200 tx/beat) | 2.0 | 1,000ms | 500ms |
| 10× (1000 tx/beat) | 10.0 | 1,000ms | 100ms |

### 7.3 Materiality Threshold

Side-chains generate new blocks when accumulated pending transaction value reaches the **Materiality** threshold:

$$
\text{NewBlock} \iff \sum_{tx \in \text{pending}} \text{amount}(tx) \geq \text{Materiality} \cdot V_{\text{chain}}
$$

Where:
- **Materiality** ∈ (0, 1), default 0.05 (5%)
- **V_chain** = total value of all transactions on the existing chain
- For empty chains, a minimum absolute threshold **V_min** = 100 is used

This ensures blocks are created proportionally to economic activity — highly active DAOs produce blocks more frequently.

---

## 8. DAO Valuation Method

The value of a DAO on the Jsonic network is determined by its composite reputation score and economic activity:

$$
\text{Valuation}(v) = \text{Rep}(v) \cdot V_{\text{chain}}(v) \cdot (1 - \text{Anxiety}(v))
$$

Where:
- **Rep(v)** = composite reputation score (PageRank × diversity)
- **V_chain(v)** = total value of matched transactions on v's side-chain
- **Anxiety(v)** = proportion of v's transactions that failed validation

This creates a direct mapping from real-world business activity to on-chain valuation:
- More verified transactions → higher V_chain → higher valuation
- Better trading partners → higher Rep → higher valuation
- Fewer invalid transactions → lower Anxiety → higher valuation

---

## 9. Consumer Integration

End consumers participate in the graph as **Consumer nodes**, identified by tokenized payment credentials:

| Credential Type | Tokenization |
|----------------|-------------|
| Credit Card | SHA-256(card_number \|\| expiry) |
| UPI ID | SHA-256(upi_id) |
| Wallet Address | Direct (already pseudonymous) |

**Consumer PageRank** follows the same formula as DAO PageRank. A consumer who buys from multiple legitimate DAOs accumulates reputation, which in turn makes their future purchases more valuable to sellers.

**Privacy**: Only the hash of the credential is stored on-chain. The actual credit card number or UPI ID is never exposed.

**Impact of consumer purchases on DAO rewards**:

A consumer with reputation PR(c) purchasing from DAO v contributes:

$$
\Delta R(v) = \ln(1 + s(c, v)) \cdot n(c, v) \cdot \text{PR}(c) \cdot (1 + \text{div}(v))
$$

This means:
- **1 million unique consumers** each buying 1 item from DAO v generates a massive reward (each consumer contributes their individual PageRank, and diversity is maximized)
- **1 consumer buying 1 million items** generates a smaller reward (only one consumer's PageRank, diversity is minimal, and log-dampened value limits the impact)

---

## 10. Properties and Guarantees

### 10.1 Rank Conservation

$$
\sum_{v \in V} \text{PR}(v) = 1
$$

The total PageRank is always exactly 1, regardless of network size or topology.

### 10.2 Non-Negativity

$$
\text{PR}(v) \geq \frac{1 - d}{N} > 0 \quad \forall v \in V
$$

Every node has a strictly positive minimum reputation.

### 10.3 Monotonicity

If a new edge (u, v) is added to the graph (u buys from v), then:

$$
\text{PR}'(v) \geq \text{PR}(v)
$$

Gaining a new buyer never decreases your reputation.

### 10.4 Sybil Bound

For k Sybil nodes in a network of N legitimate nodes:

$$
R_{\text{sybil}} \leq O\left(\frac{k}{N}\right) \cdot w_{\text{fake}}
$$

The attack reward diminishes with network size.

### 10.5 Inflation Control

$$
\text{Supply}(t) = \text{Supply}(t-1) + M \cdot \mathbb{1}\left[\sum_{v} R(v) > 0\right]
$$

Tokens are only minted when there is genuine economic activity (R > 0). No activity = no inflation.

---

## References

1. Brin, S. and Page, L. (1998). "The anatomy of a large-scale hypertextual web search engine." *Computer Networks and ISDN Systems*, 30(1-7), 107-117.

2. Langville, A.N. and Meyer, C.D. (2004). "Deeper inside PageRank." *Internet Mathematics*, 1(3), 335-380.

3. Cheng, A. and Friedman, E. (2005). "Sybilproof reputation mechanisms." *Proceedings of the 2005 ACM SIGCOMM Workshop on Economics of Peer-to-Peer Systems*.

---

<p align="center">
<em>Copyright 2024 <a href="https://github.com/protosphinx">@protosphinx</a>. Licensed under CC-BY-SA-4.0.</em>
</p>
