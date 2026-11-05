# Applying Template Updates

This guide covers updating a project built from the agents-common
Copier template, and the `agentsmgr populate` step that goes with it.
It is template-owned: corrections land here, not in per-project files.

## Standard update

Run the update non-interactively with the data file, then populate
from this repository's `distribution/` tree:

```sh
copier update \
    --data-file https://raw.githubusercontent.com/emcd/agents-common/<ref>/.auxiliary/data/copier/single-source-instructions.yaml \
    --answers-file .auxiliary/configuration/copier-answers--agents.yaml \
    --skip-answered
agentsmgr populate project <source>/distribution <target>
```

Track the latest release by default: a bare `copier update` selects
the latest template tag on its own. Pin `--vcs-ref` to a tag only when
a specific version was explicitly requested (for example, to reproduce
an exact run); a pin that merely dodges an environment problem is a
workaround, not a fix — fix the environment instead. Whatever ref the
update uses, the data-file URL must name the same tag in place of
`<ref>`: the file carries renamed answers so the update does not
silently keep a retired layout, and it holds no guidance beyond that.
(If this project also uses the Rust template, it keeps a separate
answers file alongside this one; each template's copy and populate
leaves the other's answers file untouched.)

Do not pass `--quiet`: the post-update banner is suppressed in quiet
mode, and you will miss conflict and follow-up notices.

## Ordering rules

- **Update first, then populate.** Populate aborts when the
  template-rendered `.auxiliary/agents/agents.md` does not exist yet.
- **Review before populating.** Resolve update conflicts first; do
  not populate over a conflicted tree.
- **Commit together.** Land the update and the populate in the same
  changeset. Committing the update first leaves downstream clones
  holding half-migrated trees.

## Retired content

When the template retires something, delete leftover tracked copies
outright. Do not add exclude entries for retired paths: removed means
absent, not excluded. Populate copies but never deletes, so stale
trees (old instruction homes, retired `coders/*/agents` and
`coders/*/commands` directories, legacy `*.rst` standards) must be
removed by hand.

A moved or reformatted key is not a line-level change: Copier's merge
can keep both the stale local copy and the template's new one (seen
with a duplicated JSON key that the `check-json` hook then rejects).
After updating, treat hook failures about duplicate keys as this
shape until proven otherwise — delete the stale copy by hand so the
file matches a fresh template render.

## Custom skills

Project-owned skills live in `.auxiliary/agents/skills/` (directory
package with `SKILL.md`, or flat `<name>.md`). Populate merges into
that tree and never deletes foreign entries. Only the per-coder
`skills/` discovery symlinks are replaced on populate, so move any
custom skills out of `coders/*/skills/` first — or delete them if
vestigial.

## Legacy real paths

Populate retargets a wrong symlink but only warns and skips a real
path. Before populating, check whether `.claude/`, `.opencode/`, and
root memory files are symlinks or legacy real directories/files: a
legacy real path blocks its managed symlink from materializing.
Remove legacy real paths by hand (after rescuing anything
project-owned).

## Rollback

- Recorded answers persist: re-running the update without the data
  file does NOT restore prior values — Copier keeps the recorded
  answers. To roll back the answers migration, read the prior value
  from version control without touching the worktree (checking it
  out would dirty the tree and trip the clean-tree gate), pass it
  back in as data, then re-run populate so generated output matches:

      PRIOR=$(git show <pre-update-ref>:.auxiliary/configuration/copier-answers--agents.yaml | grep instructions_target | awk '{print $2}')
      copier update --answers-file .auxiliary/configuration/copier-answers--agents.yaml --skip-answered --data instructions_target=$PRIOR
      agentsmgr populate project <source>/distribution <target>

- Populated content: first rescue any project-owned files from the
  managed trees — custom skills in `.auxiliary/agents/skills/`
  coexist with distributed ones and must be preserved — then delete
  the managed trees and re-populate.
