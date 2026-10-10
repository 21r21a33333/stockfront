import { useEffect, useRef, useState } from "react";
import { useAccount, usePublicClient, useWalletClient } from "wagmi";
import type { SignTypedDataParameters } from "viem";
import type {
  PreparedRecipe,
  RecipeDefinition,
  SubmitOrderRequest,
} from "@/api/generated";
import { recipesApi } from "@/api/recipes";
import { stockfrontChain } from "@/wallet/config";
import {
  createRun,
  executionProgress,
  isRunComplete,
  isUnresolved,
  latestAttempts,
  nextAttemptNumber,
  patchAttempt,
  sameRecipe,
  ExecutionMessage,
  type ExecutionAttempt,
  type ExecutionRun,
} from "./model";
import { readRun, runKey, writeRun } from "./journal";
import { preparedExecution, supportsAtomicCalls } from "./preparedExecution";
import { useExecutionStatus } from "./useExecutionStatus";

const RFQ_RETRY_WINDOW_MS = 30 * 60_000;

export function useRecipeExecution({
  recipe,
  prepared,
}: {
  recipe: RecipeDefinition;
  prepared?: PreparedRecipe;
}) {
  const account = useAccount();
  const publicClient = usePublicClient();
  const { data: walletClient } = useWalletClient();
  const key = account.address
    ? runKey(account.address, account.chainId ?? 0)
    : undefined;
  const context = useRef(key);
  context.current = key;
  const memory = useRef(new Map<string, ExecutionRun>());
  const storageFaults = useRef(new Set<string>());
  const signedOrders = useRef(new Map<string, SubmitOrderRequest>());
  const usedPreparations = useRef(new WeakSet<PreparedRecipe>());
  const currentPreparation = useRef(prepared);
  currentPreparation.current = prepared;
  const running = useRef(false);
  const [run, setRun] = useState<ExecutionRun>();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);

  function publish(value: ExecutionRun): void {
    const target = runKey(value.wallet, value.chainId);
    memory.current.set(target, value);
    if (context.current === target) setRun(value);
  }

  function save(value: ExecutionRun): void {
    const target = runKey(value.wallet, value.chainId);
    try {
      writeRun(value);
      storageFaults.current.delete(target);
    } catch {
      storageFaults.current.add(target);
      publish(value);
      throw new ExecutionMessage(
        "Could not save recovery data. Copy the visible reference before leaving this page.",
      );
    }
    publish(value);
  }

  useEffect(() => {
    setError(undefined);
    const load = () => {
      if (!account.address || !key) return setRun(undefined);
      try {
        const saved = storageFaults.current.has(key)
          ? memory.current.get(key)
          : readRun(account.address, account.chainId ?? 0);
        if (saved) memory.current.set(key, saved);
        setRun(saved);
      } catch (cause) {
        setError(message(cause, "Recovery data could not be read."));
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
  const matches = !current || sameRecipe(current.recipe, recipe);
  const saved = executionProgress(current?.attempts ?? [], busy);
  const progress = matches ? saved : [];
  // An unresolved action blocks every draft, not only the one that started it.
  const pending = saved.find(isUnresolved);
  const completed = matches && current !== undefined && isRunComplete(current);
  const unfinishedOtherRun =
    current !== undefined && !matches && !isRunComplete(current);

  const status = useExecutionStatus({
    run: current,
    busy,
    onStatus: async (snapshot, attempt, patch) => {
      await withRunLock(snapshot, (saved) => {
        const latest = latestAttempts(saved.attempts).find(
          (item) => item.id === attempt.id,
        );
        if (latest && isUnresolved(latest))
          save(patchAttempt(saved, latest, patch));
      });
    },
  });

  async function execute(nextPrepared?: PreparedRecipe): Promise<void> {
    const plan = nextPrepared ?? prepared;
    if (
      running.current ||
      !plan ||
      !walletClient ||
      !publicClient ||
      !account.address ||
      account.chainId !== stockfrontChain.id
    )
      return;
    if (!navigator.locks)
      return setError("This browser cannot safely lock recipe execution.");

    running.current = true;
    setBusy(true);
    setError(undefined);
    try {
      await navigator.locks.request(
        runKey(account.address, account.chainId),
        { ifAvailable: true },
        async (lock) => {
          if (!lock)
            throw new ExecutionMessage(
              "This wallet is executing in another tab.",
            );
          assertWalletContext();
          const execution = await preparedExecution(
            plan,
            publicClient,
            walletClient.account.address,
          );
          if (
            execution.kind === "CALL_BATCH" &&
            execution.atomicRequired &&
            !(await supportsAtomicCalls(
              walletClient,
              walletClient.account.address,
              stockfrontChain.id,
            ))
          )
            throw new ExecutionMessage(
              "Atomic batching is not supported by this wallet on BNB Chain. No transaction was sent.",
            );

          let active =
            readRun(account.address!, account.chainId!) ??
            createRun(structuredClone(recipe), account.address!);
          if (active.attempts.some(isUnresolved))
            throw new ExecutionMessage(
              "Previous action is unresolved. Check its status first.",
            );
          if (!sameRecipe(active.recipe, recipe))
            active = createRun(structuredClone(recipe), account.address!);
          if (isRunComplete(active)) return;
          if (usedPreparations.current.has(plan))
            throw new ExecutionMessage(
              "Refresh the route and prepare again before continuing.",
            );

          const attempt: ExecutionAttempt = {
            id: crypto.randomUUID(),
            nodeId: execution.nodeId,
            kind: execution.action,
            attempt: nextAttemptNumber(active.attempts, execution.nodeId),
            phase: "awaiting",
            executionMode: execution.executionMode,
            steps: execution.steps,
            expectedOutput:
              execution.kind === "CALL_BATCH" ? execution.output : undefined,
            approvalSkipped:
              execution.action === "SUPPLY"
                ? plan.steps.some(
                    (step) =>
                      step.kind === "SUPPLY" &&
                      !step.action.transactions.some(
                        (transaction) => transaction.kind === "APPROVE",
                      ),
                  )
                : execution.action === "REPAY"
                  ? plan.steps.some(
                      (step) =>
                        step.kind === "REPAY" &&
                        !step.action.transactions.some(
                          (transaction) => transaction.kind === "APPROVE",
                        ),
                    )
                  : plan.steps.some((step) => step.kind === "APPROVAL") &&
                    !execution.steps.some((step) => step === "APPROVAL"),
          };
          active = { ...active, attempts: [...active.attempts, attempt] };
          save(active);
          usedPreparations.current.add(plan);

          try {
            assertWalletContext();
            if (execution.kind === "CALL_BATCH") {
              const submitted = await walletClient.sendCalls({
                account: walletClient.account,
                chain: stockfrontChain,
                calls: execution.calls,
                forceAtomic: execution.atomicRequired,
              });
              active = patchAttempt(active, attempt, {
                phase: "pending",
                firstSubmittedAtMs: Date.now(),
                reference: {
                  kind: "CALL_BATCH",
                  id: submitted.id,
                  atomicRequired: execution.atomicRequired,
                },
              });
            } else {
              const typedData = JSON.parse(
                execution.order.typedData,
              ) as SignTypedDataParameters;
              const signature = await walletClient.signTypedData({
                ...typedData,
                account: walletClient.account,
              });
              const order: SubmitOrderRequest = {
                submissionId: execution.order.submissionId,
                routeQuoteId: execution.order.routeQuoteId,
                venue: execution.order.venue,
                signature,
              };
              signedOrders.current.set(attempt.id, order);
              active = patchAttempt(active, attempt, {
                phase: "submitting",
                firstSubmittedAtMs: Date.now(),
                reference: {
                  kind: "RFQ_ORDER",
                  submissionId: order.submissionId,
                },
              });
              save(active);
              const submitted = await recipesApi.submitOrder(order);
              active = patchAttempt(active, attempt, {
                phase: "pending",
                reference: {
                  kind: "RFQ_ORDER",
                  submissionId: order.submissionId,
                  orderId: submitted.orderId,
                },
              });
              signedOrders.current.delete(attempt.id);
            }
            save(active);
          } catch (cause) {
            const rejected = userRejected(cause);
            save(
              patchAttempt(active, attempt, {
                phase: rejected ? "failed" : "attention",
                error: rejected
                  ? "Wallet request rejected. Refresh the route to retry."
                  : "Submission outcome is unknown. Check wallet activity before retrying.",
              }),
            );
            throw cause;
          }
        },
      );
    } catch (cause) {
      setError(publicExecutionError(cause));
    } finally {
      running.current = false;
      setBusy(false);
      status.resetWindow();
    }
  }

  async function retrySubmission(): Promise<void> {
    if (!current || running.current) return;
    const attempt = current.attempts.at(-1);
    const order = attempt && signedOrders.current.get(attempt.id);
    const reference = attempt?.reference;
    if (
      !attempt ||
      !order ||
      reference?.kind !== "RFQ_ORDER" ||
      reference.orderId ||
      !attempt.firstSubmittedAtMs ||
      Date.now() - attempt.firstSubmittedAtMs >= RFQ_RETRY_WINDOW_MS
    )
      return;
    running.current = true;
    setBusy(true);
    try {
      await withRunLock(current, async (saved) => {
        const submitted = await recipesApi.submitOrder(order);
        save(
          patchAttempt(saved, attempt, {
            phase: "pending",
            error: undefined,
            reference: {
              ...reference,
              orderId: submitted.orderId,
            },
          }),
        );
        signedOrders.current.delete(attempt.id);
      });
    } catch {
      setError("Submission outcome remains unknown. No new order was created.");
    } finally {
      running.current = false;
      setBusy(false);
    }
  }

  async function startNewRun(): Promise<void> {
    if (!current || running.current || !navigator.locks) return;
    await navigator.locks.request(
      runKey(current.wallet, current.chainId),
      { ifAvailable: true },
      (lock) => {
        if (!lock) return;
        const saved = readRun(current.wallet, current.chainId);
        if (!saved || !isRunComplete(saved)) return;
        localStorage.removeItem(runKey(current.wallet, current.chainId));
        memory.current.delete(runKey(current.wallet, current.chainId));
        setRun(undefined);
        setError(undefined);
      },
    );
  }

  /**
   * Forgets the saved run even when an action is unresolved. The caller must
   * have the user confirm that nothing is still pending in their wallet, because
   * the journal is the only guard against submitting the same spend twice.
   */
  async function abandonRun(): Promise<void> {
    if (!current || running.current || !navigator.locks) return;
    await navigator.locks.request(
      runKey(current.wallet, current.chainId),
      { ifAvailable: true },
      (lock) => {
        if (!lock) return;
        localStorage.removeItem(runKey(current.wallet, current.chainId));
        memory.current.delete(runKey(current.wallet, current.chainId));
        signedOrders.current.clear();
        setRun(undefined);
        setError(undefined);
      },
    );
  }

  async function withRunLock(
    snapshot: ExecutionRun,
    action: (saved: ExecutionRun) => void | Promise<void>,
  ): Promise<void> {
    if (!navigator.locks) return;
    await navigator.locks.request(
      runKey(snapshot.wallet, snapshot.chainId),
      { ifAvailable: true },
      async (lock) => {
        if (
          !lock ||
          context.current !== runKey(snapshot.wallet, snapshot.chainId)
        )
          return;
        const saved = readRun(snapshot.wallet, snapshot.chainId);
        if (saved?.id === snapshot.id) await action(saved);
      },
    );
  }

  function assertWalletContext(): void {
    if (
      context.current !== key ||
      walletClient?.account.address.toLowerCase() !==
        account.address?.toLowerCase()
    )
      throw new ExecutionMessage(
        "Wallet changed. Return to the original wallet to continue.",
      );
  }

  const latest = latestAttempts(matches ? (current?.attempts ?? []) : []);
  const last = latest.at(-1);
  const requiresFreshPreparation = latest.some(
    (item) =>
      (item.kind === "APPROVAL" || item.kind === "ENABLE_COLLATERAL") &&
      item.phase === "complete",
  );
  const canPrepareFresh = Boolean(
    navigator.locks &&
    !pending &&
    !completed &&
    !unfinishedOtherRun &&
    !busy &&
    account.chainId === stockfrontChain.id,
  );
  return {
    run: current,
    progress,
    completed,
    blocked: pending !== undefined,
    busy,
    sameRecipe: matches,
    unfinishedOtherRun,
    requiresFreshPreparation,
    canPrepareFresh,
    paused: status.paused,
    error: error ?? status.error,
    canExecute: Boolean(
      navigator.locks &&
      prepared &&
      !usedPreparations.current.has(prepared) &&
      !pending &&
      !completed &&
      !unfinishedOtherRun &&
      !busy &&
      account.chainId === stockfrontChain.id,
    ),
    canRetrySubmission: Boolean(
      last?.reference?.kind === "RFQ_ORDER" &&
      !last.reference.orderId &&
      last.firstSubmittedAtMs &&
      signedOrders.current.has(last.id) &&
      Date.now() - last.firstSubmittedAtMs < RFQ_RETRY_WINDOW_MS,
    ),
    execute,
    checkStatus: status.checkStatus,
    retrySubmission,
    startNewRun,
    abandonRun,
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

function message(cause: unknown, fallback: string): string {
  return cause instanceof Error ? cause.message : fallback;
}

function publicExecutionError(cause: unknown): string {
  return cause instanceof ExecutionMessage
    ? cause.message
    : "Execution paused. Review the action status before continuing.";
}
