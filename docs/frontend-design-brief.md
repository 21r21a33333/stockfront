# Stockfront: UI Design Brief

## The idea

**Stockfront, "More from every stock."** Tokenized stocks (bStocks) on BNB Chain can do more than sit in a wallet. In Stockfront you describe what you want in plain English (*"buy $20 of Tesla, borrow USDT against it and earn on it"*). The AI turns it into a **strategy made of building blocks**, you see every step, and you sign each one in your own wallet. Stockfront never holds your funds.

The whole app is about **strategies**: composing them with AI, shaping them with blocks, and running them step by step. It must **not** look like another finance dashboard full of tables and tabs.

## The feel

- **Simple, striking, a little bit magical.** One big idea per screen, lots of breathing room, and one or two "wow" moments rather than many widgets.
- **The signature visual:** a sentence that transforms into a chain of glowing blocks connected by flowing lines. This motif ties the landing page, the AI composer and the builder together.
- **Calm and premium:** like a modern brokerage crossed with a creative tool (think Linear, Arc, Raycast), not a crypto trading terminal.
- **Motion:** smooth and purposeful.
  - Blocks assemble one by one when the AI answers.
  - Lines draw between blocks.
  - Steps tick off with a soft check animation.
  - Numbers ease when they update.
  - Short 150–300 ms transitions; nothing bouncy or noisy.
- **Colour direction, "Ink & Paper":**
  - Warm paper background `#F7F5F0`, white cards, deep ink text `#16181D`.
  - Ink navy `#1B2A4A` as the main action colour.
  - **Saffron `#E8A33D`** as the single bright accent: AI moments, active states, highlights.
  - Gains green `#1F9D6B`, losses red `#D9534F`.
  - Each block type has its own soft accent: Trade copper, Supply slate blue, Borrow ochre, Repay plum, Withdraw cyan, Earn sage green.
  - Dark mode "Midnight": `#0D1117` background, saffron glow accents. It looks great for the landing hero.
- **Fonts:** **Manrope** for UI and headings (large, confident headlines), **JetBrains Mono** for amounts, prices and addresses.
- **Desktop-first** (1440px). Mobile should still read well (stacked layout, sheets instead of side panels).

## Screens

### 1. Landing page

One scroll; short and memorable.

- **Hero:**
  - Headline "More from every stock."
  - Subline: "Describe a strategy. See it as blocks. Sign every step."
  - A large **AI prompt box** right in the hero, with example chips:
    - "Buy $20 of Tesla"
    - "Borrow USDT against my Nvidia stock"
    - "Borrow and earn on my stock"
    - "Repay my loan and get my stock back"
  - Primary button "Start building", secondary "Connect wallet".
- **Hero animation:** a typed sentence dissolves into a chain of blocks: `Wallet USDT → Buy TSLA → Supply to Venus → Borrow USDT → Earn`, with lines flowing between them.
- **How it works:** three simple panels.
  1. **Describe:** tell the AI what you want.
  2. **Shape:** adjust the blocks, see live prices, health and yield.
  3. **Sign:** approve each step in your wallet; nothing runs on its own.
- **The blocks:** a playful row of the six action blocks (Trade, Supply, Borrow, Repay, Withdraw, Earn), each with an icon and a one-line description. On hover, a block lifts and shows an example.
- **Trust strip:** You sign everything · Your funds never leave your wallet · Live BNB Chain data · Built on Binance Web3, Venus and PancakeSwap.
- **Footer:** short risk note ("Returns aren't guaranteed. Tokenized stocks and DeFi carry risk."), links.

### 2. Compose (app home, the AI composer)

The first screen after "Start building".

- Centred, generous **prompt box** ("What do you want your stocks to do?") with the example chips below.
- **Strategy templates:** 4 cards, each showing its block chain in miniature:
  - **Buy a stock:** Wallet USDT → Trade.
  - **Borrow against a stock:** Supplied stock → Borrow.
  - **Borrow & earn:** Supplied stock → Borrow USDT → Earn.
  - **Repay & reclaim:** Debt → Repay, then Withdraw.
- **AI thinking state:** an elegant animated indicator with quiet status lines ("Reading your request…", "Checking live prices…", "Validating strategy…").
- **AI plan card** (the result):
  - A one-sentence summary ("Buy ~0.025 NVDA with 6 USDT, supply it to Venus as collateral").
  - The strategy as **assembled blocks**, animating in one by one.
  - Key numbers: amounts, expected rate or spread, health factor after borrowing.
  - Assumptions and warnings, each in one short line.
  - A status pill: "Ready", or "Needs your input" (e.g. choose an amount).
  - Buttons: **Open in builder** (primary), **Refine** (follow-up prompt), **Start over**.
- A small persistent note: *"AI proposes. You review and sign. Nothing runs automatically."*
- **Recent strategies:** a compact row of saved drafts.

### 3. Builder (the centrepiece)

A beautiful canvas where the strategy lives.

- **Left: block palette** (slim, collapsible).
  - **Start from:** Wallet asset, Supplied stock (a position already on Venus), Debt (a loan already on Venus).
  - **Actions:** Trade, Supply, Borrow, Repay, Withdraw, Earn. **Liquidity** is greyed out as "Soon".
  - Drag a block onto the canvas or click to add.
- **Canvas:**
  - Blocks are rounded cards with a coloured icon, a title ("Buy NVDA") and one line of live detail ("~0.0255 NVDA for 6 USDT").
  - Connections are smooth curved lines with a small label of what flows through ("USDT", "NVDA", "Supplied TSLA").
  - An open output shows a ghost "+" that suggests the next sensible block.
  - Subtle dotted background, zoom and fit controls.
