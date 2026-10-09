import { useRecipeEditor } from "./store";

beforeEach(() => {
  localStorage.clear();
  useRecipeEditor.setState({
    nodes: [],
    edges: [],
    selectedNodeId: undefined,
    initialized: false,
    mode: "builder",
  });
});

it("restores a versioned draft with node positions and connections", () => {
  useRecipeEditor.getState().initialize({
    settlementToken: "0x55d398326f99059fF775485246999027B3197955",
    bstockToken: "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
  });
  const before = useRecipeEditor.getState();
  before.saveDraft();
  useRecipeEditor.setState({ nodes: [], edges: [], initialized: false });

  expect(useRecipeEditor.getState().restoreDraft()).toBe(true);
  const restored = useRecipeEditor.getState();
  expect(restored.nodes).toEqual(before.nodes);
  expect(restored.edges).toEqual(before.edges);
});

it("appends a trade to the selected output once", () => {
  const defaults = {
    settlementToken: "0x55d398326f99059fF775485246999027B3197955" as const,
    bstockToken: "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436" as const,
  };
  useRecipeEditor.getState().initialize(defaults);
  const source = useRecipeEditor.getState().nodes[1].id;
  useRecipeEditor.getState().appendTrade(source, defaults.settlementToken);
  const state = useRecipeEditor.getState();
  expect(state.nodes).toHaveLength(3);
  expect(state.edges).toHaveLength(2);
  expect(state.edges[1]).toMatchObject({
    source,
    sourceHandle: "buy",
    target: state.nodes[2].id,
    targetHandle: "sell",
  });
  expect(state.nodes[2].data).toMatchObject({
    kind: "TRADE",
    buyToken: defaults.settlementToken,
  });
  useRecipeEditor.getState().appendTrade(source, defaults.settlementToken);
  expect(useRecipeEditor.getState().nodes).toHaveLength(3);
});

it("appends a terminal Venus supply to a trade output once", () => {
  useRecipeEditor.getState().initialize({
    settlementToken: "0x55d398326f99059fF775485246999027B3197955",
    bstockToken: "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
  });
  const source = useRecipeEditor.getState().nodes[1].id;

  useRecipeEditor.getState().appendSupply(source);

  const state = useRecipeEditor.getState();
  expect(state.nodes[2]).toMatchObject({
    type: "supply",
    data: { kind: "SUPPLY", venue: "venus" },
  });
  expect(state.edges[1]).toMatchObject({
    source,
    sourceHandle: "buy",
    target: state.nodes[2].id,
    targetHandle: "asset",
  });
  useRecipeEditor.getState().appendSupply(source);
  expect(useRecipeEditor.getState().nodes).toHaveLength(3);
});

it("appends a terminal withdrawal to a supplied position once", () => {
  const pool = "0xfD36E2c2a6789Db23113685031d7F16329158384" as const;
  const token = "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436" as const;
  useRecipeEditor
    .getState()
    .addNode(
      "SUPPLIED_POSITION",
      { x: 70, y: 180 },
      { suppliedPosition: { venue: "venus", pool, token } },
    );
  const source = useRecipeEditor.getState().nodes[0].id;

  useRecipeEditor.getState().appendWithdraw(source);

  const state = useRecipeEditor.getState();
  expect(state.nodes[1]).toMatchObject({
    type: "withdraw",
    data: { kind: "WITHDRAW" },
  });
  expect(state.edges[0]).toMatchObject({
    source,
    sourceHandle: "position",
    target: state.nodes[1].id,
    targetHandle: "position",
  });
  useRecipeEditor.getState().appendWithdraw(source);
  expect(useRecipeEditor.getState().nodes).toHaveLength(2);
});

it("starts a new recipe with an empty board and removes the saved draft", () => {
  useRecipeEditor.getState().initialize({
    settlementToken: "0x55d398326f99059fF775485246999027B3197955",
    bstockToken: "0x02Fca66C1D1aFB4E2A7884261eB00F63598a7436",
  });
  useRecipeEditor.getState().saveDraft();

  useRecipeEditor.getState().startNewRecipe();

  expect(useRecipeEditor.getState()).toMatchObject({
    nodes: [],
    edges: [],
    selectedNodeId: undefined,
    mode: "builder",
    initialized: true,
  });
  expect(useRecipeEditor.getState().restoreDraft()).toBe(false);
});
