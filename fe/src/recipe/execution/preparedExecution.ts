import type {
  PreparedRecipe,
  RecipeNodeOutput,
  SubmitOrderRequest,
} from "@/api/generated";
import type { Address, Hex, WalletClient } from "viem";
import { ExecutionMessage } from "./model";
import {
  hasSufficientAllowance,
  simulatePreparedTransaction,
} from "./preflight";

export type PreparedExecution =
  | {
      kind: "CALL_BATCH";
      nodeId: string;
      action:
        | "APPROVAL"
        | "TRADE"
        | "SUPPLY"
        | "EARN_DEPOSIT"
        | "WITHDRAW"
        | "ENABLE_COLLATERAL"
        | "BORROW"
        | "REPAY";
      executionMode:
        "SWAP" | "SUPPLY" | "EARN_DEPOSIT" | "WITHDRAW" | "BORROW" | "REPAY";
      steps: (
        | "APPROVAL"
        | "TRADE"
        | "SUPPLY"
        | "EARN_DEPOSIT"
        | "WITHDRAW"
        | "ENABLE_COLLATERAL"
        | "BORROW"
        | "REPAY"
      )[];
      calls: { to: Address; data: Hex; value?: bigint }[];
      atomicRequired: boolean;
      output?: RecipeNodeOutput;
    }
  | {
      kind: "RFQ";
      nodeId: string;
      action: "TRADE";
      executionMode: "RFQ";
      steps: ["TRADE"];
      order: Omit<SubmitOrderRequest, "signature"> & { typedData: string };
    };

type RpcClient = Parameters<typeof hasSufficientAllowance>[0];

/**
 * Turns a backend preparation into one user-visible wallet action.
 * An approval and its swap are never exposed as independent submissions.
 */
