# 0001 — Stack base

- **Estado:** aceptada
- **Fecha:** 2026-11-13

## Contexto
App de escritorio para usuarios no técnicos, distribuida gratis como open source. Debe ser liviana, fácil de instalar, con acceso a APIs de bajo nivel de Windows para capturar teclado y con posibilidad de llegar a Linux/macOS.

## Decisión
Tauri 2 con backend en Rust y UI en React + TypeScript. Licencia GPL-3.0. MVP solo Windows.

## Alternativas consideradas
- **Electron + addon nativo:** instaladores de 100+ MB y un addon en C++ para la captura; más pesado de mantener.
- **C# / WinUI:** rápido para Windows, pero cierra la puerta a otras plataformas.
- **Python:** bueno para prototipos, distribución y rendimiento del hook menos adecuados.
- **MIT en lugar de GPL-3.0:** más adopción, pero permite forks cerrados; el objetivo es que siga siendo de la comunidad.

## Consecuencias
- Instalador pequeño (~10 MB) y auto-updater incluido.
- Curva de Rust para contribuidores; se mitiga con módulos pequeños y documentación.
- La captura queda detrás de `InputBackend` para añadir otros SO.