- **Right: inspector** for the selected block, written for humans. **Live market info lives inside the blocks** instead of on separate pages.
  - **Wallet asset:** token picker with balance, amount input with 25/50/75/Max.
  - **Trade:**
    - What you sell and what you buy.
    - The stock picker shows each stock's price, its **gap vs the real stock price**, and a **market open/closed badge** (e.g. "US market closed · opens Mon 9:30 ET").
    - Route options (venue, instant swap or signed order, amount received, price impact, minimum received).
  - **Supply:** venue (Venus), amount from the previous block, "Becomes collateral you can borrow against".
  - **Borrow:**
    - Stablecoin and amount, with a **"Safe max"** button.
    - A **health-factor gauge** showing before → after with Safe / Caution / At-risk zones.
    - Borrow rate and available liquidity.
    - A two-stage note: "First enable collateral, then borrow".
    - Blocked state with a clear reason if the health factor would fall below 1.5.
  - **Earn:**
    - List of eligible earn options showing **earn rate, borrow rate and net spread** (highlighted), plus the protocol name.
    - Non-eligible options appear faded with the reason ("Spread too low", "Not reviewed yet").
  - **Repay:** "Repays your full debt", current debt, wallet balance, and whether an approval is needed.
  - **Withdraw:** "Withdraws all of your supplied stock". Blocked with an explanation while debt remains.
  - Every block has a small **"Checks"** area: green when fine, plain-English issues when not.
- **Top bar:**
  - Strategy name (editable), status pill (Draft / Ready / Needs attention).
  - Buttons: **Ask AI** (reopens the composer to modify this strategy), **Preview**, **Run** (primary).

### 4. Run (execute the strategy)

Opens as a focused panel or full screen when the user presses **Run**.

- **A vertical step tracker** mirroring the blocks:
  - `1 Approve USDT` → `2 Buy NVDA` → `3 Approve NVDA` → `4 Supply to Venus`, and so on.
  - Combined steps where the wallet can batch them ("Approve + Buy").
- **Each step has a review card** before signing:
  - **You pay / You get** with token icons and dollar values.
  - Network fee.
  - Health factor before → after, when relevant.
  - "Interacting with Venus" plus a shortened contract address.
  - A collapsed "Details" section for the technical info.
- **Step states:**
  - Up next.
  - Waiting for your wallet (gentle pulse).
  - Confirming on-chain (progress with an explorer link).
  - Done (animated check).
  - Failed (reason plus "Try again").
- **Special moments:**
  - **Price changed:** "You'll now get 0.0251 NVDA (was 0.0255)" with **Accept new price** or **Cancel**. Never silently.
  - **Cancelled in wallet:** "No problem, nothing was sent" with Try again.
  - **Result unknown:** "We couldn't confirm this step" with Check again or Abandon run.
- **Finish screen:** a satisfying completion moment (blocks light up in sequence), then a **"What you own now"** card:
  - stock held;
  - collateral supplied;
  - debt and health factor;
  - earn position and its rate.

  Then a "Build another strategy" button.

### 5. Positions sheet (slide-over, not a page)

Opened from the top bar. A light, read-only snapshot.

- **Stocks in wallet:** amount, value, and how many real shares that represents.
- **On Venus:** supplied stocks, borrowed stablecoins, and the health factor gauge.
- **Earning:** earn positions, value, and claimable or pending withdrawals.
- Each item has a **"Use in a strategy"** button that opens the builder starting from it (e.g. a loan opens with Debt → Repay).
- **Empty state:** "Nothing here yet. Start with your first strategy."

### 6. Global bits

- **Top bar:**
  - Logo; nav with only **Compose** and **Builder**.
  - **Positions** button, network badge (BNB Chain), **wallet button** (address, balance; copy, view on explorer, disconnect).
- **Connect wallet modal:** wallet choices and a "No wallet found" state with install help. Short line: "Stockfront never holds your funds."
- **Wrong network:** a friendly banner with "Switch to BNB Chain".
- **Toasts:** step submitted, confirmed, failed.
- **Loading:** soft skeleton shimmer shaped like the final content.
- **Empty and error states:** a friendly illustration, one sentence and one action.

## Content and copy rules

- Plain language: "Borrow against your stock", not protocol jargon. Protocol names (Venus, PancakeSwap, Lista) appear as small secondary labels.
- Money: `$1,234.56`; token amounts trimmed to 4–6 decimals; rates like `5.12%`; spread `+5.79%` in green or `−0.40%` in red.
- Health factor: number plus a word (Safe / Caution / At risk).
- Addresses shortened: `0x8894…d4e3`.
- Never promise returns.

## Not in this design

Separate markets, portfolio or analytics dashboards, data-heavy tables, leverage or perpetual trading, and anything where the app acts without the user signing.

## Sample data for realistic mockups

- Stocks: TSLAB (Tesla), NVDAB (Nvidia), AAOIB (Applied Optoelectronics). Stablecoins: USDT, USDC.
- Trade: 6 USDT → 0.0255 NVDAB, venue "LiquidMesh", price impact 0.0008%, minimum received 0.0254 NVDAB.
- Borrow: TSLAB collateral, borrow 1 USDT at 5.12% APR, health factor 3.40 → 2.85, safe max 4.2 USDT.
- Earn: Lista USDT, earn 10.91%, borrow 5.12%, **net spread +5.79%**.
- Market status: "US market closed · opens Mon 9:30 ET". Price gap example: NVDAB $234.90 vs NVDA $235.40 (−0.21%).
