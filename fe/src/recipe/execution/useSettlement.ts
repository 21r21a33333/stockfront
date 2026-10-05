import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { usePublicClient } from "wagmi";
import { recipesApi } from "@/api/recipes";
import { StockfrontApiError } from "@/api/client";
import {
  isUnresolved,
  latestAttempts,
  runKey,
  type ExecutionAttempt,
  type ExecutionRun,
} from "./journal";

const CHECK_WINDOW_MS = 120_000;

export function useSettlement({
  run: current,
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
  const pending = latestAttempts(current?.attempts ?? []).find(isUnresolved);
  const key = current ? runKey(current.wallet, current.chainId) : undefined;
  const retryUntil = useRef(0);
  const [visible, setVisible] = useState(
    document.visibilityState !== "hidden" && navigator.onLine,
  );
  const [deadline, setDeadline] = useState(Date.now() + CHECK_WINDOW_MS);
  const [paused, setPaused] = useState(false);

  useEffect(() => {
    const resume = () => {
      const active = document.visibilityState !== "hidden" && navigator.onLine;
      setVisible(active);
      if (active) {
        setDeadline(Date.now() + CHECK_WINDOW_MS);
        setPaused(false);
      }
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
    if (!visible) return;
    const timer = window.setTimeout(
      () => setPaused(true),
      Math.max(0, deadline - Date.now()),
    );
    return () => window.clearTimeout(timer);
  }, [deadline, visible]);

  useEffect(() => {
    retryUntil.current = 0;
    setDeadline(Date.now() + CHECK_WINDOW_MS);
    setPaused(false);
  }, [current?.id, pending?.attempt, pending?.nodeId, pending?.kind]);

  const status = useQuery({
    queryKey: [
      "recipe-settlement",
      key,
      current?.id,
      pending?.nodeId,
      pending?.kind,
      pending?.attempt,
    ],
    enabled: Boolean(
      current &&
      pending &&
      !pending.needsReconciliation &&
      (pending.transactionHash || pending.orderId) &&
      visible &&
      !paused &&
      !busy &&
      publicClient,
    ),
    retry: false,
    refetchOnWindowFocus: false,
    refetchInterval: () =>
      paused || !visible
        ? false
        : Math.max(3000, retryUntil.current - Date.now()),
    queryFn: async ({ signal }) => {
      if (
        !current ||
        !pending ||
        !publicClient ||
        pending.needsReconciliation ||
        !navigator.onLine ||
        document.visibilityState === "hidden" ||
        Date.now() < retryUntil.current
      )
        return null;
      try {
        let patch: Partial<ExecutionAttempt> = {};
        if (pending.executionMode === "RFQ" && pending.orderId) {
          const result = await recipesApi.orderStatus(pending.orderId, signal);
          if (result.state === "filled")
            patch = {
              phase: "complete",
              transactionHash: result.transactionHash,
              amountsUnavailable:
                !result.filledBuyAmount || !result.filledSellAmount,
              error: undefined,
            };
          else if (["failed", "expired", "cancelled"].includes(result.state))
            patch = {
              phase: "failed",
              error: `Order ${result.state}. Refresh the route before retrying.`,
            };
        } else if (pending.transactionHash) {
          const hash = pending.replacementHash ?? pending.transactionHash;
          if (!pending.receiptConfirmed) {
            const receipt = await publicClient.getTransactionReceipt({ hash });
            if (receipt.status === "reverted")
              patch = {
                phase: "failed",
                error:
                  "Transaction reverted. Refresh the route before retrying.",
              };
            else patch = { receiptConfirmed: true };
          }
          if (patch.phase !== "failed") {
            if (pending.kind === "APPROVAL")
              patch = { ...patch, phase: "complete", error: undefined };
            else {
              const result = await recipesApi.transactionStatus(hash, signal);
              if (result.state === "succeeded")
                patch = {
                  ...patch,
                  phase: "complete",
                  amountsUnavailable:
                    !result.sold?.length || !result.bought?.length,
                  error: undefined,
                };
              else if (result.state === "failed")
                patch = {
                  ...patch,
                  phase: "failed",
                  error: "Trade failed. Refresh the route before retrying.",
                };
            }
          }
        }

        if (!signal.aborted && Object.keys(patch).length)
          await onStatus(current, pending, patch);
        return null;
      } catch (cause) {
        if (
          cause instanceof StockfrontApiError &&
          cause.retryAfterMs !== undefined
        ) {
          retryUntil.current = Date.now() + cause.retryAfterMs;
        }
        throw cause;
      }
    },
  });

  function resetWindow(): void {
    setPaused(false);
    setDeadline(Date.now() + CHECK_WINDOW_MS);
  }
  async function checkStatus(): Promise<void> {
    if (
      !navigator.onLine ||
      document.visibilityState === "hidden" ||
      Date.now() < retryUntil.current ||
      busy
    )
      return;
    resetWindow();
    await status.refetch();
  }
  return {
    paused,
    checkStatus,
    resetWindow,
    error: status.isError
      ? "Status is temporarily unavailable. The action has not been resubmitted."
      : undefined,
  };
}
