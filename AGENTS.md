# Stockfront working agreement

## Scope and workflow

- Implement one approved vertical slice at a time. Do not pre-build abstractions, ports, configuration, folders or dependencies for later phases.
- Keep work uncommitted until it has been reviewed, unless the user explicitly requests a commit.
- Allowed working-branch prefixes are `chore/`, `fix/` and `feat/` only.
- Reuse stable libraries and protocol APIs before writing equivalent infrastructure.
- Before presenting an integration LLD, inspect the current official documentation, OpenAPI contracts, generated SDKs and existing endpoints for every required capability. Record what will be reused and justify any custom RPC, indexer or equivalent infrastructure only after official options have been ruled out.
- Re-check provider capabilities immediately before implementation because external APIs and SDKs change.

## Architecture

- `stockfront-core` owns domain types, rules, use cases and outbound ports. It must remain deterministic and I/O-free.
- `stockfront-adapters` implements core ports for RPC, HTTP, persistence and protocol integrations.
- `stockfront-app` is the thin composition root and runtime entry point.
- Dependencies point inward: `app -> adapters -> core`. Core never depends on adapters or app.
- Add modules and ports only when their first working vertical slice needs them.
- Organize adapters by responsibility, such as authentication, provider calls, conversion, requests, responses and errors. Do not accumulate an integration in one large file.

## Boundaries and integration types

- Reuse domain read models directly inside the shared API `Response` and `List` envelopes when their meaning and public shape match. Do not add field-for-field response DTO copies; add a response DTO only for material redaction, versioning or shape differences.
- Reuse domain commands as HTTP request bodies when their meaning and public shape match. Add transport request DTOs only for a material shape, versioning or trust-boundary difference.
- Trace every public request field to a downstream provider parameter or an explicit Stockfront rule before adding it. A status or lookup endpoint takes its identifier only; never require callers to replay wallet, asset, chain or settlement context solely to revalidate the provider response.
- Define endpoint-specific commands instead of flattening a broad context object into requests that use only part of it. Use the same field name and representation whenever the same domain value crosses multiple endpoints.
- Return only data needed for the current result or the next client action. Do not echo request fields, embed a previously returned quote, fabricate expiry metadata or expose provider fields that the client does not use.
- Prefer `From` and `TryFrom` for provider-to-domain conversions over free `map_*` or `assemble_*` functions. If Rust's orphan rules prevent a direct implementation, use a minimal adapter-local wrapper rather than leaking provider types into core.
- Reuse canonical ecosystem primitives, such as Alloy addresses, instead of maintaining local equivalents.
- Generate provider request and response models from a pinned OpenAPI contract when one is available. Keep the compatibility specification limited to endpoints and fields Stockfront actually consumes, delete stale generated files after regeneration, and never hand-edit generated files.
- Do not repeat identity validation for fields already fixed by the request unless the provider can return mixed or ambiguous results.
- Treat newly introduced provider enum values as unknown or skip unsupported records when safe; one unfamiliar optional value must not fail an otherwise usable response.
- Add an independent RPC read, transfer verifier, settlement reader or indexer only when the approved product behavior requires evidence the provider cannot supply. Do not add one defensively around Binance status responses.
- Centralize application configuration in `setting.rs` and deserialize environment-backed secrets declaratively with Serde. Do not scatter `std::env::var` calls through runtime composition code.

## Rust standards

- Production code must not use `unwrap()`, `expect()` or uncontrolled panics. Return typed errors at boundaries.
- Financial amounts and rates must use integer fixed-point or decimal types, never `f32` or `f64`.
- Prefer named result types over positional tuples when returning multiple domain values.
- Keep files focused, but do not split code into tiny modules without a clear ownership boundary.
- Before review, search every newly added field, type, trait, error variant and helper. Remove anything with no concrete call site or invariant; retain a trait only when it is an active hexagonal port and retain an adapter wrapper only when it enables a required boundary conversion.
- Run `cargo fmt --check`, Clippy with warnings denied and the relevant tests before review.

## Tests

- TDD is not required. Tests should protect behavior or invariants that can realistically regress.
- Do not test trivial constructors, derives, route registration, static configuration or empty composition glue.
- Core rules receive deterministic unit tests. Adapters receive integration tests against controlled fixtures or pinned forks when their behavior warrants it.
- Every test must have a clear failure mode and user or protocol consequence.

## Comments and documentation

- Comments explain why a decision or invariant exists, not what the next line does.
- Do not add task, phase, plan or change-history breadcrumbs to source comments.
- Public documentation should be brief and describe contracts or non-obvious guarantees.
- Prefer expressive names and types over explanatory comments. Delete comments that merely restate code.

## Safety

- Never log secrets, signatures, raw private transactions or authorization headers.
- Use direct contract state and protocol oracles when an approved feature explicitly owns solvency or execution verification; do not introduce that responsibility into a provider-proxy slice.
- Treat responses from the pinned Binance Web3 client as provider-owned data. Parse fields required by Stockfront's typed domain model and enforce Stockfront-owned policy, but do not revalidate response identities, calldata semantics or values already fixed by the request.
