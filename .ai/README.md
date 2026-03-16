# Internal Docs

This folder is the internal documentation hub for the project.

## Core Docs

- [`PRODUCT.md`](PRODUCT.md): product goals, use cases, constraints, and value proposition
- [`ARCHITECTURE.md`](ARCHITECTURE.md): technical overview, components, paths, and runtime flows
- [`STATUS.md`](STATUS.md): current behavior, known gaps, recent changes, and near-term priorities
- [`ROADMAP.md`](ROADMAP.md): milestone-based roadmap focused on product maturity and distribution
- [`CONTRIBUTING.md`](CONTRIBUTING.md): contributor workflow, validation steps, release hygiene, and documentation rules
- [`RUST_GUIDE_FOR_JAVA_PYTHON.md`](RUST_GUIDE_FOR_JAVA_PYTHON.md): practical Rust reading guide for this repository

## Working Rules

- read [`STATUS.md`](STATUS.md) before changing user-visible behavior
- update [`STATUS.md`](STATUS.md) when the product behavior changes
- update [`ARCHITECTURE.md`](ARCHITECTURE.md) when components, file flows, or runtime paths change
- update [`ROADMAP.md`](ROADMAP.md) when delivery priorities change
- keep public user docs at the repository root and internal maintenance docs in `.ai/`

## Public Docs

Public-facing repository docs live at the repository root:

- [`../README.md`](../README.md)
- [`../INSTALL.md`](../INSTALL.md)
- [`../SHORTCUTS.md`](../SHORTCUTS.md)
- [`../RELEASES.md`](../RELEASES.md)
- [`../STRUCTURE.md`](../STRUCTURE.md)
