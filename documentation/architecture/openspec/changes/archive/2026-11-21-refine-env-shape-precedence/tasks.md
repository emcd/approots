## 1. Declaration and shape precedence

- [x] 1.1 Allow one declaration per (class, shape) pair; keep rejecting same-shape duplicates with `DuplicateEnvDeclaration`.
- [x] 1.2 After capture and before tier choice, validate both declared values; apply exact-directory before layout base in `resolve` and family gap-filling.

## 2. Contract tests

- [x] 2.1 Pin exact-beats-base when both are valid, and typed refusal when either shape is malformed even though the other would win (child-process fixtures, no runner env mutation).
- [x] 2.2 Pin single-shape behavior unchanged (all existing env tests keep passing).

## 3. Verification and review

- [x] 3.1 Verify gates (nextest, clippy with warnings denied, openspec strict, cross-target checks).
- [ ] 3.2 Route the packet to Tier-1, then Tier-2 on Tier-1 approval, with no pushes of unapproved commits.
