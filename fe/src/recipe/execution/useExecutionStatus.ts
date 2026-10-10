import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { usePublicClient, useWalletClient } from "wagmi";
import type { Address, Hash } from "viem";
import { recipesApi } from "@/api/recipes";
import { StockfrontApiError } from "@/api/client";
import { exactBuyAmountFromTransferLogs, type TransferLog } from "./settlement";
import {
  isUnresolved,
  latestAttempts,
  type ExecutionAttempt,
  type ExecutionRun,
} from "./model";

const CHECK_WINDOW_MS = 120_000;
// BNB Chain produces a block roughly every 0.75s, so a slower poll reports a
// settled action long after it landed.
const CHECK_INTERVAL_MS = 1_500;
/// Three unreadable reads in a row is a status we cannot reach, not a slow one.
const UNREADABLE_LIMIT = 3;

export function useExecutionStatus({
  run,
  busy,
  onStatus,
}: {
  run?: ExecutionRun;
  busy: boolean;
  onStatus(
    run: ExecutionRun,
    attempt: ExecutionAttempt,
    patch: Partial<ExecutionAttempt>,
  ): Promise<void>;
}) {
  const publicClient = usePublicClient();
  const { data: wallet } = useWalletClient();
  const walletClient = wallet as CallsStatusClient | undefined;
  const pending = latestAttempts(run?.attempts ?? []).find(isUnresolved);
  const retryAfter = useRef(0);
  const unreadable = useRef(0);
  const [active, setActive] = useState(visibleAndOnline);
  const [deadline, setDeadline] = useState(Date.now() + CHECK_WINDOW_MS);
  const [paused, setPaused] = useState(false);

  useEffect(() => {
    const resume = () => {
      const next = visibleAndOnline();
      setActive(next);
      if (next) resetWindow();
    };
    document.addEventListener("visibilitychange", resume);
    window.addEventListener("online", resume);
    window.addEventListener("offline", resume);
    window.addEventListener("focus", resume);
    return () => {
      document.removeEventListener("visibilitychange", resume);
      window.removeEventListener("online", resume);
      window.removeEventListener("offline", resume);
      window.removeEventListener("focus", resume);
    };
  }, []);

  useEffect(() => {
    retryAfter.current = 0;
    unreadable.current = 0;
    resetWindow();
  }, [run?.id, pending?.id]);

  useEffect(() => {
    if (!active) return;
    const timer = window.setTimeout(
      () => setPaused(true),
      Math.max(0, deadline - Date.now()),
    );
    return () => window.clearTimeout(timer);
  }, [active, deadline]);

  const status = useQuery({
    queryKey: ["recipe-execution-status", run?.id, pending?.id],
    enabled: Boolean(
      run &&
      pending?.reference &&
      active &&
      !paused &&
      !busy &&
      publicClient &&
      (pending.reference.kind !== "CALL_BATCH" || walletClient),
    ),
    retry: false,
    refetchOnWindowFocus: false,
    refetchInterval: () =>
      active && !paused
        ? Math.max(CHECK_INTERVAL_MS, retryAfter.current - Date.now())
        : false,
    queryFn: async ({ signal }) => {
      if (
        !run ||
        !pending?.reference ||
        !publicClient ||
        !visibleAndOnline() ||
        Date.now() < retryAfter.current
      )
        return null;
      try {
        const patch = await readStatus({
          run,
          attempt: pending,
          publicClient,
          walletClient,
          signal,
        });
        unreadable.current = 0;
        if (!signal.aborted && patch) await onStatus(run, pending, patch);
      } catch (cause) {
        if (
          cause instanceof StockfrontApiError &&
          cause.retryAfterMs !== undefined
        )
          retryAfter.current = Date.now() + cause.retryAfterMs;
        unreadable.current += 1;
        // Spinning on a status that cannot be read strands the run, so hand the
        // user the recovery actions instead of waiting out the window.
        if (unreadable.current >= UNREADABLE_LIMIT && !signal.aborted)
          await onStatus(run, pending, {
            phase: "attention",
            error:
              "We could not read this action's status. Check your wallet activity before retrying.",
          });
        throw cause;
      }
      return null;
    },
  });

  function resetWindow(): void {
    setPaused(false);
    setDeadline(Date.now() + CHECK_WINDOW_MS);
  }
  async function checkStatus(): Promise<void> {
    if (!visibleAndOnline() || Date.now() < retryAfter.current || busy) return;
    resetWindow();
    await status.refetch();
  }
  return {
    paused,
    checkStatus,
    resetWindow,
    error: status.isError
      ? "Status is temporarily unavailable. The action was not resubmitted."
      : undefined,
  };
}

