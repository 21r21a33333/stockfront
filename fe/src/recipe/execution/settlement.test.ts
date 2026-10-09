import {
  encodeAbiParameters,
  encodeEventTopics,
  erc20Abi,
  type Address,
  type Hash,
  type Log,
} from "viem";
import { exactBuyAmountFromTransferLogs } from "./settlement";

const wallet = "0x1111111111111111111111111111111111111111" as const;
const token = "0x2222222222222222222222222222222222222222" as const;
const other = "0x3333333333333333333333333333333333333333" as const;

it("uses the transaction-scoped net wallet delta for the expected token", () => {
  const logs = [
    transfer(token, other, wallet, 20n),
    transfer(token, wallet, other, 3n),
    transfer(other, other, wallet, 100n),
    transfer(token, other, other, 50n),
  ];

  expect(exactBuyAmountFromTransferLogs(logs, token, wallet)).toBe(17n);
});

it("returns no output when the expected token does not increase", () => {
  expect(
    exactBuyAmountFromTransferLogs(
      [transfer(token, wallet, other, 3n)],
      token,
      wallet,
    ),
  ).toBeUndefined();
});

it("fails closed when an expected-token transfer log is malformed", () => {
  const malformed = {
    ...transfer(token, other, wallet, 20n),
    data: "0x01" as const,
  };

  expect(
    exactBuyAmountFromTransferLogs([malformed], token, wallet),
  ).toBeUndefined();
});

function transfer(address: Address, from: Address, to: Address, value: bigint) {
  return {
    address,
    topics: encodeEventTopics({
      abi: erc20Abi,
      eventName: "Transfer",
      args: { from, to },
    }) as Log["topics"],
    data: encodeAbiParameters([{ type: "uint256" }], [value]),
    blockHash: `0x${"00".repeat(32)}` as Hash,
    blockNumber: 1n,
    logIndex: 0,
    transactionHash: `0x${"11".repeat(32)}` as Hash,
    transactionIndex: 0,
    removed: false,
  };
}
