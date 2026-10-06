import {
  decodeFunctionData,
  encodeFunctionData,
  erc20Abi,
  isAddressEqual,
  numberToHex,
  type Address,
  type Hex,
} from "viem";

type RpcRequest = {
  (args: {
    method: "eth_call";
    params: [
      {
        from?: Address;
        to: Address;
        data: Hex;
        value?: Hex;
      },
      "latest",
    ];
  }): Promise<Hex>;
};

type RpcClient = { request: unknown };

type PreparedApproval = {
  to: string;
  spender: string;
  data: string;
};

type PreparedTransaction = {
  to: string;
  data: string;
  value: string;
};

/**
 * Checks the exact allowance requested by a provider-produced ERC-20 approval.
 * Any malformed action or RPC failure returns false, so execution fails closed
 * and never skips an approval it cannot prove is unnecessary.
 */
export async function hasSufficientAllowance(
  client: RpcClient,
  owner: Address,
  action: PreparedApproval,
): Promise<boolean> {
  try {
    const request = client.request as RpcRequest;
    const decoded = decodeFunctionData({
      abi: erc20Abi,
      data: action.data as Hex,
    });
    if (decoded.functionName !== "approve") return false;
    const [spender, requiredAmount] = decoded.args;
    if (!isAddressEqual(spender, action.spender as Address)) return false;

    const allowance = await request({
      method: "eth_call",
      params: [
        {
          to: action.to as Address,
          data: encodeFunctionData({
            abi: erc20Abi,
            functionName: "allowance",
            args: [owner, spender],
          }),
        },
        "latest",
      ],
    });
    return allowance !== "0x" && BigInt(allowance) >= requiredAmount;
  } catch {
    return false;
  }
}

/** Runs the same read-only call a wallet uses to detect a reverting swap. */
export async function simulatePreparedTransaction(
  client: RpcClient,
  owner: Address,
  transaction: PreparedTransaction,
): Promise<void> {
  const request = client.request as RpcRequest;
  await request({
    method: "eth_call",
    params: [
      {
        from: owner,
        to: transaction.to as Address,
        data: transaction.data as Hex,
        value: numberToHex(BigInt(transaction.value)),
      },
      "latest",
    ],
  });
}
