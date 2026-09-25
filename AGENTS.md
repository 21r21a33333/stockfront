# Stockfront working agreement

## Scope and workflow

- Implement one approved vertical slice at a time. Do not pre-build abstractions, ports, configuration, folders or dependencies for later phases.
- Keep work uncommitted until it has been reviewed, unless the user explicitly requests a commit.
- Allowed working-branch prefixes are `chore/`, `fix/` and `feat/` only.
- Reuse stable libraries and protocol APIs before writing equivalent infrastructure.

## Architecture

- `stockfront-core` owns domain types, rules, use cases and outbound ports. It must remain deterministic and I/O-free.
- `stockfront-adapters` implements core ports for RPC, HTTP, persistence and protocol integrations.
- `stockfront-app` is the thin composition root and runtime entry point.
- Dependencies point inward: `app -> adapters -> core`. Core never depends on adapters or app.
- Add modules and ports only when their first working vertical slice needs them.

## Rust standards

- Production code must not use `unwrap()`, `expect()` or uncontrolled panics. Return typed errors at boundaries.
- Financial amounts and rates must use integer fixed-point or decimal types, never `f32` or `f64`.
- Prefer named result types over positional tuples when returning multiple domain values.
- Keep files focused, but do not split code into tiny modules without a clear ownership boundary.
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
- Treat direct contract state and protocol oracles as authoritative for solvency and execution.
- Validate externally produced transaction targets, selectors, chain IDs and amounts before returning them to users.
