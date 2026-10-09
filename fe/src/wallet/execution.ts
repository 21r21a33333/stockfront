import { useAccount, useDisconnect, useSwitchChain } from "wagmi";
import { stockfrontChain } from "./config";

export function useWalletSession() {
  const account = useAccount();
  const disconnect = useDisconnect();
  const switchChain = useSwitchChain();
  return {
    ...account,
    disconnect: () => disconnect.disconnect(),
    switchToBnb: () => switchChain.switchChain({ chainId: stockfrontChain.id }),
  };
}
