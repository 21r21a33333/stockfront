import { useEffect, useRef, useState } from "react";
import { useAccount, usePublicClient, useWalletClient } from "wagmi";
import type { SignTypedDataParameters } from "viem";
import type {
  PreparedRecipe,
  RecipeDefinition,
  SubmitOrderRequest,
} from "@/api/generated";
import { recipesApi } from "@/api/recipes";
import { useSettlement } from "./useSettlement";
import { stockfrontChain } from "@/wallet/config";
import {
  executionProgress,
  isRunComplete,
  isUnresolved,
  latestAttempts,
  readRun,
  runKey,
  writeRun,
  type ExecutionAttempt,
  type ExecutionRun,
} from "./journal";

const RFQ_RETRY_WINDOW_MS = 30 * 60_000;

export function useRecipeExecution({
  recipe,
  prepared,
}: {
  recipe: RecipeDefinition;
  prepared?: PreparedRecipe;
}) {
  const account = useAccount();
  const { data: walletClient } = useWalletClient();
  const publicClient = usePublicClient();
  const key = account.address
    ? runKey(account.address, account.chainId ?? 0)
    : undefined;
  const context = useRef({
    key,
    address: account.address,
    chainId: account.chainId,
  });
  context.current = { key, address: account.address, chainId: account.chainId };
  const memory = useRef(new Map<string, ExecutionRun>());
  const storageFaults = useRef(new Set<string>());
  const signedOrders = useRef(new Map<string, SubmitOrderRequest>());
  const usedPreparations = useRef(new WeakSet<PreparedRecipe>());
  const currentPreparation = useRef(prepared);
  currentPreparation.current = prepared;
  const running = useRef(false);
  const mounted = useRef(true);
  const [run, setRun] = useState<ExecutionRun>();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  function publish(value: ExecutionRun): void {
    const target = runKey(value.wallet, value.chainId);
    memory.current.set(target, value);
    if (context.current.key === target) setRun(value);
  }
  function save(value: ExecutionRun): void {
    try {
      writeRun(value);
      storageFaults.current.delete(runKey(value.wallet, value.chainId));
    } catch {
      storageFaults.current.add(runKey(value.wallet, value.chainId));
      publish(value);
      throw new Error(
        "Could not save recovery data. Copy the transaction/order reference and do not start another spend.",
      );
    }
    publish(value);
  }
  function update(
    value: ExecutionRun,
    attempt: ExecutionAttempt,
    patch: Partial<ExecutionAttempt>,
    persist = true,
  ): ExecutionRun {
    const next = {
      ...value,
      updatedAtMs: Date.now(),
      attempts: value.attempts.map((item) =>
        item.nodeId === attempt.nodeId &&
        item.kind === attempt.kind &&
        item.attempt === attempt.attempt
          ? { ...item, ...patch }
          : item,
      ),
    };
    if (persist) save(next);
    else publish(next);
    return next;
  }

  useEffect(() => {
    setError(undefined);
    const load = () => {
      if (!account.address || !key) {
        setRun(undefined);
        return;
      }
      try {
        const saved = storageFaults.current.has(key)
          ? memory.current.get(key)
          : readRun(account.address, account.chainId ?? 0);
        if (saved) memory.current.set(key, saved);
        setRun(saved);
      } catch (cause) {
        setError(
          cause instanceof Error
            ? cause.message
            : "Recovery data could not be read.",
        );
      }
    };
    load();
    const onStorage = (event: StorageEvent) => {
      if (event.key === key) {
        if (currentPreparation.current)
          usedPreparations.current.add(currentPreparation.current);
        load();
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, [key, account.address, account.chainId]);

  const current =
    run && key === runKey(run.wallet, run.chainId) ? run : undefined;
  const progress = executionProgress(current?.attempts ?? [], busy);
  const pending = progress.find(isUnresolved);
  const completed = current !== undefined && isRunComplete(current);
  const blocked = pending !== undefined;
  const sameRecipe =
    !current || JSON.stringify(current.recipe) === JSON.stringify(recipe);

  const settlement = useSettlement({
    run: current,
    busy,
    onStatus: async (current, pending, patch) => {
      if (!navigator.locks) {
        // Read-only recovery still works; shared storage requires a lock.
        update(current, pending, patch, false);
        return;
      }
      await navigator.locks.request(
        runKey(current.wallet, current.chainId),
        { ifAvailable: true },
        (lock) => {
          if (!lock) return;
          const target = runKey(current.wallet, current.chainId);
          const saved = storageFaults.current.has(target)
            ? memory.current.get(target)
            : readRun(current.wallet, current.chainId);
          if (!saved || saved.id !== current.id) return;
          const latest = latestAttempts(saved.attempts).find(
            (item) =>
              item.nodeId === pending.nodeId && item.kind === pending.kind,
          );
          if (
            latest?.attempt === pending.attempt &&
            isUnresolved(latest) &&
            !latest.needsReconciliation
          )
            update(saved, latest, patch);
        },
      );
    },
  });

  async function execute(): Promise<void> {
    if (
      running.current ||
      !prepared ||
      !walletClient ||
      !publicClient ||
      !account.address ||
      account.chainId !== stockfrontChain.id
    )
      return;
    if (!navigator.locks) {
      setError(
        "This browser does not support safe execution locks. Use a current browser on HTTPS or localhost.",
      );
      return;
    }
    const executionKey = runKey(account.address, account.chainId);
    const assertContext = () => {
      if (
        !mounted.current ||
        context.current.key !== executionKey ||
        walletClient.account.address.toLowerCase() !==
          account.address?.toLowerCase()
      )
        throw new Error(
          "Wallet changed. Return to the original wallet to continue.",
        );
    };
    running.current = true;
    setBusy(true);
    setError(undefined);
    try {
      await navigator.locks.request(
        executionKey,
        { ifAvailable: true },
        async (lock) => {
          if (!lock) {
            setError("This wallet is executing in another tab.");
            return;
          }
          assertContext();
          if (storageFaults.current.has(executionKey))
            throw new Error(
              "Recovery storage is unavailable. Check the saved reference before continuing.",
            );
          let active = readRun(account.address!, account.chainId!);
          if (active) {
            publish(active);
            if (isRunComplete(active)) return;
            if (active.attempts.some(isUnresolved)) {
              setError(
                "Previous action is unresolved. Check status; do not submit it again.",
              );
              return;
            }
            if (JSON.stringify(active.recipe) !== JSON.stringify(recipe))
              throw new Error(
                "The draft differs from the execution snapshot. Finish the existing run first.",
              );
          }
          if (usedPreparations.current.has(prepared))
            throw new Error(
              "Refresh the route and prepare again before continuing.",
            );
          if (!prepared.steps.length) return;
          active ??= {
            version: 1,
            id: crypto.randomUUID(),
            wallet: account.address!,
            chainId: account.chainId!,
            recipe: structuredClone(recipe),
            createdAtMs: Date.now(),
            updatedAtMs: Date.now(),
            attempts: [],
          };
          save(active);
          usedPreparations.current.add(prepared);
          for (const step of prepared.steps) {
            if (
              active.attempts.some(
                (item) =>
                  item.nodeId === step.nodeId &&
                  item.kind === "TRADE" &&
                  item.phase === "complete",
              )
            )
              continue;
            assertContext();
            const attempt: ExecutionAttempt = {
              nodeId: step.nodeId,
              kind: step.kind,
              attempt:
                1 +
                Math.max(
                  0,
                  ...active.attempts
                    .filter(
                      (item) =>
                        item.nodeId === step.nodeId && item.kind === step.kind,
                    )
                    .map((item) => item.attempt),
                ),
              phase: "awaiting",
              executionMode:
                step.kind === "TRADE" ? step.action.executionMode : undefined,
            };
            active = { ...active, attempts: [...active.attempts, attempt] };
            save(active);
            try {
              if (
                step.kind === "TRADE" &&
                step.action.executionMode === "RFQ"
              ) {
                const typedData = JSON.parse(
                  step.action.typedData,
                ) as SignTypedDataParameters;
                const signature = await walletClient.signTypedData({
                  ...typedData,
                  account: walletClient.account,
                });
                assertContext();
                const order: SubmitOrderRequest = {
                  submissionId: step.action.submissionId,
                  routeQuoteId: step.action.routeQuoteId,
                  venue: step.action.venue,
                  signature,
                };
                signedOrders.current.set(active.id, order);
                active = update(active, attempt, {
                  phase: "submitting",
                  submissionId: order.submissionId,
                  firstSubmittedAtMs: Date.now(),
                });
                const submitted = await recipesApi.submitOrder(order);
                active = update(active, attempt, {
                  phase: "pending",
                  orderId: submitted.orderId,
                });
                signedOrders.current.delete(active.id);
              } else {
                const tx =
                  step.kind === "APPROVAL"
                    ? step.action
                    : step.action.executionMode === "SWAP"
                      ? step.action.transaction
                      : undefined;
                if (!tx) throw new Error("Unsupported prepared action.");
                const hash = await walletClient.sendTransaction({
                  account: walletClient.account,
                  chain: stockfrontChain,
                  to: tx.to,
                  data: tx.data,
                  value: "value" in tx ? BigInt(tx.value) : undefined,
                  gas: tx.gasLimit ? BigInt(tx.gasLimit) : undefined,
                  gasPrice: tx.gasPrice ? BigInt(tx.gasPrice) : undefined,
                });
                active = update(active, attempt, {
                  phase: "pending",
                  transactionHash: hash,
                });
                let replaced = false;
                const receipt = await publicClient.waitForTransactionReceipt({
                  hash,
                  timeout: 30_000,
                  onReplaced: ({ reason, transaction }) => {
                    replaced = reason !== "repriced";
                    active = update(active!, attempt, {
                      replacementHash: transaction.hash,
                      ...(replaced
                        ? {
                            phase:
                              reason === "cancelled" ? "failed" : "attention",
                            needsReconciliation: reason !== "cancelled",
                            error:
                              reason === "cancelled"
                                ? "Transaction cancelled in wallet."
                                : "Transaction replaced with a different action. Reconcile before retrying.",
                          }
                        : {}),
                    });
                  },
                });
                if (replaced) return;
                active = update(
                  active,
                  attempt,
                  receipt.status === "success"
                    ? {
                        phase:
                          step.kind === "APPROVAL" ? "complete" : "pending",
                        receiptConfirmed: true,
                      }
                    : {
                        phase: "failed",
                        error:
                          "Transaction reverted. Refresh the route before retrying.",
                      },
                );
                if (receipt.status !== "success") return;
              }
            } catch (cause) {
              const saved = memory.current.get(executionKey) ?? active;
              const latest = saved.attempts.at(-1)!;
              if (!storageFaults.current.has(executionKey)) {
                const rejected =
                  userRejected(cause) && latest.phase === "awaiting";
                active = update(saved, latest, {
                  phase: rejected ? "failed" : "attention",
                  error: rejected
                    ? "Wallet request rejected. Refresh the route to retry."
                    : "Outcome not confirmed. Check the reference before retrying.",
                });
              }
              throw cause;
            }
            if (step.kind === "TRADE") return;
          }
        },
      );
    } catch (cause) {
      // Do not surface wallet/provider exception payloads: they can contain signed requests.
      setError(
        storageFaults.current.has(executionKey)
          ? "Could not save recovery data. Copy the visible reference before leaving this page."
          : cause instanceof Error &&
              cause.message.startsWith("Refresh the route")
            ? cause.message
            : "Execution paused. Review the action status before continuing.",
      );
    } finally {
      running.current = false;
      setBusy(false);
      settlement.resetWindow();
    }
  }

  async function retrySubmission(): Promise<void> {
    if (!current || running.current || !navigator.locks) return;
    running.current = true;
    setBusy(true);
    try {
      await navigator.locks.request(
        runKey(current.wallet, current.chainId),
        { ifAvailable: true },
        async (lock) => {
          if (
            !lock ||
            context.current.key !== runKey(current.wallet, current.chainId)
          )
            return;
          const saved = readRun(current.wallet, current.chainId);
          const attempt = saved?.attempts.at(-1);
          const order = signedOrders.current.get(current.id);
          if (
            !saved ||
            saved.id !== current.id ||
            !attempt ||
            !order ||
            attempt.orderId ||
            !attempt.firstSubmittedAtMs ||
            attempt.submissionId !== order.submissionId ||
            Date.now() - attempt.firstSubmittedAtMs >= RFQ_RETRY_WINDOW_MS ||
            storageFaults.current.has(runKey(current.wallet, current.chainId))
          )
            return;
          save(saved);
          const submitted = await recipesApi.submitOrder(order);
          update(saved, attempt, {
            phase: "pending",
            orderId: submitted.orderId,
            error: undefined,
          });
          signedOrders.current.delete(current.id);
        },
      );
    } catch {
      setError(
        "Submission outcome is still unknown. Keep this page open and do not create another order.",
      );
    } finally {
      running.current = false;
      setBusy(false);
      settlement.resetWindow();
    }
  }

  async function startNewRun(): Promise<void> {
    if (!current || !navigator.locks || running.current) return;
    await navigator.locks
      .request(
        runKey(current.wallet, current.chainId),
        { ifAvailable: true },
        (lock) => {
          if (!lock) return;
          const saved = readRun(current.wallet, current.chainId);
          if (!saved || !isRunComplete(saved)) return;
          if (prepared) usedPreparations.current.add(prepared);
          localStorage.removeItem(runKey(current.wallet, current.chainId));
          memory.current.delete(runKey(current.wallet, current.chainId));
          setRun(undefined);
          setError(undefined);
        },
      )
      .catch(() =>
        setError(
          "Could not clear the completed run. Recovery data was retained.",
        ),
      );
  }

  const last = current?.attempts.at(-1);
  return {
    run: current,
    progress,
    completed,
    blocked,
    busy,
    sameRecipe,
    paused: settlement.paused,
    error: error ?? settlement.error,
    canExecute: Boolean(
      navigator.locks &&
      prepared &&
      !usedPreparations.current.has(prepared) &&
      !blocked &&
      !completed &&
      sameRecipe &&
      !busy &&
      account.chainId === stockfrontChain.id,
    ),
    canRetrySubmission: Boolean(
      current &&
      last?.firstSubmittedAtMs &&
      !last.orderId &&
      signedOrders.current.has(current.id) &&
      Date.now() - last.firstSubmittedAtMs < RFQ_RETRY_WINDOW_MS,
    ),
    execute,
    checkStatus: settlement.checkStatus,
    retrySubmission,
    startNewRun,
  };
}

function userRejected(cause: unknown): boolean {
  for (
    let depth = 0;
    depth < 5 && cause && typeof cause === "object";
    depth++
  ) {
    if ("code" in cause && cause.code === 4001) return true;
    cause = "cause" in cause ? cause.cause : undefined;
  }
  return false;
}
