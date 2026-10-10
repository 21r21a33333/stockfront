import type {
  PreparedRecipeStep,
  RecipeDefinition,
  RecipeNodeOutput,
  RecipePrimitive,
} from "@/api/generated";
import type { Address, Hash } from "viem";
import type {
  ExecutionAttempt,
  ExecutionMode,
  ExecutionPhase,
  ExecutionReference,
  ExecutionRun,
} from "./model";

export * from "./model";

/**
 * Every action and mode a stored attempt may name, as records so the compiler
 * refuses a primitive the recipe language has gained but the journal has not.
 * A missing one is written and then rejected on the way back, stranding the run.
 */
const KINDS: Record<PreparedRecipeStep["kind"], true> = {
  APPROVAL: true,
  TRADE: true,
  SUPPLY: true,
  EARN_DEPOSIT: true,
  EARN_WITHDRAW: true,
  WITHDRAW: true,
  ENABLE_COLLATERAL: true,
  BORROW: true,
  REPAY: true,
};

/**
 * What each recipe node must carry to be trusted after a reload, one entry per
 * primitive so a new one cannot be stored and then refused on the way back.
 */
const NODES: Record<
  RecipePrimitive["kind"],
  (node: Record<string, unknown>) => boolean
> = {
  WALLET_ASSET: (node) => address(node.token) && unsigned(node.rawAmount),
  SUPPLIED_POSITION: (node) =>
    node.venue === "venus" && address(node.pool) && address(node.token),
  BORROWED_POSITION: (node) =>
    node.venue === "venus" && address(node.pool) && address(node.token),
  TRADE: (node) => address(node.buyToken),
  SUPPLY: (node) => node.venue === "venus",
  WITHDRAW: () => true,
  BORROW: (node) => address(node.debtToken) && unsigned(node.rawAmount),
  REPAY: () => true,
  EARN_DEPOSIT: (node) =>
    address(node.collateralToken) && typeof node.investmentId === "string",
  EARN_POSITION: (node) => typeof node.investmentId === "string",
  EARN_WITHDRAW: () => true,
};

const MODES: Record<ExecutionMode, true> = {
  SWAP: true,
  RFQ: true,
  SUPPLY: true,
  EARN_DEPOSIT: true,
  EARN_WITHDRAW: true,
  WITHDRAW: true,
  BORROW: true,
  REPAY: true,
};

const PHASES: ExecutionPhase[] = [
  "awaiting",
  "submitting",
  "pending",
  "complete",
  "failed",
  "attention",
];

export function runKey(wallet: string, chainId: number): string {
  return `stockfront:execution:${chainId}:${wallet.toLowerCase()}`;
}

export function readRun(
  wallet: string,
  chainId: number,
): ExecutionRun | undefined {
  try {
    const raw = localStorage.getItem(runKey(wallet, chainId));
    if (raw === null) return undefined;
    const value: unknown = JSON.parse(raw);
    const run =
      isRecord(value) && value.version === 1 ? migrateV1(value) : value;
    if (!validRun(run, wallet, chainId)) throw new Error("Invalid journal");
    return run;
  } catch {
    throw new Error(
      "Recovery data could not be read. Do not retry a spend until the previous run is reviewed.",
    );
  }
}

export function writeRun(run: ExecutionRun): void {
  // Persist references and outcomes only. Calldata and wallet signatures stay in memory.
  const { version, id, wallet, chainId, recipe, createdAtMs, updatedAtMs } =
    run;
  const attempts = run.attempts.map(
    ({
      id,
      nodeId,
      kind,
      attempt,
      phase,
      executionMode,
      steps,
      reference,
      firstSubmittedAtMs,
      approvalSkipped,
      amountsUnavailable,
      settledOutput,
      expectedOutput,
      error,
    }) => ({
      id,
      nodeId,
      kind,
      attempt,
      phase,
      executionMode,
      steps,
      reference,
      firstSubmittedAtMs,
      approvalSkipped,
      amountsUnavailable,
      settledOutput,
      expectedOutput,
      error,
    }),
  );
  localStorage.setItem(
    runKey(wallet, chainId),
    JSON.stringify({
      version,
      id,
      wallet,
      chainId,
      recipe,
      createdAtMs,
      updatedAtMs,
      attempts,
    }),
  );
}

