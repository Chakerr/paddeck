# Captura de teclado en Windows — notas de investigación

Documento vivo de la Fase 0. Ver criterios de aceptación en `PLAN.md` §5.

## Enfoque
Raw Input + `WH_KEYBOARD_LL` con correlación. Ver ADR 0002.

## Hallazgos
- **Paso 1 (Raw Input).** Cada teclado llega con una ruta de interfaz distinta, aunque se presione la misma tecla.
- La ruta (`RIDI_DEVICENAME`) **se mantiene igual** al desconectar y reconectar el teclado, así que sirve como identificador persistente.
- Un mismo teclado puede exponer varias interfaces de teclado (`VID_258A`: `MI_00` y `MI_01&Col04`); las teclas llegan solo por una. Hay que registrar la interfaz que envía, no el VID/PID.
- NumLock no cambia el scancode: el `1` del numpad es siempre `0x4F`, y solo cambia el vkey (`0x61` con NumLock activo, `0x23`/Fin con NumLock inactivo). Confirma que hay que mapear por scancode + E0.

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

## Criterios de aceptación
- [ ] El pad registrado no escribe en otras apps; el teclado principal (incluido su numpad) sí.
- [ ] Funciona con NumLock activo e inactivo.
- [ ] Latencia añadida al teclado principal imperceptible.
- [ ] Sin teclas pegadas tras 10 min de pulsaciones rápidas alternadas.
- [ ] Reconoce el pad tras desconectarlo y reconectarlo.
