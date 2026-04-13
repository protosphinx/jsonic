# Contributing to Jsonic

Thank you for your interest in contributing to Jsonic! This document provides guidelines and information for contributors.

## How to Contribute

### Reporting Issues

- Use [GitHub Issues](https://github.com/protosphinx/jsonic/issues) to report bugs or suggest enhancements
- Check existing issues before creating a new one to avoid duplicates
- Provide as much detail as possible, including steps to reproduce any bugs

### Proposing Changes to the Protocol

Jsonic is a blockchain protocol, and changes to the core specification require careful consideration. To propose changes:

1. **Open a discussion** — Start by opening a GitHub Issue describing the proposed change and its rationale
2. **Reference the whitepaper** — Clearly identify which section(s) of the [whitepaper](whitepaper.md) would be affected
3. **Provide analysis** — Include technical analysis of how the change impacts:
   - The consensus mechanism (POT)
   - Tokenomics and DAO valuation
   - Side-chain and main-chain interactions
   - Backwards compatibility

### Submitting Pull Requests

1. Fork the repository
2. Create a feature branch from `main` (`git checkout -b feature/your-feature`)
3. Make your changes
4. Ensure your changes follow the existing formatting and style
5. Commit with clear, descriptive messages
6. Push to your fork and submit a pull request

### Pull Request Guidelines

- Keep PRs focused — one logical change per PR
- Update the table of contents in the whitepaper if adding or removing sections
- Ensure all diagrams and images are placed in the appropriate `assets/` subdirectory
- Reference any related issues in your PR description

## Style Guide

### Whitepaper

- Use clear, technical language appropriate for a protocol specification
- Define new terms in the Definitions and Abbreviations section (Section 2.2)
- Include diagrams where they aid understanding — place them in `assets/whitepaper/`
- Use proper markdown heading hierarchy (h2 for sections, h3 for subsections)

### Brand Assets

- Logo files should follow the existing naming convention: `logo-jsconic-{variant}-{color}.{ext}`
- Provide both PNG and SVG formats where possible
- Include light/dark mode variants for logos used in documentation

## Code of Conduct

- Be respectful and constructive in all interactions
- Focus on the technical merits of contributions
- Welcome newcomers and help them get started
- Disagree respectfully — critique ideas, not people

## Questions?

Join our community channels for discussion:

- [Discord](https://discord.gg/EjPJNwNA)
- [Twitter](https://twitter.com/protosphinx)
- [Substack](https://protosphinx.substack.com/)
