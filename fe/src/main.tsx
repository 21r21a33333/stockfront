import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { WagmiProvider } from "wagmi";
import { walletConfig } from "@/wallet/config";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RecipeBuilderPage } from "@/recipe/RecipeBuilderPage";

import "@fontsource-variable/geist";
import "@fontsource/geist-mono/400.css";
import "@fontsource/geist-mono/500.css";
import "@xyflow/react/dist/style.css";
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
    <WagmiProvider config={walletConfig}>
      <QueryClientProvider client={queryClient}>
        <RecipeBuilderPage />
      </QueryClientProvider>
    </WagmiProvider>
  </StrictMode>,
);