function validRun(
  value: unknown,
  wallet: string,
  chainId: number,
): value is ExecutionRun {
  if (!isRecord(value) || !validRecipe(value.recipe, chainId)) return false;
  const recipe = value.recipe;
  return (
    value.version === 2 &&
    nonEmpty(value.id) &&
    address(value.wallet) &&
    value.wallet.toLowerCase() === wallet.toLowerCase() &&
    value.chainId === chainId &&
    finite(value.createdAtMs) &&
    finite(value.updatedAtMs) &&
    Array.isArray(value.attempts) &&
    value.attempts.every((item) => validAttempt(item, recipe))
  );
}

function validRecipe(
  value: unknown,
  chainId: number,
): value is RecipeDefinition {
  if (!isRecord(value)) return false;
  return (
    value.schemaVersion === 1 &&
    value.chainId === chainId &&
    Array.isArray(value.nodes) &&
    value.nodes.every(
      (node) =>
        isRecord(node) &&
        nonEmpty(node.id) &&
        typeof node.kind === "string" &&
        node.kind in NODES &&
        NODES[node.kind as RecipePrimitive["kind"]](node),
    ) &&
    Array.isArray(value.edges) &&
    value.edges.every(
      (edge) =>
        isRecord(edge) &&
        nonEmpty(edge.id) &&
        isRecord(edge.from) &&
        nonEmpty(edge.from.nodeId) &&
        nonEmpty(edge.from.port) &&
        isRecord(edge.to) &&
        nonEmpty(edge.to.nodeId) &&
        nonEmpty(edge.to.port),
    )
  );
}

function validAttempt(value: unknown, recipe: RecipeDefinition): boolean {
  if (!isRecord(value)) return false;
  const node = recipe.nodes.find((item) => item.id === value.nodeId);
  const output = value.settledOutput;
  const outputMatchesNode =
    output === undefined ||
    (value.phase === "complete" && validNodeOutput(output, node));
  const expected = value.expectedOutput;
  const expectedMatchesNode =
    expected === undefined ||
    (value.kind === "BORROW" && validNodeOutput(expected, node));
  return (
    nonEmpty(value.id) &&
    nonEmpty(value.nodeId) &&
    recipe.nodes.some((node) => node.id === value.nodeId) &&
    known(KINDS, value.kind) &&
    PHASES.includes(value.phase as ExecutionPhase) &&
    Number.isInteger(value.attempt) &&
    Number(value.attempt) > 0 &&
    Array.isArray(value.steps) &&
    value.steps.length > 0 &&
    value.steps.every((step) => known(KINDS, step)) &&
    optionalString(value.error) &&
    optionalBoolean(value.approvalSkipped) &&
    optionalBoolean(value.amountsUnavailable) &&
    optionalFinite(value.firstSubmittedAtMs) &&
    (value.executionMode === undefined || known(MODES, value.executionMode)) &&
    validReference(value.reference) &&
    outputMatchesNode &&
    expectedMatchesNode
  );
}

function known(allowed: Record<string, true>, value: unknown): boolean {
  return typeof value === "string" && value in allowed;
}

function validReference(value: unknown): boolean {
  if (value === undefined) return true;
  if (!isRecord(value)) return false;
  if (value.kind === "CALL_BATCH")
    return (
      nonEmpty(value.id) &&
      typeof value.atomicRequired === "boolean" &&
      optionalHash(value.transactionHash)
    );
  if (value.kind === "RFQ_ORDER")
    return (
      nonEmpty(value.submissionId) &&
      optionalString(value.orderId) &&
      optionalHash(value.transactionHash)
    );
  return value.kind === "LEGACY_TRANSACTION" && hash(value.transactionHash);
}

function validNodeOutput(
  value: unknown,
  node: RecipeDefinition["nodes"][number] | undefined,
): boolean {
  if (!isRecord(value) || !isRecord(value.asset)) return false;
  const expected =
    node?.kind === "TRADE"
      ? { port: "buy", token: node.buyToken }
      : node?.kind === "BORROW"
        ? { port: "borrowed", token: node.debtToken }
        : undefined;
  if (!expected || value.port !== expected.port) return false;
  return (
    address(value.asset.token) &&
    value.asset.token.toLowerCase() === expected.token.toLowerCase() &&
    unsigned(value.asset.amount) &&
    BigInt(value.asset.amount) > 0n
  );
}

