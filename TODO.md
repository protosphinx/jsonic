# Jsonic TODO

## Current Task: Reconcile whitepaper.md and paper.md

Merge both documents into a single unified `whitepaper.md` and delete `paper.md`.

### Plan: Push in 3 sections

- [ ] **Part 1**: Header, Abstract, Introduction, Architecture (Sections 1-3)
      - Ecosystem narrative, diagrams, block structures, JVM, smart contracts
      - Source: original whitepaper.md sections 1-3
- [ ] **Part 2**: Formal Mathematics (Sections 4-7)
      - Transaction Graph Model, PageRank algorithm, Edge Weight Function
      - Diversity Factor, Token Minting, Sybil Resistance, Network Dynamics
      - Source: paper.md sections 1-7
- [ ] **Part 3**: Integration, Properties, Roadmap, References (Sections 8-11)
      - Consumer Integration, DAO Valuation, Properties & Guarantees
      - Ecosystem Integration, Roadmap, Long-term Vision, References
      - Source: paper.md sections 8-10 + whitepaper.md sections 5-6
- [ ] Delete `paper.md`
- [ ] Update README.md reference from `paper.md` to `whitepaper.md`
- [ ] Commit and push to main

### Notes
- Author config: `protosphinx` / `protosphinx@users.noreply.github.com`
- Branch: push directly to `main`
- GitHub MCP tools available for PR workflow if needed
- All Rust code (52 tests) already merged to main
