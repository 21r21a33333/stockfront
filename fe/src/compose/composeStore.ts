import { create } from "zustand";
import type { ComposedStrategy } from "@/api/generated";

/** Keeps the last plan across route changes and carries a prompt typed on the landing page. */
export const useComposeStore = create<{
  pendingPrompt?: string;
  prompt: string;
  composed?: ComposedStrategy;
  setPendingPrompt(prompt?: string): void;
  setPrompt(prompt: string): void;
  setComposed(composed?: ComposedStrategy): void;
}>((set) => ({
  prompt: "",
  setPendingPrompt: (pendingPrompt) => set({ pendingPrompt }),
  setPrompt: (prompt) => set({ prompt }),
  setComposed: (composed) => set({ composed }),
}));