async function readStatus({
  run,
  attempt,
  publicClient,
  walletClient,
  signal,
}: {
  run: ExecutionRun;
  attempt: ExecutionAttempt;
  publicClient: NonNullable<ReturnType<typeof usePublicClient>>;
  walletClient?: CallsStatusClient;
  signal: AbortSignal;
}): Promise<Partial<ExecutionAttempt> | undefined> {
  const reference = attempt.reference;
  if (!reference) return undefined;
  const buyToken = run.recipe.nodes.find(
    (node) => node.id === attempt.nodeId && node.kind === "TRADE",
  );

  if (reference.kind === "CALL_BATCH" && walletClient) {
    // A batch id lives only as long as the wallet session, so after a reload the
    // only durable handle on the action is the hash it already produced.
    const result = await walletClient
      .getCallsStatus({ id: reference.id })
      .catch((cause: unknown) => {
        if (!reference.transactionHash) throw cause;
        return undefined;
      });
    if (!result)
      return await minedStatus(
        publicClient,
        attempt,
        buyToken,
        run.wallet,
        reference,
        reference.transactionHash as Hash,
      );
    if (result.status === "failure")
      return {
        phase: "failed",
        error:
          attempt.kind === "SUPPLY"
            ? "Atomic approval and supply reverted. Prepare the supply again before retrying."
            : attempt.kind === "EARN_DEPOSIT"
              ? "Atomic approval and earn deposit reverted. Prepare the deposit again before retrying."
              : attempt.kind === "WITHDRAW"
                ? "Withdrawal reverted. Prepare the withdrawal again before retrying."
                : attempt.kind === "ENABLE_COLLATERAL"
                  ? "Collateral activation reverted. Prepare it again before borrowing."
                  : attempt.kind === "BORROW"
                    ? "Borrow reverted. Refresh the risk preview before retrying."
                    : attempt.kind === "REPAY"
                      ? "Repayment reverted. Refresh the live debt before retrying."
                      : reference.atomicRequired
                        ? "Atomic approval and swap reverted. Refresh the route before retrying."
                        : "Transaction failed. Refresh the route before retrying.",
      };
    const pendingHash = result.receipts?.at(-1)?.transactionHash;
    if (result.status !== "success")
      return pendingHash && pendingHash !== reference.transactionHash
        ? { reference: { ...reference, transactionHash: pendingHash } }
        : undefined;
    if (reference.atomicRequired && !result.atomic)
      return {
        phase: "attention",
        error:
          "The wallet did not report atomic execution. Review wallet activity before retrying.",
      };
    const receipts = result.receipts ?? [];
    return completePatch(
      attempt,
      buyToken?.kind === "TRADE" ? buyToken.buyToken : undefined,
      receipts.flatMap((receipt) => receipt.logs ?? []),
      run.wallet,
      {
        ...reference,
        transactionHash: receipts.at(-1)?.transactionHash,
      },
    );
  }

  if (reference.kind === "RFQ_ORDER" && reference.orderId) {
    const result = await recipesApi.orderStatus(reference.orderId, signal);
    if (["failed", "expired", "cancelled"].includes(result.state))
      return {
        phase: "failed",
        error: `Order ${result.state}. Refresh the route before retrying.`,
      };
    if (result.state !== "filled") return undefined;
    const amount = positiveInteger(result.filledBuyAmount)
      ? result.filledBuyAmount
      : undefined;
    return {
      phase: "complete",
      reference: { ...reference, transactionHash: result.transactionHash },
      settledOutput:
        buyToken?.kind === "TRADE" && amount
          ? { port: "buy", asset: { token: buyToken.buyToken, amount } }
          : undefined,
      amountsUnavailable: !amount,
      error: undefined,
    };
  }

  if (reference.kind === "LEGACY_TRANSACTION")
    return await minedStatus(
      publicClient,
      attempt,
      buyToken,
      run.wallet,
      reference,
      reference.transactionHash,
    );
  return undefined;
}

/// Reads an action's outcome from the chain rather than from the wallet.
async function minedStatus(
  publicClient: NonNullable<ReturnType<typeof usePublicClient>>,
  attempt: ExecutionAttempt,
  buyToken: { kind: string; buyToken?: Address } | undefined,
  wallet: Address,
  reference: NonNullable<ExecutionAttempt["reference"]>,
  hash: Hash,
): Promise<Partial<ExecutionAttempt> | undefined> {
  const receipt = await publicClient.getTransactionReceipt({ hash });
  if (receipt.status === "reverted")
    return {
      phase: "failed",
      error: "Transaction reverted. Refresh the route before retrying.",
    };
  return completePatch(
    attempt,
    buyToken?.kind === "TRADE" ? buyToken.buyToken : undefined,
    receipt.logs,
    wallet,
    reference,
  );
}

function completePatch(
  attempt: ExecutionAttempt,
  buyToken: Address | undefined,
  logs: readonly TransferLog[],
  wallet: Address,
  reference: ExecutionAttempt["reference"],
): Partial<ExecutionAttempt> {
  const amount =
    buyToken && attempt.kind === "TRADE"
      ? exactBuyAmountFromTransferLogs(logs, buyToken, wallet)
      : undefined;
  const settledOutput =
    attempt.kind === "BORROW"
      ? attempt.expectedOutput
      : buyToken && amount
        ? {
            port: "buy" as const,
            asset: { token: buyToken, amount: amount.toString() },
          }
        : undefined;
  return {
    phase: "complete",
    reference,
    settledOutput,
    amountsUnavailable: attempt.kind === "TRADE" && amount === undefined,
    error: undefined,
  };
}

type CallsStatusClient = {
  getCallsStatus(args: { id: string }): Promise<{
    status: string;
    atomic?: boolean;
    receipts?: readonly {
      transactionHash?: Hash;
      logs?: readonly TransferLog[];
    }[];
  }>;
};

function visibleAndOnline(): boolean {
  return document.visibilityState !== "hidden" && navigator.onLine;
}

function positiveInteger(value: string | undefined): value is string {
  return value !== undefined && /^\d+$/.test(value) && BigInt(value) > 0n;
}
