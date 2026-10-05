# Recipe builder

```sh
pnpm install
pnpm dev
```

Vite forwards `/api` to the Stockfront backend. See `.env.example` for local configuration.

## Layout

- `src/api`: typed HTTP client and generated API types.
- `src/recipe`: page composition, header, action palette and shared styling.
  - `canvas/`: graph rendering, nodes and asset edges.
  - `inspector/`: asset configuration, route preview and transaction controls.
  - `editor/`: draft state, graph types and conversion to API recipes.
- `src/wallet`: connection and wallet-signed execution.
- `src/ui/assets`: shared token selector, identity components and their styles.

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
