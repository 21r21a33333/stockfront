// Generated from Rust. Run pnpm codegen; do not edit.
export interface paths {
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
    ActionWarning: {
      code: string;
      level: string;
      message: string;
    };
    /** @enum {string} */
    AssetKind: "stock" | "preIpo" | "etf" | "unknown";
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
    RecipeCatalog: {
      bstocks: components["schemas"]["BstockSummary"][];
      primitives: components["schemas"]["RecipePrimitiveDescriptor"][];
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
      | "UNSUPPORTED_WITHDRAWAL_POSITION";
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
    RecipeEdge: {
      from: components["schemas"]["RecipePortRef"];
      /** Format: uuid */
      id: string;
      to: components["schemas"]["RecipePortRef"];
    };
    /** @enum {string} */
    RecipeFieldControl: "TOKEN" | "AMOUNT" | "VENUE" | "POSITION";
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
    RecipeOptionSource: "SETTLEMENT_TOKENS" | "BSTOCKS" | "SUPPLIED_POSITIONS";
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
    RecipePortValueType: "TOKEN_AMOUNT" | "SUPPLIED_POSITION";
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
      "WALLET_ASSET" | "SUPPLIED_POSITION" | "TRADE" | "SUPPLY" | "WITHDRAW";
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
    Response: {
      error?: components["schemas"]["Error"];
      result?: components["schemas"]["TupleUnit"];
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
        bstocks: components["schemas"]["BstockSummary"][];
        primitives: components["schemas"]["RecipePrimitiveDescriptor"][];
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
export type ActionWarning = components["schemas"]["ActionWarning"];
export type AssetKind = components["schemas"]["AssetKind"];
export type BorrowingAccount = components["schemas"]["BorrowingAccount"];
export type BorrowingAsset = components["schemas"]["BorrowingAsset"];
export type BorrowingPosition = components["schemas"]["BorrowingPosition"];
export type BorrowingPreview = components["schemas"]["BorrowingPreview"];
export type BorrowingToken = components["schemas"]["BorrowingToken"];
export type BorrowingVenue = components["schemas"]["BorrowingVenue"];
export type BstockId = components["schemas"]["BstockId"];
export type BstockSummary = components["schemas"]["BstockSummary"];
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
export type PreparedRecipe = components["schemas"]["PreparedRecipe"];
export type PreparedRecipeStep = components["schemas"]["PreparedRecipeStep"];
export type PreparedTrade = components["schemas"]["PreparedTrade"];
export type PreparedTransaction = components["schemas"]["PreparedTransaction"];
export type RecipeAssetTransition =
  components["schemas"]["RecipeAssetTransition"];
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
export type Response = components["schemas"]["Response"];
export type Response_BorrowingAccount =
  components["schemas"]["Response_BorrowingAccount"];
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
