import type { PreparedRecipe, SubmitOrderRequest } from "@/api/generated";
import type { Address, Hex, WalletClient } from "viem";
import {
  hasSufficientAllowance,
  simulatePreparedTransaction,
} from "./preflight";

export type PreparedExecution =
  | {
      kind: "CALL_BATCH";
      nodeId: string;
      action: "APPROVAL" | "TRADE" | "SUPPLY";
      executionMode: "SWAP" | "SUPPLY";
      steps: ("APPROVAL" | "TRADE" | "SUPPLY")[];
      calls: { to: Address; data: Hex; value?: bigint }[];
      atomicRequired: boolean;
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
  if (nodes.size !== 1) throw new Error("Prepare one recipe action at a time.");
  const nodeId = prepared.steps[0]?.nodeId;
  if (!nodeId) throw new Error("No prepared recipe action is available.");

  const approval = prepared.steps.find((step) => step.kind === "APPROVAL");
  const trade = prepared.steps.find((step) => step.kind === "TRADE");
  const supply = prepared.steps.find((step) => step.kind === "SUPPLY");

  if (supply?.kind === "SUPPLY") {
    if (prepared.steps.length !== 1)
      throw new Error("A supply preparation must contain one recipe step.");
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
    if (!needsApproval) throw new Error("The allowance is already sufficient.");
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
  throw new Error("Unsupported prepared action.");
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
    throw new Error("The prepared supply action is incomplete.");
  return transactions.map(call);
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
    throw new Error("The prepared transaction is malformed.");
  return {
    to: action.to as Address,
    data: action.data as Hex,
    ...(action.value === undefined ? {} : { value: BigInt(action.value) }),
  };
}
