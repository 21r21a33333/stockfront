# Stockfront web app

Three ways into the same artifact: describe a strategy in plain language, draw
it as blocks, or load one of the curated strategies. All of them produce a
recipe that the backend compiler validates before anything can be signed.

```sh
pnpm install
pnpm dev
```

Vite forwards `/api` to the Stockfront backend. See `.env.example` for local configuration.

## Layout

- `src/app`: routing (`#/`, `#/compose`, `#/builder`, `#/run`), header and error boundary.
- `src/landing`: the marketing page.
- `src/compose`: the AI strategy composer (`/v1/strategies/compose`), the curated
  strategies with their live figures, and the plan card.
- `src/builder`: the block canvas, palette and inspector, with live health-factor
  previews and per-trade route selection.
- `src/run`: step-by-step signing, price-change and recovery states, and the finish screen.
- `src/positions`: the positions slide-over.
- `src/session`: the shared recipe session (catalog, balances, validation, live preview, fresh preparation, execution).
- `src/recipe`: editor state, recipe compilation and the wallet execution engine.
- `src/wallet`, `src/api`, `src/ui`: wallet connection, the typed API client, and shared visual pieces.

Node kinds are mapped through `Record<Kind, …>` tables rather than hand-written
lists, so a new primitive in the Rust language fails the TypeScript build here
instead of silently going unhandled at runtime.

Tests stay beside the code they cover. Imports point directly to their owning
module; there are no barrel files or extra service layers.

API shapes come from the Rust handlers and domain models, not frontend DTOs.
`pnpm codegen` exports `openapi.json` and generates `src/api/generated.ts`.
It requires Cargo but no running server or credentials. The running backend also
serves this recipe-focused contract at `/v1/openapi.json`.

```sh
pnpm codegen:check
pnpm gate
pnpm test:e2e
```

Browser tests use mocked APIs on a separate Vite port (5174) and never sign or broadcast transactions.
