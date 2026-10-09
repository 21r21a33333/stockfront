import {
  decodeEventLog,
  erc20Abi,
  isAddressEqual,
  type Address,
  type Hex,
} from "viem";

export interface TransferLog {
  address: Address;
  data: Hex;
  topics: readonly Hex[];
}

/** Returns the exact increase of one token in the wallet during one transaction. */
export function exactBuyAmountFromTransferLogs(
  logs: readonly TransferLog[],
  token: Address,
  wallet: Address,
): bigint | undefined {
  const delta = logs.reduce((amount, log) => {
    if (!isAddressEqual(log.address, token)) return amount;
    if (!log.topics.length) return amount;
    try {
      const transfer = decodeEventLog({
        abi: erc20Abi,
        eventName: "Transfer",
        data: log.data,
        topics: [...log.topics] as [Hex, ...Hex[]],
        strict: true,
      });
      const received = isAddressEqual(transfer.args.to, wallet)
        ? transfer.args.value
        : 0n;
      const sent = isAddressEqual(transfer.args.from, wallet)
        ? transfer.args.value
        : 0n;
      return amount + received - sent;
    } catch {
      return amount;
    }
  }, 0n);
  return delta > 0n ? delta : undefined;
}