// Version 1 stored approval and swap as separate rows. Collapse a shared call
// batch into the single wallet action the user actually approved.
function migrateV1(value: Record<string, unknown>): unknown {
  if (!validV1Shell(value)) return value;
  const recipe = value.recipe as RecipeDefinition;
  const old = value.attempts as Record<string, unknown>[];
  const attempts: ExecutionAttempt[] = [];

  for (const item of old) {
    if (item.kind === "APPROVAL" && sharedTrade(old, item)) continue;
    const phase = migratePhase(item.phase);
    const callBatchId = stringValue(item.callBatchId);
    const transactionHash = hashValue(
      item.replacementHash ?? item.transactionHash,
    );
    const submissionId = stringValue(item.submissionId);
    const orderId = stringValue(item.orderId);
    const reference: ExecutionReference | undefined = callBatchId
      ? {
          kind: "CALL_BATCH",
          id: callBatchId,
          atomicRequired: Boolean(sharedApproval(old, item)),
          transactionHash,
        }
      : submissionId
        ? { kind: "RFQ_ORDER", submissionId, orderId, transactionHash }
        : transactionHash
          ? { kind: "LEGACY_TRANSACTION", transactionHash }
          : undefined;
    const nodeId = String(item.nodeId ?? "");
    const kind = item.kind === "APPROVAL" ? "APPROVAL" : "TRADE";
    attempts.push({
      id: `${nodeId}:${Number(item.attempt) || 1}:${attempts.length}`,
      nodeId,
      kind,
      attempt: Number(item.attempt) || 1,
      phase:
        (phase === "awaiting" || phase === "submitting") && !reference
          ? "attention"
          : phase,
      executionMode:
        item.executionMode === "RFQ" || item.executionMode === "SWAP"
          ? item.executionMode
          : undefined,
      steps: sharedApproval(old, item) ? ["APPROVAL", "TRADE"] : [kind],
      reference,
      firstSubmittedAtMs: finiteValue(item.firstSubmittedAtMs),
      approvalSkipped: booleanValue(item.approvalSkipped),
      amountsUnavailable: booleanValue(item.amountsUnavailable),
      settledOutput: validNodeOutput(
        item.settledOutput,
        recipe.nodes.find((node) => node.id === nodeId),
      )
        ? (item.settledOutput as RecipeNodeOutput)
        : undefined,
      error: stringValue(item.error),
    });
  }

  return { ...value, version: 2, recipe, attempts };
}

function validV1Shell(value: Record<string, unknown>): boolean {
  return (
    value.version === 1 &&
    isRecord(value.recipe) &&
    Array.isArray(value.attempts) &&
    value.attempts.every(isRecord)
  );
}

function sharedTrade(
  attempts: Record<string, unknown>[],
  approval: Record<string, unknown>,
): boolean {
  return attempts.some(
    (item) =>
      item.kind === "TRADE" &&
      item.nodeId === approval.nodeId &&
      nonEmpty(approval.callBatchId) &&
      item.callBatchId === approval.callBatchId,
  );
}

function sharedApproval(
  attempts: Record<string, unknown>[],
  trade: Record<string, unknown>,
): boolean {
  return attempts.some(
    (item) =>
      item.kind === "APPROVAL" &&
      item.nodeId === trade.nodeId &&
      nonEmpty(trade.callBatchId) &&
      item.callBatchId === trade.callBatchId,
  );
}

function migratePhase(value: unknown): ExecutionPhase {
  if (value === "ready") return "awaiting";
  return PHASES.includes(value as ExecutionPhase)
    ? (value as ExecutionPhase)
    : "attention";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
function nonEmpty(value: unknown): value is string {
  return typeof value === "string" && value.length > 0;
}
function address(value: unknown): value is Address {
  return typeof value === "string" && /^0x[\da-f]{40}$/i.test(value);
}
function hash(value: unknown): value is Hash {
  return typeof value === "string" && /^0x[\da-f]{64}$/i.test(value);
}
function unsigned(value: unknown): value is string {
  return typeof value === "string" && /^\d+$/.test(value);
}
function finite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}
function optionalString(value: unknown): boolean {
  return value === undefined || typeof value === "string";
}
function optionalBoolean(value: unknown): boolean {
  return value === undefined || typeof value === "boolean";
}
function optionalFinite(value: unknown): boolean {
  return value === undefined || finite(value);
}
function optionalHash(value: unknown): boolean {
  return value === undefined || hash(value);
}
function stringValue(value: unknown): string | undefined {
  return typeof value === "string" && value ? value : undefined;
}
function hashValue(value: unknown): Hash | undefined {
  return hash(value) ? value : undefined;
}
function finiteValue(value: unknown): number | undefined {
  return finite(value) ? value : undefined;
}
function booleanValue(value: unknown): boolean | undefined {
  return typeof value === "boolean" ? value : undefined;
}
