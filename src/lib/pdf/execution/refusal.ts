import type { ContractRefusal, RefusalCode } from "./contracts";

/** Synchronous codecs throw a typed refusal; async adapters use ContractResult. */
export class ContractError extends Error implements ContractRefusal {
  constructor(readonly code: RefusalCode, readonly reason: string) {
    super(`${code}: ${reason}`);
    this.name = "ContractError";
  }
}
