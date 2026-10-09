import { BuilderPage } from "@/builder/BuilderPage";
import { ComposePage } from "@/compose/ComposePage";
import { LandingPage } from "@/landing/LandingPage";
import { PositionsSheet } from "@/positions/PositionsSheet";
import { RunPage } from "@/run/RunPage";
import { RecipeSessionProvider } from "@/session/RecipeSession";
import { ConnectWalletModal } from "@/wallet/ConnectWalletModal";
import { useRoute } from "./route";

export function App() {
  const route = useRoute();
  return (
    <>
      {route === "landing" ? (
        <LandingPage />
      ) : (
        <RecipeSessionProvider autoPreview={route === "builder"}>
          {route === "compose" ? (
            <ComposePage />
          ) : route === "builder" ? (
            <BuilderPage />
          ) : (
            <RunPage />
          )}
          <PositionsSheet />
        </RecipeSessionProvider>
      )}
      <ConnectWalletModal />
    </>
  );
}
