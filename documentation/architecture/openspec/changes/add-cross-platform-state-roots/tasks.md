## 1. State root mapping

- [x] 1.1 Base the Windows state root on the provider's local application-data base and the macOS state root on its application-support base in the platform adapter.
- [x] 1.2 Confirm Linux state behavior is unchanged (XDG override and home-derived default, existing snapshot tests still pass).

## 2. Cross-platform test execution

- [x] 2.1 Add all-kind platform availability coverage asserting every kind resolves to an absolute path ending in the validated application name on the host platform.
- [x] 2.2 Add Windows tests executing the drive-relative and root-relative explicit-root refusal.
- [x] 2.3 Add Windows discovery coverage through a junction or directory symlink (capability-probed) and a non-ASCII checkout name.
- [x] 2.4 Update the source-export fallthrough test to expect the resolved state root now that every supported platform defines one.

## 3. Verification

- [ ] 3.1 Run nextest, clippy with warnings denied, and strict OpenSpec validation locally; compile-check tests for the Windows and macOS targets.
- [ ] 3.2 Push and confirm the GitHub `qa` matrix is green on Linux, macOS, and Windows.
