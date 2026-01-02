<!--
Sync Impact Report:
Version change: 1.0.0 → 1.1.0
Modified principles: N/A
Added sections: Build Verification & GPG Signing (in Development Workflow)
Removed sections: N/A
Templates requiring updates:
  ✅ plan-template.md - Constitution Check section already covers build requirements
  ✅ spec-template.md - No changes needed
  ✅ tasks-template.md - No changes needed
Follow-up TODOs: None
-->

# Data Modelling SDK Constitution

## Core Principles

### I. Code Quality & Security (NON-NEGOTIABLE)

All code MUST pass language-specific security audit, formatting, linting, and best practices checks before merge. This includes:

- **Security Audit**: All dependencies MUST pass `cargo audit` with no unaddressed vulnerabilities. Known unmaintained dependency warnings may be allowed via `cargo-audit.toml` configuration, but MUST be documented and reviewed periodically.
- **Formatting**: All code MUST pass `cargo fmt --all -- --check`. No manual formatting exceptions allowed.
- **Linting**: All code MUST pass `cargo clippy --all-targets --all-features -- -D warnings`. All clippy warnings MUST be addressed or explicitly allowed with justification.
- **Dependency Management**: Dependencies MUST use the latest stable versions compatible with the project's minimum supported Rust version. Version updates MUST be evaluated for security, compatibility, and feature needs.
- **Pre-commit Hooks**: All developers MUST have pre-commit hooks installed and enabled. CI/CD MUST enforce these checks as gates before merge.

**Rationale**: Consistent code quality, security posture, and maintainability are foundational to a shared SDK used across multiple platforms. Automated enforcement prevents technical debt accumulation and security vulnerabilities.

### II. Storage Abstraction

The SDK MUST use trait-based storage backends to support multiple environments (native filesystem, WASM browser storage, HTTP API). All storage operations MUST be async and use the `StorageBackend` trait interface.

- Storage backends MUST be independently testable and mockable
- Storage operations MUST handle errors consistently via `StorageError`
- Path traversal attacks MUST be prevented in filesystem backends
- Storage backends MUST be feature-gated when platform-specific

**Rationale**: Enables the SDK to work across native applications, WASM/browser environments, and API-based systems without code duplication.

### III. Feature Flags & Dependencies

Optional functionality MUST be gated behind Cargo features to minimize dependencies and support multiple deployment targets.

- Features MUST be clearly documented in `Cargo.toml`
- Default features MUST provide a functional core without optional dependencies
- Platform-specific features (e.g., `wasm`, `native-fs`) MUST be clearly separated
- Feature combinations MUST be tested in CI/CD

**Rationale**: Reduces binary size, compilation time, and dependency surface area while maintaining flexibility for different use cases.

### IV. Testing Requirements

All features MUST include appropriate test coverage:

- **Unit Tests**: Individual module tests for core logic
- **Integration Tests**: End-to-end workflows testing storage backends, import/export, and validation
- **Doctests**: Documentation examples MUST be executable and tested
- Tests MUST run with `cargo test --all-features` to verify feature combinations
- Tests MUST be platform-agnostic where possible, with platform-specific tests clearly marked

**Rationale**: Comprehensive testing ensures reliability across platforms and prevents regressions during refactoring.

### V. Import/Export Patterns

Importers MUST convert external formats to SDK `Table` models. Exporters MUST convert SDK models to external formats. All import/export operations MUST:

- Handle errors gracefully with structured error types
- Validate input data before processing
- Support the primary format (ODCS v3.1.0) with legacy format support where needed
- Be independently testable and mockable

**Rationale**: Consistent import/export patterns enable format conversion while maintaining a single source of truth (SDK models).

### VI. Error Handling Standards

Error handling MUST use structured error types:

- Use `thiserror` for library errors that may be matched by consumers
- Use `anyhow::Result` for application-level convenience where error details are less critical
- Error messages MUST be clear, actionable, and include context
- Error types MUST implement `std::error::Error` and support error chaining

**Rationale**: Structured errors enable proper error handling in consuming applications and improve debugging.

## Security Requirements

### Input Validation

All imported data MUST be validated before processing:

- Table and column names MUST conform to naming rules (alphanumeric, hyphens, underscores, max 255 chars)
- SQL identifiers MUST be properly escaped when exporting
- Path parameters MUST be validated to prevent directory traversal
- Domain parameters MUST be validated (max 100 chars, alphanumeric/hyphens/underscores only)

### Dependency Security

- Security advisories MUST be tracked via `cargo audit`
- Known unmaintained dependency warnings MUST be documented in `cargo-audit.toml`
- Security updates MUST be prioritized and applied promptly
- Dependency versions MUST be reviewed regularly for security patches

## Development Workflow

### Commit Requirements (NON-NEGOTIABLE)

Code MAY ONLY be committed when:

- **Build Verification**: Code MUST build successfully (`cargo build` or `cargo build --release`) before commit. No broken builds allowed in the repository.
- **GPG Signing**: All commits MUST be signed with a valid GPG key. Developers MUST configure Git to sign commits (`git config commit.gpgsign true` or use `git commit -S`). Unsigned commits MUST be rejected.

**Rationale**: Build verification prevents broken code from entering the repository and maintains a clean, buildable codebase. GPG signing provides commit authenticity and non-repudiation, essential for security and auditability in a shared SDK.

### Pre-commit Requirements

All developers MUST:

- Install pre-commit hooks: `pre-commit install`
- Run hooks manually before committing: `pre-commit run --all-files`
- Ensure all hooks pass before pushing code
- Configure GPG signing: `git config commit.gpgsign true` or use `git commit -S` for each commit

### CI/CD Gates

All pull requests MUST pass:

- Build verification (`cargo build --all-features`)
- Formatting check (`cargo fmt --all -- --check`)
- Linting check (`cargo clippy --all-targets --all-features -- -D warnings`)
- Security audit (`cargo audit`)
- All tests (`cargo test --all-features`)
- Release build verification
- GPG signature verification (commits MUST be signed)

### Code Review Requirements

All code reviews MUST verify:

- Constitution compliance (especially Principle I: Code Quality & Security)
- Test coverage for new features
- Error handling follows standards
- Documentation is updated
- No breaking changes without version bump

## Governance

This constitution supersedes all other development practices. Amendments require:

1. Documentation of the change rationale
2. Update to this constitution file with version bump
3. Propagation to dependent templates and documentation
4. Team review and approval

All PRs and code reviews MUST verify compliance with these principles. Complexity or exceptions MUST be justified and documented.

**Version**: 1.1.0 | **Ratified**: 2026-01-02 | **Last Amended**: 2026-01-02
