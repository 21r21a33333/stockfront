import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { WagmiProvider } from "wagmi";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { walletConfig } from "@/wallet/config";
import { App } from "@/app/App";
import { ErrorBoundary } from "@/app/ErrorBoundary";

import "@fontsource-variable/geist";
import "@fontsource/geist-mono/400.css";
import "@fontsource/geist-mono/500.css";
import "@fontsource/space-grotesk/500.css";
import "@fontsource/space-grotesk/600.css";
import "./styles/tokens.css";
import "./styles/base.css";

const root = document.getElementById("root");
if (!root) throw new Error("#root is missing from index.html");
const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000, retry: 1, refetchOnWindowFocus: false },
    mutations: { retry: 0 },
  },
});

createRoot(root).render(
  <StrictMode>
    <ErrorBoundary>
      <WagmiProvider config={walletConfig}>
        <QueryClientProvider client={queryClient}>
          <App />
        </QueryClientProvider>
      </WagmiProvider>
    </ErrorBoundary>
  </StrictMode>,
);
