# 0002 — Captura de teclado por dispositivo en Windows

- **Estado:** propuesta
- **Fecha:** 2026-11-13

## Contexto
Hay que saber qué teclado envió cada tecla y bloquear solo las del pad registrado (`PLAN.md` §5), sin drivers. Windows no ofrece una API que haga las dos cosas: Raw Input identifica el dispositivo pero no bloquea, y `WH_KEYBOARD_LL` bloquea pero no sabe de qué dispositivo viene la tecla.

## Decisión
Un hilo dedicado con una ventana *message-only* que registra Raw Input (`RIDEV_INPUTSINK | RIDEV_DEVNOTIFY`) e instala `WH_KEYBOARD_LL`.

- Cada evento Raw Input se guarda en un buffer corto: `hDevice`, makecode, flag E0, make/break y hora.
- En el hook se busca un evento del buffer que coincida por scancode + E0 + make/break. Si no hay ninguno, se bombea solo `WM_INPUT` (`PeekMessage`) con un timeout de unos 30 ms.
- Si la tecla viene del pad registrado, se bloquea (`return 1`) y se emite un `KeyEvent`. En cualquier otro caso, incluido el timeout, se deja pasar: si hay duda, el teclado principal nunca se rompe.
- Las teclas se identifican por scancode + E0, no por vkey, para que NumLock no cambie nada.
- El pad se identifica por la ruta de interfaz del dispositivo (`RIDI_DEVICENAME`), no por `hDevice`, porque `hDevice` cambia al reconectar.
- Las teclas inyectadas por la propia app llevan una marca en `dwExtraInfo` y se ignoran.

**Riesgo a validar primero:** el orden real entre el hook LL y `WM_INPUT`. Si `WM_INPUT` solo se genera *después* de que el hook retorna, la espera dentro del hook nunca resuelve nada. En ese caso se pasa al plan B.

## Alternativas consideradas
- **Plan B — bloqueo tentativo y reinyección:** el hook bloquea toda tecla que el pad pueda enviar; cuando llega `WM_INPUT`, si no era del pad, se reinyecta con `SendInput`. Funciona sin importar el orden, pero la tecla reinyectada sale marcada como `LLKHF_INJECTED` (algunos juegos y anticheats la ignoran) y suma latencia al numpad del teclado principal.
- **Hook `WH_KEYBOARD` (no LL) en una DLL inyectada:** ahí `WM_INPUT` llega antes que el hook. Pero requiere DLLs de 32 y 64 bits, no entra en procesos elevados ni protegidos y la inyección en todos los procesos genera desconfianza. Descartado.
- **Driver Interception:** resuelve todo, pero exige instalación como admin y reinicio. Va contra el principio 2; queda como modo opcional en la fase 4.

## Consecuencias
- Sin driver, pero con un límite: sin privilegios de admin, el hook no ve las teclas cuando el foco está en una ventana elevada.
- El hook debe responder siempre dentro de `LowLevelHooksTimeout`; si no, Windows lo quita sin avisar. Hace falta detectarlo y reinstalarlo.
- El spike solo imprime en consola (no escribe archivos) y únicamente datos de dispositivo y scancode, para no violar el principio 6.
