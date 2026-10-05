import { api, apiResult } from "./client";
import type {
  PrepareRecipeRequest,
  RecipeDefinition,
  RecipePreviewRequest,
  SubmitOrderRequest,
} from "@/api/generated";

export const recipesApi = {
  catalog: () => apiResult(api.GET("/v1/recipe-primitives")),
  validate: (body: RecipeDefinition) =>
    apiResult(api.POST("/v1/recipes/validate", { body })),
  preview: (body: RecipePreviewRequest) =>
    apiResult(api.POST("/v1/recipes/preview", { body })),
  prepare: (body: PrepareRecipeRequest) =>
    apiResult(api.POST("/v1/recipes/prepare", { body })),
  submitOrder: (body: SubmitOrderRequest) =>
    apiResult(api.POST("/v1/bstock-trades/orders", { body })),
  orderStatus: (orderId: string, signal?: AbortSignal) =>
    apiResult(
      api.GET("/v1/bstock-trades/orders/{order_id}", {
        params: { path: { order_id: orderId } },
        signal,
      }),
    ),
  transactionStatus: (transactionHash: string, signal?: AbortSignal) =>
    apiResult(
      api.GET("/v1/bstock-trades/transactions/{transaction_hash}", {
        params: { path: { transaction_hash: transactionHash } },
        signal,
      }),
    ),
};
