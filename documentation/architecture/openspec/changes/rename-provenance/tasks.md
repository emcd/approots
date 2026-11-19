## 1. Rename

- [x] 1.1 Rename the `Provenance` variants (`CallerProvided` → `ExplicitDirectory`, `SuppliedDirectory` → `ExplicitBase`, `DevelopmentCommon`/`DevelopmentWorktree` → `Discovery`, `PlatformEnvironment` → `SysvarUserDirectory`, `PlatformDefault` → `DefaultUserDirectory`) and update every match in `src/` and `tests/`.
- [x] 1.2 Update all spec scenarios and rustdoc naming the old values; sweep generic prose too (not just enum tokens — e.g. `DiscoveredBase` rustdoc still narrating development-common/worktree provenance); verify no VCS-specific or topology-specific term remains in the generic vocabulary or docs.

## 2. Archive-time manual Purpose sync

- [ ] 2.1 At archive, replace the development-layout main-spec Purpose with the neutralized text (no Common-worktree/worktree-local/core-scope encoding): "Discovered development candidates with caller-selectable anchor scope and caller-overridable layout. Owns layout and anchor scope; delegates layer-list mechanics to `platform-roots`." Delta files cannot carry Purpose changes, so this is a tracked manual sync step.

## 3. Verification and review

- [x] 3.1 Verify gates (nextest, clippy with warnings denied, openspec strict, cross-target checks) and confirm byte-identical resolution output apart from labels.
- [ ] 3.2 Route the packet to Tier-1, then Tier-2 on Tier-1 approval, with explicit instruction to verify the generic vocabulary carries no topology-specific terms, and with no pushes of unapproved commits.
