import { useAccount, useConnect, useDisconnect, useSwitchChain } from "wagmi";
import { stockfrontChain } from "./config";

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
