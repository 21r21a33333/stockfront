# Stockfront

Stockfront is a non-custodial backend for discovering, holding, and trading tokenized stocks on BNB Chain. The current S1 release integrates Binance Web3 APIs for bStock market data, wallet portfolios, quotes, transaction preparation, RFQ orders, and settlement status.

> Development status: S1 is complete. The API is under active development and is not production-deployed.

## What works

- Browse bStocks and retrieve market and company details.
- Read a wallet's bStock positions without custodying funds.
- Quote either stablecoin → bStock or bStock → stablecoin trades.
- Prepare exact approvals and wallet-signed swap transactions.
- Prepare and submit Binance-selected RFQ orders.
- Verify swap and RFQ settlement through Binance Web3 status APIs.

Stockfront never accepts user private keys. Swap transactions are signed and broadcast by the user's wallet.

## Architecture

The Rust workspace follows hexagonal architecture:

```text
crates/core              domain models, policies, use cases, outbound ports
crates/adapters          Binance Web3 and Axum adapters
crates/app               configuration and composition root
crates/binance-web3-rwa  generated Binance Web3 client
openapi                  pinned compatibility schema and generator config
scripts                  reproducible client generation
```

Dependencies point inward:

```text
app → adapters → core
```

The generated crate must not be edited manually. Provider types are converted at the adapter boundary and never leak into the domain.

## Run locally

Requirements: stable Rust, a Binance Web3 API key and secret.

```bash
export BINANCE_WEB3_API_KEY='...'
export BINANCE_WEB3_API_SECRET='...'
cargo run -p stockfront-app
```

The server listens on `127.0.0.1:3000` by default.

Optional configuration:

| Variable | Default |
| --- | --- |
| `STOCKFRONT_BIND_ADDRESS` | `127.0.0.1:3000` |
| `BINANCE_WEB3_BASE_URL` | `https://web3.binance.com/build` |
| `STOCKFRONT_TRADE_SETTLEMENT_TOKENS` | BNB Chain USDT and USDC |
| `STOCKFRONT_MAX_TRADE_PRICE_IMPACT_BPS` | `100` |
| `STOCKFRONT_TRADE_SLIPPAGE_BPS` | `50` |

Example:

```bash
curl 'http://127.0.0.1:3000/v1/bstocks?limit=10'
```

## HTTP API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/v1/bstocks` | List and search bStocks |
| `GET` | `/v1/bstocks/{chain_id}/{contract}` | Get one bStock |
| `GET` | `/v1/wallets/{address}/portfolio` | Read bStock positions |
| `POST` | `/v1/bstock-trades/quotes` | Quote a buy or sell |
| `POST` | `/v1/bstock-trades/approval` | Prepare an exact token approval |
| `POST` | `/v1/bstock-trades/prepare` | Prepare a swap or RFQ payload |
| `POST` | `/v1/bstock-trades/orders` | Submit a signed RFQ order |
| `GET` | `/v1/bstock-trades/orders/{order_id}` | Read RFQ order status |
| `GET` | `/v1/bstock-trades/transactions/{transaction_hash}` | Verify swap settlement |

## Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Live Binance tests are ignored by default and require credentials:

```bash
cargo test -p stockfront-adapters --test binance_bstock_catalog -- --ignored
cargo test -p stockfront-adapters --test binance_wallet_portfolio -- --ignored
cargo test -p stockfront-adapters --test binance_bstock_trade -- --ignored
```

The round-trip test spends real BNB Chain USDT and BNB and must be invoked explicitly. Read the test configuration before running it.

## Regenerate the Binance client

The local OpenAPI file is a narrow compatibility subset containing only the Binance endpoints Stockfront consumes.

```bash
./scripts/generate-binance-web3-rwa.sh
git diff --exit-code -- openapi crates/binance-web3-rwa
```

Upstream schema synchronization and drift detection are planned; local schema-to-client generation is already deterministic.

## For contributors and coding agents

Read [AGENTS.md](AGENTS.md) before changing the project. It defines the vertical-slice workflow, dependency rules, boundary-type policy, generated-code ownership, testing expectations, and secret-handling requirements.

Keep changes inside the active slice, do not hand-edit `crates/binance-web3-rwa`, and never commit credentials or private keys.

## License

MIT