export async function preparedExecution(
  prepared: PreparedRecipe,
  client: RpcClient,
  owner: Address,
): Promise<PreparedExecution> {
  const nodes = new Set(prepared.steps.map((step) => step.nodeId));
  if (nodes.size !== 1)
    throw new ExecutionMessage("Prepare one recipe action at a time.");
  const nodeId = prepared.steps[0]?.nodeId;
  if (!nodeId)
    throw new ExecutionMessage("No prepared recipe action is available.");

  const approval = prepared.steps.find((step) => step.kind === "APPROVAL");
  const trade = prepared.steps.find((step) => step.kind === "TRADE");
  const supply = prepared.steps.find((step) => step.kind === "SUPPLY");
  const earn = prepared.steps.find((step) => step.kind === "EARN_DEPOSIT");
  const withdrawal = prepared.steps.find((step) => step.kind === "WITHDRAW");
  const collateral = prepared.steps.find(
    (step) => step.kind === "ENABLE_COLLATERAL",
  );
  const borrow = prepared.steps.find((step) => step.kind === "BORROW");
  const repayment = prepared.steps.find((step) => step.kind === "REPAY");

  if (repayment?.kind === "REPAY") {
    if (prepared.steps.length !== 1)
      throw new ExecutionMessage(
        "A repayment preparation must contain one recipe step.",
      );
    const calls = repaymentCalls(repayment.action.transactions);
    const approvalRequired = calls.length === 2;
    return {
      kind: "CALL_BATCH",
      nodeId,
      action: "REPAY",
      executionMode: "REPAY",
      steps: approvalRequired ? ["APPROVAL", "REPAY"] : ["REPAY"],
      calls,
      atomicRequired: approvalRequired,
    };
  }

  if (collateral?.kind === "ENABLE_COLLATERAL") {
    const transaction = oneBorrowingTransaction(
      prepared,
      collateral.action.transactions,
      "ENABLE_COLLATERAL",
      "collateral activation",
    );
    await simulatePreparedTransaction(client, owner, transaction);
    return {
      kind: "CALL_BATCH",
      nodeId,
      action: "ENABLE_COLLATERAL",
      executionMode: "BORROW",
      steps: ["ENABLE_COLLATERAL"],
      calls: [call(transaction)],
      atomicRequired: false,
    };
  }

  if (borrow?.kind === "BORROW") {
    const transaction = oneBorrowingTransaction(
      prepared,
      borrow.action.transactions,
      "BORROW",
      "borrow",
    );
    await simulatePreparedTransaction(client, owner, transaction);
    return {
      kind: "CALL_BATCH",
      nodeId,
      action: "BORROW",
      executionMode: "BORROW",
      steps: ["BORROW"],
      calls: [call(transaction)],
      atomicRequired: false,
      output: { port: "borrowed", asset: borrow.output },
    };
  }

  if (withdrawal?.kind === "WITHDRAW") {
    if (prepared.steps.length !== 1)
      throw new ExecutionMessage(
        "A withdrawal preparation must contain one recipe step.",
      );
    const [transaction] = withdrawal.action.transactions;
    if (
      withdrawal.action.transactions.length !== 1 ||
      transaction?.kind !== "WITHDRAW"
    )
      throw new ExecutionMessage(
        "The prepared withdrawal must contain one withdrawal transaction.",
      );
    return {
      kind: "CALL_BATCH",
      nodeId,
      action: "WITHDRAW",
      executionMode: "WITHDRAW",
      steps: ["WITHDRAW"],
      calls: [call(transaction)],
      atomicRequired: false,
    };
  }

  if (earn?.kind === "EARN_DEPOSIT") {
    if (prepared.steps.length !== 1)
      throw new ExecutionMessage(
        "An earn deposit preparation must contain one recipe step.",
      );
    const calls = earnDepositCalls(earn.action.transactions);
    return {
      kind: "CALL_BATCH",
      nodeId,
      action: "EARN_DEPOSIT",
      executionMode: "EARN_DEPOSIT",
      steps: ["EARN_DEPOSIT"],
      calls,
      atomicRequired: calls.length > 1,
    };
  }

  if (supply?.kind === "SUPPLY") {
    if (prepared.steps.length !== 1)
      throw new ExecutionMessage(
        "A supply preparation must contain one recipe step.",
      );
    const calls = supplyCalls(supply.action.transactions);
    return {
      kind: "CALL_BATCH",
      nodeId,
      action: "SUPPLY",
      executionMode: "SUPPLY",
      steps: ["SUPPLY"],
      calls,
      atomicRequired: calls.length > 1,
    };
  }

  const needsApproval = approval
    ? !(await hasSufficientAllowance(client, owner, approval.action))
    : false;

  if (trade?.kind === "TRADE" && trade.action.executionMode === "RFQ") {
    if (needsApproval && approval?.kind === "APPROVAL")
      return {
        kind: "CALL_BATCH",
        nodeId,
        action: "APPROVAL",
        executionMode: "SWAP",
        steps: ["APPROVAL"],
        calls: [call(approval.action)],
        atomicRequired: false,
      };
    return {
      kind: "RFQ",
      nodeId,
      action: "TRADE",
      executionMode: "RFQ",
      steps: ["TRADE"],
      order: {
        submissionId: trade.action.submissionId,
        routeQuoteId: trade.action.routeQuoteId,
        venue: trade.action.venue,
        typedData: trade.action.typedData,
      },
    };
  }

  if (trade?.kind === "TRADE" && trade.action.executionMode === "SWAP") {
    if (!needsApproval)
      await simulatePreparedTransaction(
        client,
        owner,
        trade.action.transaction,
      );
    const calls = [
      ...(needsApproval && approval?.kind === "APPROVAL"
        ? [call(approval.action)]
        : []),
      call(trade.action.transaction),
    ];
    return {
      kind: "CALL_BATCH",
      nodeId,
      action: "TRADE",
      executionMode: "SWAP",
      steps: needsApproval ? ["APPROVAL", "TRADE"] : ["TRADE"],
      calls,
      atomicRequired: calls.length > 1,
    };
  }

  if (approval?.kind === "APPROVAL") {
    if (!needsApproval)
      throw new ExecutionMessage("The allowance is already sufficient.");
    return {
      kind: "CALL_BATCH",
      nodeId,
      action: "APPROVAL",
      executionMode: "SWAP",
      steps: ["APPROVAL"],
      calls: [call(approval.action)],
      atomicRequired: false,
    };
  }
  throw new ExecutionMessage("Unsupported prepared action.");
}

