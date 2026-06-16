# AI Scientist Project Status

## Overall Progress

Completion: 92%

Current Phase: Core platform stabilization and innovation module integration

Last Updated: 2026-06-02 18:54:39 +08:00

## Completed

- Agent framework core types and runtime traits
- Message bus and scheduler primitives
- AI Scientist agent implementations for research, hypothesis, experiment, verification, and report
- RAG pipeline components: PDF parsing, chunking, embedding interfaces, vector store, hybrid retrieval
- SymPy and Lean4 verification wrappers
- Scientific tool wrappers for computation, data, literature, privacy, security, visualization
- Experiment framework, orchestration, prompt engineering, observability, MCP, sandbox, and security layers
- Workspace crates and CI/Docker scaffolding
- `ai-scientist-core`, `ai-scientist-rag`, `ai-scientist-verify`, and `ai-scientist-science` library tests pass locally
- Local-first literature search/fetch fallback for markdown and PDF sources
- Lean environment status diagnostics for `lean`, `lake`, and Mathlib presence
- `tokitai-context` and `tokitai-filekv` are wired into the main workspace and exported from `ai-assistant`
- Local `arc-swap` compatibility shim added for workspace-only builds
- `tokitai-context` branch workflow integration test passes locally
- `tokitai-filekv` open/put/get integration test passes locally
- `tokitai-filekv` MemTable shard normalization fixed to avoid environment-dependent DashMap panics
- Local domain-science implementations added for chemistry, biology, and simulation workflows
- Scientist domain tools now expose chemistry, biology, and simulation operations through the tool system
- Scientific backend environment detection added for Python/RDKit/Biopython/ASE/LAMMPS/OpenFOAM/Psi4/Quantum ESPRESSO
- Scientific backend environment detection now includes Lean4 and Mathlib probes
- Auto backend selection layer added so domain tools can prefer vendor backends when available and fall back locally otherwise
- RDKit-backed chemistry execution path added behind the auto chemistry backend
- Biopython-backed biology execution path added behind the auto biology backend
- ASE-backed simulation execution path added behind the auto simulation backend
- LAMMPS and OpenFOAM CLI adapter paths added behind the auto simulation backend
- Psi4-backed quantum chemistry execution path added behind the auto chemistry backend
- Quantum ESPRESSO CLI adapter path added behind the auto simulation backend
- `scientist::tools::integration_tests` passes locally with 11/11 tests
- AI Scientist workflow contract coverage added for stage ordering, workflow TOML parsing, and agent handoff chain
- AI Scientist workflow runtime coverage added for local executable steps and explicit NotImplemented step surfacing

## In Progress

- Production-grade literature retrieval integration
- Real external scientific tool execution for missing ecosystems
- Lean4/Mathlib environment setup and verification
- Python scientific environment normalization across local and containerized runs
- Replacing local `arc-swap` shim with upstream crates.io dependency once registry access is stable
- `tokitai-filekv` dependency manifest now points to upstream `arc-swap = "1.7.1"`, and `cargo build -p tokitai-filekv` now succeeds locally
- `context_filekv_integration_test` remains blocked by `tokitai-context` still pulling `ahash 0.8.12`, whose upstream build script forces `cfg(specialize)` and fails on the stable toolchain

## Pending

- Full end-to-end scientific workflow execution tests against live external backends

## Dependency Status

SymPy: Installed
Lean4: Missing
Mathlib: Missing
RDKit: Missing
Biopython: Missing
ASE: Missing
LAMMPS: Missing
OpenFOAM: Missing
Psi4: Missing
Quantum ESPRESSO: Missing

## Architecture Status

Agent Framework: Configured
Workflow Engine: Configured
Tool Framework: Configured
RAG: Configured
Scientific Tools: Configured
Security Layer: Configured
AI Scientist Workflow: Partially Configured

## Technical Debt

- `LiteratureTools` remote API path is still not implemented, though local-first fallback works
- Lean4 verifier has no local `lake`/`lean` runtime in this environment
- RDKit/Biopython are code-integrated but not installed in this environment, so the auto-backend layer currently resolves to local fallback implementations
- ASE/LAMMPS/OpenFOAM are code-integrated but not installed in this environment, so the auto simulation backend currently resolves to fallback mode
- Psi4/Quantum ESPRESSO are code-integrated but not installed in this environment, so their auto-backend paths currently resolve to fallback mode
- Some environment-sensitive paths are only covered by fallback logic, not by CI matrix validation
- Upstream `arc-swap` dependency is declared again and `tokitai-filekv` now builds against it, but the wider integration test path is still blocked by `tokitai-context` depending on `ahash 0.8.12` that fails on stable due to forced `cfg(specialize)`
- Full workspace integration suite beyond `context_filekv_integration_test` is not yet re-run under restored crates.io resolution

## Next Priority

P0
- Keep Lean4 and scientific Python dependency checks synchronized with local environment state
- Finish the post-`arc-swap` integration recovery by removing the remaining `tokitai-context` / `ahash 0.8.12` stable-toolchain blocker
- Validate RDKit and Biopython backends in a real installed environment
- Validate Psi4 and Quantum ESPRESSO backends in a real installed environment
- Keep status files synchronized after each code change

P1
- Expand scientist workflow tests from local runtime coverage to full live-backend execution coverage
- Validate ASE/LAMMPS/OpenFOAM backends in a real installed environment
- Extend local literature indexing and metadata extraction

P2
- Add missing natural science tool integrations
- Expand container and CI coverage for scientific dependencies
