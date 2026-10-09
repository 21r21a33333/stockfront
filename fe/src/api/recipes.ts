import { api, apiResult } from "./client";
import type {
  ComposeStrategyRequest,
  PrepareRecipeRequest,
  RecipeDefinition,
  RecipePreviewRequest,
  SubmitOrderRequest,
} from "@/api/generated";

export const recipesApi = {
  catalog: () => apiResult(api.GET("/v1/recipe-primitives")),
  compose: (body: ComposeStrategyRequest) =>
    apiResult(api.POST("/v1/strategies/compose", { body })),
  validate: (body: RecipeDefinition) =>
    apiResult(api.POST("/v1/recipes/validate", { body })),
  preview: (body: RecipePreviewRequest) =>
    apiResult(api.POST("/v1/recipes/preview", { body })),
  prepare: (body: PrepareRecipeRequest) =>
    apiResult(api.POST("/v1/recipes/prepare", { body })),
  borrowingAccount: (address: string) =>
    apiResult(
      api.GET("/v1/wallets/{address}/loans", {
        params: { path: { address } },
      }),
    ),
  submitOrder: (body: SubmitOrderRequest) =>
    apiResult(api.POST("/v1/bstock-trades/orders", { body })),
  orderStatus: (orderId: string, signal?: AbortSignal) =>
    apiResult(
      api.GET("/v1/bstock-trades/orders/{order_id}", {
        params: { path: { order_id: orderId } },
        signal,
      }),
    ),
};
