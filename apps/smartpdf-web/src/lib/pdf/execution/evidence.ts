import type { ContractResult, EvidencePolicy, EvidenceRecord } from "./contracts";
export function checkEvidence(policy: EvidencePolicy, record: EvidenceRecord): ContractResult<void> {
  if (record.source_revision !== policy.required_revision) {
    return { ok: false, refusal: { code: "StaleRevision", reason: "evidence revision is missing or stale" } };
  }
  if (!policy.accepted_levels.includes(record.level) || !record.proof.trim()) {
    return { ok: false, refusal: { code: "InsufficientEvidence", reason: `insufficient evidence for ${policy.operation}` } };
  }
  return { ok: true, value: undefined };
}
