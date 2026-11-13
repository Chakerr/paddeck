# Captura de teclado en Windows — notas de investigación

Documento vivo de la Fase 0. Ver criterios de aceptación en `PLAN.md` §5.

## Enfoque
Raw Input + `WH_KEYBOARD_LL` con correlación. Ver ADR 0002.

## Hallazgos
- **Paso 1 (Raw Input).** Cada teclado llega con una ruta de interfaz distinta, aunque se presione la misma tecla.
- La ruta (`RIDI_DEVICENAME`) **se mantiene igual** al desconectar y reconectar el teclado, así que sirve como identificador persistente.
- Un mismo teclado puede exponer varias interfaces de teclado (`VID_258A`: `MI_00` y `MI_01&Col04`); las teclas llegan solo por una. Hay que registrar la interfaz que envía, no el VID/PID.
- NumLock no cambia el scancode: el `1` del numpad es siempre `0x4F`, y solo cambia el vkey (`0x61` con NumLock activo, `0x23`/Fin con NumLock inactivo). Confirma que hay que mapear por scancode + E0.
- **Paso 2 (hook LL + Raw Input).**
  - Esperar `WM_INPUT` *dentro* del hook (bombeando solo `WM_INPUT` durante 50 ms) no funciona: 136 de 136 teclas dieron timeout. Windows no genera el Raw Input mientras el hook LL está corriendo.
  - Si el hook retorna sin bloquear, `WM_INPUT` llega justo después: mediana 0.43 ms, p95 0.75 ms, máximo 3.6 ms, en 814 de 816 eventos.
  - **Si el hook bloquea la tecla (`return 1`), Windows no genera `WM_INPUT`**: 0 de unas 150 teclas bloqueadas del numpad. Con un hook LL no hay forma de bloquear una tecla y además saber de qué dispositivo vino.
  - Conclusión: los planes A y B del ADR 0002 no son viables tal como están escritos.

## Equipos de prueba
| Equipo | Windows | Teclado principal | Pad numérico (modelo, VID/PID) |
|---|---|---|---|
| Luis (Win 11 Pro 26200) | 11 | `VID_258A&PID_0049` | `VID_17EF&PID_60A9` (Lenovo con numpad, hace de pad) |

## Resultados de pruebas
| Fecha | Prueba | Resultado | Notas |
|---|---|---|---|
| 2026-11-13 | Paso 1: distinguir dispositivos | OK | Rutas distintas con la misma tecla |
| 2026-11-13 | Paso 1: reconexión | OK | Misma ruta tras reconectar |
| 2026-11-13 | Paso 1: NumLock | OK | Scan `0x4F` estable; vkey cambia |
| 2026-11-13 | Paso 2: esperar WM_INPUT dentro del hook | FALLA | 136/136 timeouts de 50 ms |
| 2026-11-13 | Paso 2: retraso de WM_INPUT tras el hook | OK | p50 0.43 ms, p95 0.75 ms |
| 2026-11-13 | Paso 2: WM_INPUT de teclas bloqueadas | FALLA | No llega ninguno |

## Criterios de aceptación
- [ ] El pad registrado no escribe en otras apps; el teclado principal (incluido su numpad) sí.
- [ ] Funciona con NumLock activo e inactivo.
- [ ] Latencia añadida al teclado principal imperceptible.
- [ ] Sin teclas pegadas tras 10 min de pulsaciones rápidas alternadas.
- [ ] Reconoce el pad tras desconectarlo y reconectarlo.
