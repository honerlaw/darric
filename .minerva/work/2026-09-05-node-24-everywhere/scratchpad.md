# Scratchpad: node-24-everywhere

## Quick decisions 2026-09-05

- [decided] scope check: single unfazed unit — five files, no public interface, no runtime code touched
- [decided] approach: `.nvmrc` as single source with `node-version-file` in both workflows (rejected hardcoding 24 in five places, which recreates the README/CI drift that motivates the unit; rejected `engine-strict=true`, a hard install gate broader than the seed asks for; rejected pinning `engines` to `^24`, since the risk is "too old" not "too new")
- [decided] `@types/node` `^25` → `^24`: types describing a major the project does not run let `tsconfig.node.json` code typecheck against APIs Node 24 lacks. Aligning to the runtime is dominant; the typecheck gate proves it either way (solo)
- [decided] whole-proposal soundness: bounded and internally consistent; no public interface or cross-cutting contract
