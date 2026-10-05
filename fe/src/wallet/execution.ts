import { useCallback, useState } from "react";
import type { Hash, SignTypedDataParameters } from "viem";
import {
  useAccount,
  useConnect,
  useDisconnect,
  usePublicClient,
  useSwitchChain,
  useWalletClient,
} from "wagmi";
import { stockfrontChain } from "@/wallet/config";
import type {
  PreparedRecipe,
  PreparedRecipeStep,
  SubmitOrderRequest,
} from "@/api/generated";
import { recipesApi } from "@/api/recipes";

export type ExecutionState = "queued" | "awaiting" | "pending" | "complete";

export interface ExecutionProgress {
  index: number;
  state: ExecutionState;
  transactionHash?: Hash;
}

interface ExecutionRun {
  prepared: PreparedRecipe;
  progress: ExecutionProgress[];
  error?: string;
}

export function useWalletSession() {
  const account = useAccount();
  const connect = useConnect();
  const disconnect = useDisconnect();
  const switchChain = useSwitchChain();
  const connector = connect.connectors[0];

  return {
    ...account,
    connect: () => connector && connect.connect({ connector }),
    disconnect: () => disconnect.disconnect(),
    switchToBnb: () => switchChain.switchChain({ chainId: stockfrontChain.id }),
    connecting: connect.isPending,
  };
}

export function usePreparedExecution(prepared?: PreparedRecipe) {
  const { data: walletClient } = useWalletClient();
  const publicClient = usePublicClient();
  const [run, setRun] = useState<ExecutionRun>();
  const currentRun = run?.prepared === prepared ? run : undefined;
  const completed =
    prepared !== undefined &&
    currentRun !== undefined &&
    prepared.steps.length > 0 &&
    currentRun.progress.length === prepared.steps.length &&
    currentRun.progress.every((step) => step.state === "complete");

  const execute = useCallback(async () => {
    if (!prepared || completed) return;
    if (!walletClient || !publicClient) {
      throw new Error("Connect a BNB Chain wallet before executing.");
    }
    const activeWalletClient = walletClient;
    const activePublicClient = publicClient;
    setRun({
      prepared,
      progress: prepared.steps.map((_, index) => ({ index, state: "queued" })),
    });
    try {
      for (const [index, step] of prepared.steps.entries()) {
        setStep(index, { state: "awaiting" });
        if (step.kind === "APPROVAL") {
          const hash = await activeWalletClient.sendTransaction({
            account: activeWalletClient.account,
            chain: stockfrontChain,
            to: step.action.to,
            data: step.action.data,
            gas: optionalBigInt(step.action.gasLimit),
            gasPrice: optionalBigInt(step.action.gasPrice),
          });
          setStep(index, { state: "pending", transactionHash: hash });
          await confirmTransaction(hash);
        } else {
          await executeTrade(step, index);
        }
        setStep(index, { state: "complete" });
      }
    } catch (cause) {
      const message =
        cause instanceof Error ? cause.message : "Wallet execution failed.";
      setRun((current) =>
        current?.prepared === prepared
          ? { ...current, error: message }
          : current,
      );
      throw cause;
    }

    function setStep(index: number, patch: Partial<ExecutionProgress>): void {
      setRun((current) =>
        current && current.prepared === prepared
          ? {
              ...current,
              progress: current.progress.map((item) =>
                item.index === index ? { ...item, ...patch } : item,
              ),
            }
          : current,
      );
    }

    async function confirmTransaction(hash: Hash): Promise<void> {
      const receipt = await activePublicClient.waitForTransactionReceipt({
        hash,
      });
      if (receipt.status !== "success") {
        throw new Error("The wallet transaction failed on BNB Chain.");
      }
    }

    async function executeTrade(
      step: Extract<PreparedRecipeStep, { kind: "TRADE" }>,
      index: number,
    ): Promise<void> {
      if (step.action.executionMode === "SWAP") {
        const transaction = step.action.transaction;
        const hash = await activeWalletClient.sendTransaction({
          account: activeWalletClient.account,
          chain: stockfrontChain,
          to: transaction.to,
          data: transaction.data,
          value: BigInt(transaction.value),
          gas: BigInt(transaction.gasLimit),
          gasPrice: BigInt(transaction.gasPrice),
        });
        setStep(index, { state: "pending", transactionHash: hash });
        await confirmTransaction(hash);
        return;
      }
      const typedData = JSON.parse(
        step.action.typedData,
      ) as SignTypedDataParameters;
      const signature = await activeWalletClient.signTypedData({
        ...typedData,
        account: activeWalletClient.account,
      });
      const order: SubmitOrderRequest = {
        submissionId: step.action.submissionId,
        routeQuoteId: step.action.routeQuoteId,
        venue: step.action.venue,
        signature,
      };
      await recipesApi.submitOrder(order);
    }
  }, [completed, prepared, publicClient, walletClient]);

  return {
    execute,
    progress: currentRun?.progress ?? [],
    error: currentRun?.error,
    completed,
  };
}

function optionalBigInt(value?: string): bigint | undefined {
  return value === undefined ? undefined : BigInt(value);
}
