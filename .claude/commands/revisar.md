---
description: Revisa los cambios actuales antes de commit/PR
---

Revisa los cambios actuales (`git diff` y archivos nuevos sin seguimiento):

1. Ejecuta `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test` en los crates afectados (y `npm run lint` si hay cambios de UI).
2. Contrasta los cambios con los principios de `CLAUDE.md`, en especial: simplicidad, que no se filtren teclas del pad, y que no se registren pulsaciones de otros teclados.
3. Revisa los bloques `unsafe`: cada uno con su nota `// SAFETY:` y aislado en funciones pequeñas.
4. Señala código muerto, `unwrap()` fuera de tests/spikes, dependencias nuevas sin justificar y lógica sin tests que podría tenerlos.

Entrégame una lista corta ordenada por severidad y propón un mensaje de commit en Conventional Commits. No hagas el commit.
