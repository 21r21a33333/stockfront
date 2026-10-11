// Generated from Rust. Run pnpm codegen; do not edit.
export interface paths {
  "/v1/baskets": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["listBaskets"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/baskets/trades/orders": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    post: operations["submitBasketOrder"];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/baskets/trades/orders/{order_id}": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["basketOrderStatus"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/borrowing/markets": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["markets"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/bstock-trades/orders": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    post: operations["submit_order"];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/bstock-trades/orders/{order_id}": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["order_status"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/bstock-trades/transactions/{transaction_hash}": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["transaction_status"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/carry/opportunities": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["opportunities"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/recipe-primitives": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["primitives"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/recipes/prepare": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    post: operations["prepare"];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/recipes/preview": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    post: operations["preview"];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/recipes/validate": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    post: operations["validate"];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/strategies/compose": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get?: never;
    put?: never;
    post: operations["compose"];
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/wallets/{address}/basket-positions": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["basketPositions"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/wallets/{address}/carry-positions": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["positions"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
  "/v1/wallets/{address}/loans": {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    get: operations["account"];
    put?: never;
    post?: never;
    delete?: never;
    options?: never;
    head?: never;
    patch?: never;
    trace?: never;
  };
}
export type webhooks = Record<string, never>;
export interface components {
  schemas: {
    ActionPreview: {
      balanceChanges?: components["schemas"]["AssetBalanceChange"][];
      estimatedNetworkFee?: null | components["schemas"]["NetworkFee"];
      /** Format: evm-address */
      interactWith?: `0x${string}`;
      warnings?: components["schemas"]["ActionWarning"][];
    };
    ActionWarning: {
      code: string;
      level: string;
      message: string;
    };
    AssetBalanceChange: {
      amount: string;
      token: components["schemas"]["BorrowingToken"];
      valueUsd?: string | null;
    };
    /** @enum {string} */
    AssetKind: "stock" | "preIpo" | "etf" | "unknown";
    BasketAccount: {
      /** Format: int64 */
      chainId: number;
      /** Format: int64 */
      observedAtMs: number;
      positions: components["schemas"]["BasketPosition"][];
      totalValueUsd: string;
    };
    BasketAssetAllocation: {
      /** Format: evm-address */
      contract: `0x${string}`;
      name: string;
      symbol: string;
      weightPercent: string;
    };
    BasketId: {
      /** Format: int64 */
      chainId: number;
      /** Format: evm-address */
      contract: `0x${string}`;
    };
    /** @enum {string} */
    BasketKind: "INDEX_TOKEN" | "MANAGED_PORTFOLIO";
    BasketPosition: components["schemas"]["BasketId"] & {
      kind: components["schemas"]["BasketKind"];
      name: string;
      providerId: string;
      providerName: string;
      symbol: string;
      tokenAmount: string;
      valueUsd: string;
    };
    BasketSummary: components["schemas"]["BasketId"] & {
      annualFeePercent?: string | null;
      assets: components["schemas"]["BasketAssetAllocation"][];
      categories: string[];
      description?: string | null;
      imageUrl?: string | null;
      kind: components["schemas"]["BasketKind"];
      marketCapUsd?: string | null;
      name: string;
      priceUsd?: string | null;
      providerId: string;
      providerName: string;
      symbol: string;
    };
    /**
     * @description Live safety and liquidity information for one exact Venus borrow.
     *
     *     The selected collateral is only the position the recipe starts from. Venus
     *     still evaluates every enabled collateral and every debt in the account.
     */
    BorrowCapability: {
      availableBorrowLiquidity: string;
      borrowApyPercent: string;
      /** Format: int64 */
      chainId: number;
      collateral: components["schemas"]["BorrowingToken"];
      collateralEnabled: boolean;
      conservativeMaxAmount: string;
      debt: components["schemas"]["BorrowingToken"];
      /** Format: int32 */
      debtTokenDecimals: number;
      healthFactorAfter?: string | null;
      healthFactorBefore?: string | null;
      /** Format: int64 */
      observedAtMs: number;
      /** Format: evm-address */
      pool: `0x${string}`;
      requestedAmount: string;
      requestedRawAmount: string;
      stage: components["schemas"]["BorrowStage"];
      unavailableReasons: components["schemas"]["BorrowCapabilityUnavailableReason"][];
      venue: components["schemas"]["BorrowingVenue"];
    };
    /** @enum {string} */
    BorrowCapabilityUnavailableReason:
      | "POSITION_NOT_FOUND"
      | "UNSUPPORTED_POOL"
      | "MARKET_UNLISTED"
      | "COLLATERAL_UNAVAILABLE"
      | "BORROW_PAUSED"
      | "BORROW_CAP_REACHED"
      | "INSUFFICIENT_BORROW_LIQUIDITY"
      | "INVALID_ORACLE_PRICE"
      | "HEALTH_FACTOR_UNAVAILABLE"
      | "HEALTH_FACTOR_BELOW_MINIMUM";
    /** @enum {string} */
    BorrowStage: "ENABLE_COLLATERAL" | "BORROW";
    BorrowingAccount: {
      /** Format: int64 */
      chainId: number;
      healthFactor?: string | null;
      /** Format: int64 */
      observedAtMs: number;
      positions: components["schemas"]["BorrowingPosition"][];
      totalBorrowedUsd: string;
      totalSuppliedUsd: string;
      venue: components["schemas"]["BorrowingVenue"];
    };
    BorrowingAsset: {
      amount: string;
      token: components["schemas"]["BorrowingToken"];
      valueUsd: string;
    };
    BorrowingMarket: {
      availableBorrowLiquidity: string;
      borrowApyPercent: string;
      borrowAvailable: boolean;
      /** Format: int64 */
      chainId: number;
      collateral: components["schemas"]["BorrowingToken"];
      collateralFactor: string;
      /** Format: evm-address */
      collateralMarket: `0x${string}`;
      debt: components["schemas"]["BorrowingToken"];
      /** Format: evm-address */
      debtMarket: `0x${string}`;
      liquidationThreshold: string;
      /** Format: int64 */
      observedAtMs: number;
      oracleValid: boolean;
      remainingSupplyCapacity: string;
      supplyAvailable: boolean;
      unavailableReasons: components["schemas"]["BorrowingMarketUnavailableReason"][];
      venue: components["schemas"]["BorrowingVenue"];
    };
    /** @enum {string} */
    BorrowingMarketUnavailableReason:
      | "marketUnlisted"
      | "supplyPaused"
      | "supplyCapReached"
      | "borrowPaused"
      | "borrowCapReached"
      | "insufficientBorrowLiquidity"
      | "invalidOraclePrice";
    BorrowingPosition: {
      borrowed: components["schemas"]["BorrowingAsset"][];
      /** Format: evm-address */
      pool: `0x${string}`;
      supplied: components["schemas"]["BorrowingAsset"][];
    };
    BorrowingPreview: {
      estimatedNetworkFee?: null | components["schemas"]["NetworkFee"];
      healthFactorAfter?: string | null;
      healthFactorBefore?: string | null;
      warnings?: components["schemas"]["ActionWarning"][];
    };
    BorrowingToken: {
      /** Format: evm-address */
      contract: `0x${string}`;
      symbol: string;
    };
    /** @enum {string} */
    BorrowingVenue: "venus";
    BstockId: {
      /** Format: int64 */
      chainId: number;
      /** Format: evm-address */
      contract: `0x${string}`;
    };
    BstockSummary: components["schemas"]["BstockId"] & {
      assetKind: components["schemas"]["AssetKind"];
      marketCapUsd?: string;
      marketStatus?: components["schemas"]["MarketStatus"];
      referencePriceUsd?: string;
      tags: string[];
      /** Format: int32 */
      tokenDecimals: number;
      tokenLogoUrl?: string;
      tokenName: string;
      tokenPriceUsd?: string;
      tokenSymbol: string;
      tokenToShareRatio: string;
      underlyingName: string;
      underlyingTicker: string;
      volume24hUsd?: string;
    };
    CarryAccount: {
      /** Format: int64 */
      chainId: number;
      /** Format: int64 */
      observedAtMs: number;
      positions: components["schemas"]["CarryPosition"][];
      totalValueUsd: string;
    };
    CarryActionPreview: components["schemas"]["ActionPreview"] & {
      redemptionDelay?: null | components["schemas"]["RedemptionDelay"];
    };
    CarryOpportunity: {
      asset: components["schemas"]["BorrowingToken"];
      availableBorrowLiquidity: string;
      /** Format: int64 */
      baseEarnRateBps: number;
      baseEarnRateDisplay: string;
      baseSpreadPercent?: string | null;
      borrowApyPercent: string;
      investmentId: string;
      investmentName: string;
      /** Format: int64 */
      observedAtMs: number;
      protocolId: string;
      protocolLogoUrl?: string | null;
      protocolName: string;
      qualification: components["schemas"]["CarryQualification"];
      rateType: components["schemas"]["EarnRateType"];
      reasons: components["schemas"]["CarryOpportunityReason"][];
      reportedTvlUsd?: string | null;
    };
    /** @enum {string} */
    CarryOpportunityReason:
      | "EARN_UNAVAILABLE"
      | "BORROW_UNAVAILABLE"
      | "UNSUPPORTED_RATE_TYPE"
      | "INSUFFICIENT_BASE_SPREAD"
      | "UNREVIEWED_INVESTMENT";
    CarryPosition: {
      assets: components["schemas"]["BorrowingAsset"][];
      investmentIds: string[];
      positionId: string;
      protocolId: string;
      protocolName: string;
      redemption?: null | components["schemas"]["CarryRedemption"];
      valueUsd: string;
    };
    /** @enum {string} */
    CarryQualification: "ELIGIBLE" | "REVIEW_REQUIRED" | "UNAVAILABLE";
    CarryRedemption: {
      claimable: boolean;
      id: string;
    };
    ComposeStrategyRequest: {
      previous?: null | components["schemas"]["StrategyIntent"];
      prompt: string;
      /** Format: evm-address */
      wallet?: `0x${string}`;
    };
    ComposedStrategy: {
      assumptions: string[];
      intent: components["schemas"]["StrategyIntent"];
      missingInputs: string[];
      recipe?: null | components["schemas"]["RecipeDefinition"];
      summary: string;
      validation?: null | components["schemas"]["RecipeValidationReport"];
    };
    /** @enum {string} */
    EarnRateType: "APY" | "APR";
    Error: {
      code: string;
      diagnostics?: components["schemas"]["RecipeDiagnostic"][];
      message: string;
    };
    MarketStatus: {
      /** Format: int64 */
      nextCloseAtMs?: number;
      /** Format: int64 */
      nextOpenAtMs?: number;
      reason?: string;
      reasonCode?: string;
      session?: string;
      tradable?: boolean;
    };
    NetworkFee: {
      amount: string;
      tokenSymbol: string;
      valueUsd?: string | null;
    };
    PrepareRecipeRequest: {
      context: components["schemas"]["RecipeContext"];
      recipe: components["schemas"]["RecipeDefinition"];
      selections: components["schemas"]["RecipeNodeSelection"][];
    };
    PreparedActionTransaction: {
      /** Format: hex */
      data: `0x${string}`;
      gasLimit?: string;
      gasPrice?: string;
      /** @enum {string} */
      kind:
        | "APPROVE"
        | "DEPOSIT"
        | "ENABLE_COLLATERAL"
        | "BORROW"
        | "REPAY"
        | "WITHDRAW";
      maxFeePerGas?: string;
      maxPriorityFeePerGas?: string;
      /** Format: evm-address */
      to: `0x${string}`;
      value: string;
    };
    PreparedApproval: {
      /** Format: hex */
      data: `0x${string}`;
      gasLimit?: string;
      gasPrice?: string;
      /** Format: evm-address */
      spender: `0x${string}`;
      /** Format: evm-address */
      to: `0x${string}`;
    };
    PreparedBorrowingAction: {
      preview: components["schemas"]["BorrowingPreview"];
      transactions: components["schemas"]["PreparedActionTransaction"][];
    };
    PreparedCarryDeposit: {
      opportunity: components["schemas"]["CarryOpportunity"];
      preview: components["schemas"]["CarryActionPreview"];
      transactions: {
        /** Format: hex */
        data: `0x${string}`;
        gasLimit?: string;
        gasPrice?: string;
        /** @enum {string} */
        kind: "APPROVE" | "DEPOSIT" | "REDEEM" | "CLAIM";
        maxFeePerGas?: string;
        maxPriorityFeePerGas?: string;
        /** Format: evm-address */
        to: `0x${string}`;
        value: string;
      }[];
    };
    PreparedRecipe: {
      steps: components["schemas"]["PreparedRecipeStep"][];
    };
    PreparedRecipeStep:
      | {
          action: components["schemas"]["PreparedApproval"];
          /** @enum {string} */
          kind: "APPROVAL";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          action: components["schemas"]["PreparedTrade"];
          /** @enum {string} */
          kind: "TRADE";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          action: components["schemas"]["PreparedTrade"];
          /** @enum {string} */
          kind: "BASKET_TRADE";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          action: components["schemas"]["PreparedBorrowingAction"];
          /** @enum {string} */
          kind: "SUPPLY";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          action: components["schemas"]["PreparedBorrowingAction"];
          /** @enum {string} */
          kind: "WITHDRAW";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          action: {
            preview: components["schemas"]["CarryActionPreview"];
            transactions: {
              /** Format: hex */
              data: `0x${string}`;
              gasLimit?: string;
              gasPrice?: string;
              /** @enum {string} */
              kind: "APPROVE" | "DEPOSIT" | "REDEEM" | "CLAIM";
              maxFeePerGas?: string;
              maxPriorityFeePerGas?: string;
              /** Format: evm-address */
              to: `0x${string}`;
              value: string;
            }[];
          };
          /** @enum {string} */
          kind: "EARN_WITHDRAW";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          action: components["schemas"]["PreparedBorrowingAction"];
          /** @enum {string} */
          kind: "ENABLE_COLLATERAL";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          action: components["schemas"]["PreparedBorrowingAction"];
          /** @enum {string} */
          kind: "BORROW";
          /** Format: uuid */
          nodeId: string;
          output: components["schemas"]["TokenAmount"];
        }
      | {
          action: components["schemas"]["PreparedBorrowingAction"];
          /** @enum {string} */
          kind: "REPAY";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          action: components["schemas"]["PreparedCarryDeposit"];
          /** @enum {string} */
          kind: "EARN_DEPOSIT";
          /** Format: uuid */
          nodeId: string;
        };
    PreparedTrade:
      | {
          /** @enum {string} */
          executionMode: "SWAP";
          transaction: components["schemas"]["PreparedTransaction"];
        }
      | {
          /** @enum {string} */
          executionMode: "RFQ";
          routeQuoteId: string;
          /** Format: uuid */
          submissionId: string;
          typedData: string;
          venue: components["schemas"]["RfqVenue"];
        };
    PreparedTransaction: {
      /** Format: hex */
      data: `0x${string}`;
      gasLimit: string;
      gasPrice: string;
      maxPriorityFeePerGas?: string;
      minReceiveAmount: string;
      /** Format: evm-address */
      to: `0x${string}`;
      value: string;
    };
    RecipeAssetTransition: {
      buy: components["schemas"]["RecipeOptionSource"];
      sell: components["schemas"]["RecipeOptionSource"];
    };
    RecipeBorrowPolicy: {
      allowedInputs: components["schemas"]["RecipeBorrowRule"][];
      debtTokens: `0x${string}`[];
      minimumHealthFactor: string;
    };
    RecipeBorrowRule: {
      input: components["schemas"]["RecipeOptionSource"];
      /** Format: evm-address */
      pool: `0x${string}`;
      venue: components["schemas"]["BorrowingVenue"];
    };
    RecipeCatalog: {
      baskets: components["schemas"]["BasketSummary"][];
      borrowPolicy: components["schemas"]["RecipeBorrowPolicy"];
      bstocks: components["schemas"]["BstockSummary"][];
      earnDepositPolicy: components["schemas"]["RecipeEarnDepositPolicy"];
      primitives: components["schemas"]["RecipePrimitiveDescriptor"][];
      repayPolicy: components["schemas"]["RecipeRepayPolicy"];
      /** Format: int32 */
      schemaVersion: number;
      settlementTokens: `0x${string}`[];
      supplyPolicy: components["schemas"]["RecipeSupplyPolicy"];
      tradePolicy: components["schemas"]["RecipeTradePolicy"];
      withdrawalPolicy: components["schemas"]["RecipeWithdrawalPolicy"];
    };
    RecipeCompletedNode: {
      /** Format: uuid */
      nodeId: string;
      outputs: components["schemas"]["RecipeNodeOutput"][];
    };
    RecipeContext: {
      completedNodes?: components["schemas"]["RecipeCompletedNode"][];
      /** Format: evm-address */
      wallet: `0x${string}`;
    };
    RecipeDefinition: {
      /** Format: int64 */
      chainId: number;
      edges: components["schemas"]["RecipeEdge"][];
      nodes: components["schemas"]["RecipeNode"][];
      /** Format: int32 */
      schemaVersion: number;
    };
    RecipeDiagnostic: {
      code: components["schemas"]["RecipeDiagnosticCode"];
      location: components["schemas"]["RecipeDiagnosticLocation"];
      message: string;
      severity: components["schemas"]["RecipeDiagnosticSeverity"];
    };
    /** @enum {string} */
    RecipeDiagnosticCode:
      | "EMPTY_RECIPE"
      | "UNSUPPORTED_SCHEMA_VERSION"
      | "UNSUPPORTED_CHAIN"
      | "RECIPE_TOO_LARGE"
      | "DUPLICATE_NODE_ID"
      | "DUPLICATE_EDGE_ID"
      | "UNKNOWN_NODE"
      | "UNKNOWN_PORT"
      | "INVALID_PORT_DIRECTION"
      | "PORT_TYPE_MISMATCH"
      | "MISSING_REQUIRED_INPUT"
      | "TOO_MANY_CONNECTIONS"
      | "CONSUMABLE_OUTPUT_REUSED"
      | "SELF_CONNECTION"
      | "RECIPE_CYCLE"
      | "DISCONNECTED_NODE"
      | "INVALID_AMOUNT"
      | "UNSUPPORTED_TRADE_PAIR"
      | "UNSUPPORTED_SUPPLY_ASSET"
      | "UNSUPPORTED_WITHDRAWAL_POSITION"
      | "UNSUPPORTED_BORROW_POSITION"
      | "UNSUPPORTED_BORROW_ASSET"
      | "UNSUPPORTED_REPAYMENT_POSITION"
      | "UNSUPPORTED_EARN_DEPOSIT_INPUT"
      | "UNSUPPORTED_EARN_DEPOSIT_ASSET"
      | "UNSUPPORTED_EARN_COLLATERAL"
      | "EARN_COLLATERAL_MISMATCH"
      | "MISSING_INVESTMENT_ID";
    RecipeDiagnosticLocation:
      | {
          /** @enum {string} */
          kind: "RECIPE";
        }
      | {
          /** @enum {string} */
          kind: "NODE";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          field: string;
          /** @enum {string} */
          kind: "FIELD";
          /** Format: uuid */
          nodeId: string;
        }
      | {
          /** @enum {string} */
          kind: "PORT";
          /** Format: uuid */
          nodeId: string;
          port: string;
        }
      | {
          /** Format: uuid */
          edgeId: string;
          /** @enum {string} */
          kind: "EDGE";
        };
    /** @enum {string} */
    RecipeDiagnosticSeverity: "ERROR" | "WARNING";
    /**
     * @description Structural policy for earn deposits.
     *
     *     Reviewed investment IDs and spread qualification stay backend-owned live
     *     data. Only the graph shapes the editor may build are published here.
     */
    RecipeEarnDepositPolicy: {
      allowedInputs: components["schemas"]["RecipePrimitiveKind"][];
      depositTokens: `0x${string}`[];
    };
    RecipeEdge: {
      from: components["schemas"]["RecipePortRef"];
      /** Format: uuid */
      id: string;
      to: components["schemas"]["RecipePortRef"];
    };
    /** @enum {string} */
    RecipeFieldControl:
      "TOKEN" | "AMOUNT" | "VENUE" | "POSITION" | "INVESTMENT";
    RecipeFieldDescriptor: {
      control: components["schemas"]["RecipeFieldControl"];
      key: string;
      name: string;
      optionSources: components["schemas"]["RecipeOptionSource"][];
      required: boolean;
    };
    RecipeNode: components["schemas"]["RecipePrimitive"] & {
      /** Format: uuid */
      id: string;
    };
    RecipeNodeOutput: {
      asset: components["schemas"]["TokenAmount"];
      port: string;
    };
    RecipeNodePreview:
      | {
          /** @enum {string} */
          kind: "TRADE";
          /** Format: uuid */
          nodeId: string;
          routes?: components["schemas"]["TradeQuote"][];
          state: components["schemas"]["RecipeNodeState"];
          waitingFor?: string[];
        }
      | {
          /** @enum {string} */
          kind: "SUPPLY";
          market?: null | components["schemas"]["SupplyMarket"];
          /** Format: uuid */
          nodeId: string;
          state: components["schemas"]["RecipeNodeState"];
          waitingFor?: string[];
        }
      | {
          /** @enum {string} */
          kind: "WITHDRAW";
          /** Format: uuid */
          nodeId: string;
          position?: null | components["schemas"]["BorrowingAsset"];
          state: components["schemas"]["RecipeNodeState"];
          unavailableReason?:
            null | components["schemas"]["RecipeWithdrawalUnavailableReason"];
          waitingFor?: string[];
        }
      | {
          /** @enum {string} */
          kind: "EARN_WITHDRAW";
          /** Format: uuid */
          nodeId: string;
          position?: null | components["schemas"]["CarryPosition"];
          state: components["schemas"]["RecipeNodeState"];
          waitingFor?: string[];
        }
      | {
          capability?: null | components["schemas"]["BorrowCapability"];
          /** @enum {string} */
          kind: "BORROW";
          /** Format: uuid */
          nodeId: string;
          state: components["schemas"]["RecipeNodeState"];
          waitingFor?: string[];
        }
      | {
          capability?: null | components["schemas"]["RepaymentCapability"];
          /** @enum {string} */
          kind: "REPAY";
          /** Format: uuid */
          nodeId: string;
          state: components["schemas"]["RecipeNodeState"];
          waitingFor?: string[];
        }
      | {
          /** @enum {string} */
          kind: "EARN_DEPOSIT";
          /** Format: uuid */
          nodeId: string;
          opportunity?: null | components["schemas"]["CarryOpportunity"];
          state: components["schemas"]["RecipeNodeState"];
          waitingFor?: string[];
        };
    RecipeNodeSelection: {
      /** @enum {string} */
      kind: "TRADE";
      /** Format: uuid */
      nodeId: string;
      routeQuoteId: string;
      venue: string;
    };
    /** @enum {string} */
    RecipeNodeState:
      | "READY"
      | "WAITING_FOR_UPSTREAM"
      | "COMPLETE"
      | "OUTPUT_UNAVAILABLE"
      | "UNAVAILABLE";
    /** @enum {string} */
    RecipeOptionSource:
      | "SETTLEMENT_TOKENS"
      | "BSTOCKS"
      | "SUPPLIED_POSITIONS"
      | "BORROWED_POSITIONS"
      | "EARN_OPPORTUNITIES"
      | "EARN_POSITIONS"
      | "BASKETS";
    RecipePortDescriptor: {
      direction: components["schemas"]["RecipePortDirection"];
      flow: components["schemas"]["RecipePortFlow"];
      key: string;
      /** Format: int32 */
      maxConnections?: number;
      /** Format: int32 */
      minConnections: number;
      valueType: components["schemas"]["RecipePortValueType"];
    };
    /** @enum {string} */
    RecipePortDirection: "INPUT" | "OUTPUT";
    /** @enum {string} */
    RecipePortFlow: "CONSUME" | "REFERENCE";
    RecipePortRef: {
      /** Format: uuid */
      nodeId: string;
      port: string;
    };
    /** @enum {string} */
    RecipePortValueType:
      | "TOKEN_AMOUNT"
      | "SUPPLIED_POSITION"
      | "BORROWED_POSITION"
      | "EARN_POSITION";
    RecipePreview: {
      nodes: components["schemas"]["RecipeNodePreview"][];
    };
    RecipePreviewRequest: {
      context: components["schemas"]["RecipeContext"];
      recipe: components["schemas"]["RecipeDefinition"];
    };
    RecipePrimitive:
      | {
          /** @enum {string} */
          kind: "WALLET_ASSET";
          rawAmount: string;
          /** Format: evm-address */
          token: `0x${string}`;
        }
      | {
          /** @enum {string} */
          kind: "SUPPLIED_POSITION";
          /** Format: evm-address */
          pool: `0x${string}`;
          /** Format: evm-address */
          token: `0x${string}`;
          venue: components["schemas"]["BorrowingVenue"];
        }
      | {
          /** @enum {string} */
          kind: "BORROWED_POSITION";
          /** Format: evm-address */
          pool: `0x${string}`;
          /** Format: evm-address */
          token: `0x${string}`;
          venue: components["schemas"]["BorrowingVenue"];
        }
      | {
          /** Format: evm-address */
          buyToken: `0x${string}`;
          /** @enum {string} */
          kind: "TRADE";
        }
      | {
          /** @enum {string} */
          kind: "SUPPLY";
          venue: components["schemas"]["BorrowingVenue"];
        }
      | {
          /** @enum {string} */
          kind: "WITHDRAW";
        }
      | {
          /** Format: evm-address */
          debtToken: `0x${string}`;
          /** @enum {string} */
          kind: "BORROW";
          rawAmount: string;
        }
      | {
          /** @enum {string} */
          kind: "REPAY";
        }
      | {
          /** Format: evm-address */
          collateralToken: `0x${string}`;
          investmentId: string;
          /** @enum {string} */
          kind: "EARN_DEPOSIT";
        }
      | {
          investmentId: string;
          /** @enum {string} */
          kind: "EARN_POSITION";
        }
      | {
          /** @enum {string} */
          kind: "EARN_WITHDRAW";
        };
    RecipePrimitiveDescriptor: {
      description: string;
      fields: components["schemas"]["RecipeFieldDescriptor"][];
      kind: components["schemas"]["RecipePrimitiveKind"];
      name: string;
      ports: components["schemas"]["RecipePortDescriptor"][];
    };
    /** @enum {string} */
    RecipePrimitiveKind:
      | "WALLET_ASSET"
      | "SUPPLIED_POSITION"
      | "BORROWED_POSITION"
      | "TRADE"
      | "SUPPLY"
      | "WITHDRAW"
      | "BORROW"
      | "REPAY"
      | "EARN_DEPOSIT"
      | "EARN_POSITION"
      | "EARN_WITHDRAW";
    RecipeRepayPolicy: {
      allowedInputs: components["schemas"]["RecipeRepayRule"][];
      debtTokens: `0x${string}`[];
    };
    RecipeRepayRule: {
      input: components["schemas"]["RecipeOptionSource"];
      /** Format: evm-address */
      pool: `0x${string}`;
      venue: components["schemas"]["BorrowingVenue"];
    };
    RecipeSupplyPolicy: {
      allowedInputs: components["schemas"]["RecipeSupplyRule"][];
    };
    RecipeSupplyRule: {
      input: components["schemas"]["RecipeOptionSource"];
      venue: components["schemas"]["BorrowingVenue"];
    };
    RecipeTradePolicy: {
      allowedTransitions: components["schemas"]["RecipeAssetTransition"][];
    };
    RecipeValidationReport: {
      diagnostics: components["schemas"]["RecipeDiagnostic"][];
      valid: boolean;
    };
    RecipeWithdrawalPolicy: {
      allowedInputs: components["schemas"]["RecipeWithdrawalRule"][];
    };
    RecipeWithdrawalRule: {
      /** Format: evm-address */
      pool: `0x${string}`;
      venue: components["schemas"]["BorrowingVenue"];
    };
    /** @enum {string} */
    RecipeWithdrawalUnavailableReason:
      "POSITION_NOT_FOUND" | "UNSUPPORTED_POOL" | "OUTSTANDING_DEBT";
    RedemptionDelay: {
      maxDays: string;
      minDays: string;
    };
    /**
     * @description Current ability to repay one complete Venus debt position.
     *
     *     Amounts come from the same live observation. `approval_amount` includes a
     *     small interest buffer so a full repayment does not leave debt dust while the
     *     wallet request is pending.
     */
    RepaymentCapability: {
      approvalAmount: string;
      approvalRequired: boolean;
      /** Format: int64 */
      chainId: number;
      debt: components["schemas"]["BorrowingToken"];
      debtAmount: string;
      /** Format: int64 */
      observedAtMs: number;
      /** Format: evm-address */
      pool: `0x${string}`;
      unavailableReasons: components["schemas"]["RepaymentCapabilityUnavailableReason"][];
      venue: components["schemas"]["BorrowingVenue"];
      walletBalance: string;
    };
    /** @enum {string} */
    RepaymentCapabilityUnavailableReason:
      | "POSITION_NOT_FOUND"
      | "UNSUPPORTED_POOL"
      | "UNSUPPORTED_DEBT_TOKEN"
      | "NO_DEBT"
      | "INSUFFICIENT_WALLET_BALANCE";
    Response: {
      error?: components["schemas"]["Error"];
      result?: components["schemas"]["TupleUnit"];
      status: components["schemas"]["Status"];
    };
    Response_BasketAccount: {
      error?: components["schemas"]["Error"];
      result?: {
        /** Format: int64 */
        chainId: number;
        /** Format: int64 */
        observedAtMs: number;
        positions: components["schemas"]["BasketPosition"][];
        totalValueUsd: string;
      };
      status: components["schemas"]["Status"];
    };
    Response_BorrowingAccount: {
      error?: components["schemas"]["Error"];
      result?: {
        /** Format: int64 */
        chainId: number;
        healthFactor?: string | null;
        /** Format: int64 */
        observedAtMs: number;
        positions: components["schemas"]["BorrowingPosition"][];
        totalBorrowedUsd: string;
        totalSuppliedUsd: string;
        venue: components["schemas"]["BorrowingVenue"];
      };
      status: components["schemas"]["Status"];
    };
    Response_CarryAccount: {
      error?: components["schemas"]["Error"];
      result?: {
        /** Format: int64 */
        chainId: number;
        /** Format: int64 */
        observedAtMs: number;
        positions: components["schemas"]["CarryPosition"][];
        totalValueUsd: string;
      };
      status: components["schemas"]["Status"];
    };
    Response_ComposedStrategy: {
      error?: components["schemas"]["Error"];
      result?: {
        assumptions: string[];
        intent: components["schemas"]["StrategyIntent"];
        missingInputs: string[];
        recipe?: null | components["schemas"]["RecipeDefinition"];
        summary: string;
        validation?: null | components["schemas"]["RecipeValidationReport"];
      };
      status: components["schemas"]["Status"];
    };
    Response_List_BasketSummary: {
      error?: components["schemas"]["Error"];
      result?: {
        items: (components["schemas"]["BasketId"] & {
          annualFeePercent?: string | null;
          assets: components["schemas"]["BasketAssetAllocation"][];
          categories: string[];
          description?: string | null;
          imageUrl?: string | null;
          kind: components["schemas"]["BasketKind"];
          marketCapUsd?: string | null;
          name: string;
          priceUsd?: string | null;
          providerId: string;
          providerName: string;
          symbol: string;
        })[];
      };
      status: components["schemas"]["Status"];
    };
    Response_List_BorrowingMarket: {
      error?: components["schemas"]["Error"];
      result?: {
        items: {
          availableBorrowLiquidity: string;
          borrowApyPercent: string;
          borrowAvailable: boolean;
          /** Format: int64 */
          chainId: number;
          collateral: components["schemas"]["BorrowingToken"];
          collateralFactor: string;
          /** Format: evm-address */
          collateralMarket: `0x${string}`;
          debt: components["schemas"]["BorrowingToken"];
          /** Format: evm-address */
          debtMarket: `0x${string}`;
          liquidationThreshold: string;
          /** Format: int64 */
          observedAtMs: number;
          oracleValid: boolean;
          remainingSupplyCapacity: string;
          supplyAvailable: boolean;
          unavailableReasons: components["schemas"]["BorrowingMarketUnavailableReason"][];
          venue: components["schemas"]["BorrowingVenue"];
        }[];
      };
      status: components["schemas"]["Status"];
    };
    Response_List_CarryOpportunity: {
      error?: components["schemas"]["Error"];
      result?: {
        items: {
          asset: components["schemas"]["BorrowingToken"];
          availableBorrowLiquidity: string;
          /** Format: int64 */
          baseEarnRateBps: number;
          baseEarnRateDisplay: string;
          baseSpreadPercent?: string | null;
          borrowApyPercent: string;
          investmentId: string;
          investmentName: string;
          /** Format: int64 */
          observedAtMs: number;
          protocolId: string;
          protocolLogoUrl?: string | null;
          protocolName: string;
          qualification: components["schemas"]["CarryQualification"];
          rateType: components["schemas"]["EarnRateType"];
          reasons: components["schemas"]["CarryOpportunityReason"][];
          reportedTvlUsd?: string | null;
        }[];
      };
      status: components["schemas"]["Status"];
    };
    Response_PreparedRecipe: {
      error?: components["schemas"]["Error"];
      result?: {
        steps: components["schemas"]["PreparedRecipeStep"][];
      };
      status: components["schemas"]["Status"];
    };
    Response_RecipeCatalog: {
      error?: components["schemas"]["Error"];
      result?: {
        baskets: components["schemas"]["BasketSummary"][];
        borrowPolicy: components["schemas"]["RecipeBorrowPolicy"];
        bstocks: components["schemas"]["BstockSummary"][];
        earnDepositPolicy: components["schemas"]["RecipeEarnDepositPolicy"];
        primitives: components["schemas"]["RecipePrimitiveDescriptor"][];
        repayPolicy: components["schemas"]["RecipeRepayPolicy"];
        /** Format: int32 */
        schemaVersion: number;
        settlementTokens: `0x${string}`[];
        supplyPolicy: components["schemas"]["RecipeSupplyPolicy"];
        tradePolicy: components["schemas"]["RecipeTradePolicy"];
        withdrawalPolicy: components["schemas"]["RecipeWithdrawalPolicy"];
      };
      status: components["schemas"]["Status"];
    };
    Response_RecipePreview: {
      error?: components["schemas"]["Error"];
      result?: {
        nodes: components["schemas"]["RecipeNodePreview"][];
      };
      status: components["schemas"]["Status"];
    };
    Response_RecipeValidationReport: {
      error?: components["schemas"]["Error"];
      result?: {
        diagnostics: components["schemas"]["RecipeDiagnostic"][];
        valid: boolean;
      };
      status: components["schemas"]["Status"];
    };
    Response_SubmittedOrder: {
      error?: components["schemas"]["Error"];
      result?: {
        /** Format: int64 */
        createdAtMs: number;
        orderId: string;
        state: components["schemas"]["TradeOrderState"];
      };
      status: components["schemas"]["Status"];
    };
    Response_TradeOrderStatus: {
      error?: components["schemas"]["Error"];
      result?: {
        /** Format: int64 */
        createdAtMs?: number;
        /** Format: int64 */
        filledAtMs?: number;
        filledBuyAmount?: string;
        filledSellAmount?: string;
        state: components["schemas"]["TradeOrderState"];
        /** Format: hex */
        transactionHash?: `0x${string}`;
      };
      status: components["schemas"]["Status"];
    };
    Response_TradeTransactionStatus: {
      error?: components["schemas"]["Error"];
      result?: {
        /** Format: int64 */
        blockNumber?: number;
        bought?: components["schemas"]["SettledTokenAmount"][];
        error?: string;
        /** Format: int64 */
        executedAtMs?: number;
        sold?: components["schemas"]["SettledTokenAmount"][];
        state: components["schemas"]["TradeTransactionState"];
      };
      status: components["schemas"]["Status"];
    };
    /** @enum {string} */
    RfqVenue: "inchFusion" | "cowSwap" | "pcsXRfq";
    SettledTokenAmount: {
      amount: string;
      /** Format: evm-address */
      token: `0x${string}`;
    };
    /** @enum {string} */
    Status: "Ok" | "Error";
    /**
     * @description A strategy in human terms: symbols and decimal amounts, ordered as a chain.
     *
     *     Addresses, raw units and graph edges are deliberately absent. They are
     *     derived by Stockfront from live data so a language model cannot invent them.
     */
    StrategyIntent: {
      steps: components["schemas"]["StrategyStep"][];
    };
    StrategyStep:
      | {
          amount?: string | null;
          /** @enum {string} */
          kind: "WALLET_ASSET";
          symbol: string;
        }
      | {
          /** @enum {string} */
          kind: "SUPPLIED_POSITION";
          symbol: string;
        }
      | {
          /** @enum {string} */
          kind: "BORROWED_POSITION";
          symbol: string;
        }
      | {
          buySymbol: string;
          /** @enum {string} */
          kind: "TRADE";
        }
      | {
          /** @enum {string} */
          kind: "SUPPLY";
        }
      | {
          /** @enum {string} */
          kind: "WITHDRAW";
        }
      | {
          amount?: string | null;
          /** @enum {string} */
          kind: "BORROW";
          symbol: string;
        }
      | {
          /** @enum {string} */
          kind: "REPAY";
        }
      | {
          /** @enum {string} */
          kind: "EARN_DEPOSIT";
          symbol: string;
        };
    SubmitOrderRequest: {
      routeQuoteId: string;
      signature: string;
      /** Format: uuid */
      submissionId: string;
      venue: components["schemas"]["RfqVenue"];
    };
    SubmittedOrder: {
      /** Format: int64 */
      createdAtMs: number;
      orderId: string;
      state: components["schemas"]["TradeOrderState"];
    };
    /**
     * @description Current Venus capability for supplying one supported bStock.
     *
     *     `remaining_supply_capacity` is `None` when Venus reports an unlimited cap.
     *     This model intentionally excludes collateral and borrowing fields: supplying
     *     an asset does not require it to be collateral-eligible or borrowable.
     */
    SupplyMarket: {
      /** Format: int64 */
      chainId: number;
      /** Format: evm-address */
      market: `0x${string}`;
      /** Format: int64 */
      observedAtMs: number;
      remainingSupplyCapacity?: string | null;
      supplyApyPercent: string;
      supplyAvailable: boolean;
      token: components["schemas"]["BorrowingToken"];
      /** Format: int32 */
      tokenDecimals: number;
      unavailableReasons: components["schemas"]["SupplyMarketUnavailableReason"][];
      venue: components["schemas"]["BorrowingVenue"];
    };
    /** @enum {string} */
    SupplyMarketUnavailableReason:
      "MARKET_UNLISTED" | "SUPPLY_PAUSED" | "SUPPLY_CAP_REACHED";
    TokenAmount: {
      amount: string;
      /** Format: evm-address */
      token: `0x${string}`;
    };
    /** @enum {string} */
    TradeExecutionMode: "SWAP" | "RFQ";
    /** @enum {string} */
    TradeOrderState:
      | "pendingVendor"
      | "pendingOnchain"
      | "filled"
      | "failed"
      | "expired"
      | "cancelled"
      | "unknown";
    TradeOrderStatus: {
      /** Format: int64 */
      createdAtMs?: number;
      /** Format: int64 */
      filledAtMs?: number;
      filledBuyAmount?: string;
      filledSellAmount?: string;
      state: components["schemas"]["TradeOrderState"];
      /** Format: hex */
      transactionHash?: `0x${string}`;
    };
    TradeQuote: {
      /** Format: evm-address */
      approvalTarget?: `0x${string}`;
      buy: components["schemas"]["TokenAmount"];
      executionMode: components["schemas"]["TradeExecutionMode"];
      priceImpactPercent?: string;
      routeQuoteId: string;
      sell: components["schemas"]["TokenAmount"];
      venue: string;
    };
    /** @enum {string} */
    TradeTransactionState: "pending" | "succeeded" | "failed" | "unknown";
    TradeTransactionStatus: {
      /** Format: int64 */
      blockNumber?: number;
      bought?: components["schemas"]["SettledTokenAmount"][];
      error?: string;
      /** Format: int64 */
      executedAtMs?: number;
      sold?: components["schemas"]["SettledTokenAmount"][];
      state: components["schemas"]["TradeTransactionState"];
    };
    /** @default null */
    TupleUnit: unknown;
  };
  responses: never;
  parameters: never;
  requestBodies: never;
  headers: never;
  pathItems: never;
}
export type ActionPreview = components["schemas"]["ActionPreview"];
export type ActionWarning = components["schemas"]["ActionWarning"];
export type AssetBalanceChange = components["schemas"]["AssetBalanceChange"];
export type AssetKind = components["schemas"]["AssetKind"];
export type BasketAccount = components["schemas"]["BasketAccount"];
export type BasketAssetAllocation =
  components["schemas"]["BasketAssetAllocation"];
export type BasketId = components["schemas"]["BasketId"];
export type BasketKind = components["schemas"]["BasketKind"];
export type BasketPosition = components["schemas"]["BasketPosition"];
export type BasketSummary = components["schemas"]["BasketSummary"];
export type BorrowCapability = components["schemas"]["BorrowCapability"];
export type BorrowCapabilityUnavailableReason =
  components["schemas"]["BorrowCapabilityUnavailableReason"];
export type BorrowStage = components["schemas"]["BorrowStage"];
export type BorrowingAccount = components["schemas"]["BorrowingAccount"];
export type BorrowingAsset = components["schemas"]["BorrowingAsset"];
export type BorrowingMarket = components["schemas"]["BorrowingMarket"];
export type BorrowingMarketUnavailableReason =
  components["schemas"]["BorrowingMarketUnavailableReason"];
export type BorrowingPosition = components["schemas"]["BorrowingPosition"];
export type BorrowingPreview = components["schemas"]["BorrowingPreview"];
export type BorrowingToken = components["schemas"]["BorrowingToken"];
export type BorrowingVenue = components["schemas"]["BorrowingVenue"];
export type BstockId = components["schemas"]["BstockId"];
export type BstockSummary = components["schemas"]["BstockSummary"];
export type CarryAccount = components["schemas"]["CarryAccount"];
export type CarryActionPreview = components["schemas"]["CarryActionPreview"];
export type CarryOpportunity = components["schemas"]["CarryOpportunity"];
export type CarryOpportunityReason =
  components["schemas"]["CarryOpportunityReason"];
export type CarryPosition = components["schemas"]["CarryPosition"];
export type CarryQualification = components["schemas"]["CarryQualification"];
export type CarryRedemption = components["schemas"]["CarryRedemption"];
export type ComposeStrategyRequest =
  components["schemas"]["ComposeStrategyRequest"];
export type ComposedStrategy = components["schemas"]["ComposedStrategy"];
export type EarnRateType = components["schemas"]["EarnRateType"];
export type Error = components["schemas"]["Error"];
export type MarketStatus = components["schemas"]["MarketStatus"];
export type NetworkFee = components["schemas"]["NetworkFee"];
export type PrepareRecipeRequest =
  components["schemas"]["PrepareRecipeRequest"];
export type PreparedActionTransaction =
  components["schemas"]["PreparedActionTransaction"];
export type PreparedApproval = components["schemas"]["PreparedApproval"];
export type PreparedBorrowingAction =
  components["schemas"]["PreparedBorrowingAction"];
export type PreparedCarryDeposit =
  components["schemas"]["PreparedCarryDeposit"];
export type PreparedRecipe = components["schemas"]["PreparedRecipe"];
export type PreparedRecipeStep = components["schemas"]["PreparedRecipeStep"];
export type PreparedTrade = components["schemas"]["PreparedTrade"];
export type PreparedTransaction = components["schemas"]["PreparedTransaction"];
export type RecipeAssetTransition =
  components["schemas"]["RecipeAssetTransition"];
export type RecipeBorrowPolicy = components["schemas"]["RecipeBorrowPolicy"];
export type RecipeBorrowRule = components["schemas"]["RecipeBorrowRule"];
export type RecipeCatalog = components["schemas"]["RecipeCatalog"];
export type RecipeCompletedNode = components["schemas"]["RecipeCompletedNode"];
export type RecipeContext = components["schemas"]["RecipeContext"];
export type RecipeDefinition = components["schemas"]["RecipeDefinition"];
export type RecipeDiagnostic = components["schemas"]["RecipeDiagnostic"];
export type RecipeDiagnosticCode =
  components["schemas"]["RecipeDiagnosticCode"];
export type RecipeDiagnosticLocation =
  components["schemas"]["RecipeDiagnosticLocation"];
export type RecipeDiagnosticSeverity =
  components["schemas"]["RecipeDiagnosticSeverity"];
export type RecipeEarnDepositPolicy =
  components["schemas"]["RecipeEarnDepositPolicy"];
export type RecipeEdge = components["schemas"]["RecipeEdge"];
export type RecipeFieldControl = components["schemas"]["RecipeFieldControl"];
export type RecipeFieldDescriptor =
  components["schemas"]["RecipeFieldDescriptor"];
export type RecipeNode = components["schemas"]["RecipeNode"];
export type RecipeNodeOutput = components["schemas"]["RecipeNodeOutput"];
export type RecipeNodePreview = components["schemas"]["RecipeNodePreview"];
export type RecipeNodeSelection = components["schemas"]["RecipeNodeSelection"];
export type RecipeNodeState = components["schemas"]["RecipeNodeState"];
export type RecipeOptionSource = components["schemas"]["RecipeOptionSource"];
export type RecipePortDescriptor =
  components["schemas"]["RecipePortDescriptor"];
export type RecipePortDirection = components["schemas"]["RecipePortDirection"];
export type RecipePortFlow = components["schemas"]["RecipePortFlow"];
export type RecipePortRef = components["schemas"]["RecipePortRef"];
export type RecipePortValueType = components["schemas"]["RecipePortValueType"];
export type RecipePreview = components["schemas"]["RecipePreview"];
export type RecipePreviewRequest =
  components["schemas"]["RecipePreviewRequest"];
export type RecipePrimitive = components["schemas"]["RecipePrimitive"];
export type RecipePrimitiveDescriptor =
  components["schemas"]["RecipePrimitiveDescriptor"];
export type RecipePrimitiveKind = components["schemas"]["RecipePrimitiveKind"];
export type RecipeRepayPolicy = components["schemas"]["RecipeRepayPolicy"];
export type RecipeRepayRule = components["schemas"]["RecipeRepayRule"];
export type RecipeSupplyPolicy = components["schemas"]["RecipeSupplyPolicy"];
export type RecipeSupplyRule = components["schemas"]["RecipeSupplyRule"];
export type RecipeTradePolicy = components["schemas"]["RecipeTradePolicy"];
export type RecipeValidationReport =
  components["schemas"]["RecipeValidationReport"];
export type RecipeWithdrawalPolicy =
  components["schemas"]["RecipeWithdrawalPolicy"];
export type RecipeWithdrawalRule =
  components["schemas"]["RecipeWithdrawalRule"];
export type RecipeWithdrawalUnavailableReason =
  components["schemas"]["RecipeWithdrawalUnavailableReason"];
export type RedemptionDelay = components["schemas"]["RedemptionDelay"];
export type RepaymentCapability = components["schemas"]["RepaymentCapability"];
export type RepaymentCapabilityUnavailableReason =
  components["schemas"]["RepaymentCapabilityUnavailableReason"];
export type Response = components["schemas"]["Response"];
export type Response_BasketAccount =
  components["schemas"]["Response_BasketAccount"];
export type Response_BorrowingAccount =
  components["schemas"]["Response_BorrowingAccount"];
export type Response_CarryAccount =
  components["schemas"]["Response_CarryAccount"];
export type Response_ComposedStrategy =
  components["schemas"]["Response_ComposedStrategy"];
export type Response_List_BasketSummary =
  components["schemas"]["Response_List_BasketSummary"];
export type Response_List_BorrowingMarket =
  components["schemas"]["Response_List_BorrowingMarket"];
export type Response_List_CarryOpportunity =
  components["schemas"]["Response_List_CarryOpportunity"];
export type Response_PreparedRecipe =
  components["schemas"]["Response_PreparedRecipe"];
export type Response_RecipeCatalog =
  components["schemas"]["Response_RecipeCatalog"];
export type Response_RecipePreview =
  components["schemas"]["Response_RecipePreview"];
export type Response_RecipeValidationReport =
  components["schemas"]["Response_RecipeValidationReport"];
export type Response_SubmittedOrder =
  components["schemas"]["Response_SubmittedOrder"];
export type Response_TradeOrderStatus =
  components["schemas"]["Response_TradeOrderStatus"];
export type Response_TradeTransactionStatus =
  components["schemas"]["Response_TradeTransactionStatus"];
export type RfqVenue = components["schemas"]["RfqVenue"];
export type SettledTokenAmount = components["schemas"]["SettledTokenAmount"];
export type Status = components["schemas"]["Status"];
export type StrategyIntent = components["schemas"]["StrategyIntent"];
export type StrategyStep = components["schemas"]["StrategyStep"];
export type SubmitOrderRequest = components["schemas"]["SubmitOrderRequest"];
export type SubmittedOrder = components["schemas"]["SubmittedOrder"];
export type SupplyMarket = components["schemas"]["SupplyMarket"];
export type SupplyMarketUnavailableReason =
  components["schemas"]["SupplyMarketUnavailableReason"];
export type TokenAmount = components["schemas"]["TokenAmount"];
export type TradeExecutionMode = components["schemas"]["TradeExecutionMode"];
export type TradeOrderState = components["schemas"]["TradeOrderState"];
export type TradeOrderStatus = components["schemas"]["TradeOrderStatus"];
export type TradeQuote = components["schemas"]["TradeQuote"];
export type TradeTransactionState =
  components["schemas"]["TradeTransactionState"];
export type TradeTransactionStatus =
  components["schemas"]["TradeTransactionStatus"];
export type TupleUnit = components["schemas"]["TupleUnit"];
export type $defs = Record<string, never>;
export interface operations {
  listBaskets: {
    parameters: {
      query?: {
        search?: string;
        category?: string;
        limit?: number;
      };
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_List_BasketSummary"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  submitBasketOrder: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        "application/json": components["schemas"]["SubmitOrderRequest"];
      };
    };
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_SubmittedOrder"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  basketOrderStatus: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        order_id: string;
      };
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_TradeOrderStatus"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  markets: {
    parameters: {
      query?: {
        collateralToken?: string;
        debtToken?: string;
      };
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_List_BorrowingMarket"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  submit_order: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        "application/json": components["schemas"]["SubmitOrderRequest"];
      };
    };
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_SubmittedOrder"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  order_status: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        order_id: string;
      };
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_TradeOrderStatus"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  transaction_status: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        transaction_hash: string;
      };
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_TradeTransactionStatus"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  opportunities: {
    parameters: {
      query: {
        collateralToken: string;
        debtToken: string;
      };
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_List_CarryOpportunity"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  primitives: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_RecipeCatalog"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  prepare: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        "application/json": components["schemas"]["PrepareRecipeRequest"];
      };
    };
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_PreparedRecipe"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  preview: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        "application/json": components["schemas"]["RecipePreviewRequest"];
      };
    };
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_RecipePreview"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  validate: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        "application/json": components["schemas"]["RecipeDefinition"];
      };
    };
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_RecipeValidationReport"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  compose: {
    parameters: {
      query?: never;
      header?: never;
      path?: never;
      cookie?: never;
    };
    requestBody: {
      content: {
        "application/json": components["schemas"]["ComposeStrategyRequest"];
      };
    };
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_ComposedStrategy"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  basketPositions: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        address: string;
      };
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_BasketAccount"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  positions: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        address: string;
      };
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_CarryAccount"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
  account: {
    parameters: {
      query?: never;
      header?: never;
      path: {
        address: string;
      };
      cookie?: never;
    };
    requestBody?: never;
    responses: {
      200: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response_BorrowingAccount"];
        };
      };
      default: {
        headers: {
          [name: string]: unknown;
        };
        content: {
          "application/json": components["schemas"]["Response"];
        };
      };
    };
  };
}
