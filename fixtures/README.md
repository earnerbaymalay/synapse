# Fixtures

One golden config tree per supported tool, named after its adapter id (see
`docs/research/<tool>.md` for the paths/format each tree must match). Phase 3
(T3.1) adapters detect/import/project against these trees and must round-trip
them byte-for-byte on the parts they don't touch.

`fixtures/cursor/` is the reference tree; the other 11 tools land alongside
it as their adapters are built. (Claude Code's own paths, e.g. `.claude/`,
are deliberately not used as the reference tree here — the harness treats
those as sensitive real-config paths, and per docs/CODE_REVIEW.md §2.1 that
collision is exactly the kind of bug the fixtures should help catch, not
reproduce.)
