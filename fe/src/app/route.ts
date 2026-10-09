import { useSyncExternalStore } from "react";

export type Route = "landing" | "compose" | "builder" | "run";

const routes: Record<string, Route> = {
  "/compose": "compose",
  "/builder": "builder",
  "/run": "run",
};

function routeFromHash(hash: string): Route {
  return routes[hash.replace(/^#/, "")] ?? "landing";
}

export function navigate(route: Route): void {
  window.location.hash = route === "landing" ? "/" : `/${route}`;
  window.scrollTo({ top: 0 });
}

function subscribe(listener: () => void): () => void {
  window.addEventListener("hashchange", listener);
  return () => window.removeEventListener("hashchange", listener);
}

export function useRoute(): Route {
  return useSyncExternalStore(subscribe, () =>
    routeFromHash(window.location.hash),
  );
}
