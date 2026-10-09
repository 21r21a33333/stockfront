import { createConfig, http } from "wagmi";
import { bsc } from "wagmi/chains";
import { injected } from "wagmi/connectors";

const rpcUrl = import.meta.env.VITE_BNB_RPC_URL || undefined;

export const walletConfig = createConfig({
  chains: [bsc],
  connectors: [injected()],
  transports: { [bsc.id]: http(rpcUrl) },
});

export { bsc as stockfrontChain };