function earnDepositCalls(
  transactions: Extract<
    PreparedRecipe["steps"][number],
    { kind: "EARN_DEPOSIT" }
  >["action"]["transactions"],
) {
  const kinds = transactions.map((transaction) => transaction.kind);
  const validDeposit = kinds.length === 1 && kinds[0] === "DEPOSIT";
  const validApprovalAndDeposit =
    kinds.length === 2 && kinds[0] === "APPROVE" && kinds[1] === "DEPOSIT";
  if (!validDeposit && !validApprovalAndDeposit)
    throw new ExecutionMessage("The prepared earn deposit is incomplete.");
  return transactions.map(call);
}

function supplyCalls(
  transactions: Extract<
    PreparedRecipe["steps"][number],
    { kind: "SUPPLY" }
  >["action"]["transactions"],
) {
  const kinds = transactions.map((transaction) => transaction.kind);
  const validDeposit = kinds.length === 1 && kinds[0] === "DEPOSIT";
  const validApprovalAndDeposit =
    kinds.length === 2 && kinds[0] === "APPROVE" && kinds[1] === "DEPOSIT";
  if (!validDeposit && !validApprovalAndDeposit)
    throw new ExecutionMessage("The prepared supply action is incomplete.");
  return transactions.map(call);
}

function repaymentCalls(
  transactions: Extract<
    PreparedRecipe["steps"][number],
    { kind: "REPAY" }
  >["action"]["transactions"],
) {
  const kinds = transactions.map((transaction) => transaction.kind);
  const validRepayment = kinds.length === 1 && kinds[0] === "REPAY";
  const validApprovalAndRepayment =
    kinds.length === 2 && kinds[0] === "APPROVE" && kinds[1] === "REPAY";
  if (!validRepayment && !validApprovalAndRepayment)
    throw new ExecutionMessage("The prepared repayment action is incomplete.");
  return transactions.map(call);
}

function oneBorrowingTransaction(
  prepared: PreparedRecipe,
  transactions: Extract<
    PreparedRecipe["steps"][number],
    { kind: "ENABLE_COLLATERAL" | "BORROW" }
  >["action"]["transactions"],
  expectedKind: "ENABLE_COLLATERAL" | "BORROW",
  label: string,
) {
  const [transaction] = transactions;
  if (
    prepared.steps.length !== 1 ||
    transactions.length !== 1 ||
    transaction?.kind !== expectedKind
  )
    throw new ExecutionMessage(
      `The prepared ${label} must contain one ${label} transaction.`,
    );
  return transaction;
}

export async function supportsAtomicCalls(
  walletClient: Pick<WalletClient, "getCapabilities">,
  account: Address,
  chainId: number,
): Promise<boolean> {
  try {
    const capabilities = await walletClient.getCapabilities({
      account,
      chainId,
    });
    return ["supported", "ready"].includes(capabilities.atomic?.status ?? "");
  } catch {
    return false;
  }
}

function call(action: { to: string; data: string; value?: string }): {
  to: Address;
  data: Hex;
  value?: bigint;
} {
  if (
    !/^0x[\da-f]{40}$/i.test(action.to) ||
    !/^0x[\da-f]{8,}$/i.test(action.data)
  )
    throw new ExecutionMessage("The prepared transaction is malformed.");
  return {
    to: action.to as Address,
    data: action.data as Hex,
    ...(action.value === undefined ? {} : { value: BigInt(action.value) }),
  };
}
