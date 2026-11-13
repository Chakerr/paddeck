---
description: Genera la checklist de pruebas manuales de la captura de teclado
---

Genera una checklist de pruebas manuales para el estado actual del spike/backend de captura en Windows, basada en los criterios de aceptación de `PLAN.md` §5 y en lo implementado hasta ahora.

Para cada prueba incluye: preparación (qué teclados conectar, qué app abrir, estado de NumLock), pasos exactos, resultado esperado y qué copiar del log si falla.

Incluye al menos: aislamiento entre teclados (incluido el numpad del teclado principal), NumLock activo/inactivo, pulsaciones rápidas alternadas, mantener teclas presionadas, desconexión/reconexión del pad, comportamiento con la app de consola sin foco y latencia percibida en el teclado principal.

Al final deja una tabla vacía de resultados (prueba | resultado | notas) para que yo la llene y te la devuelva. Cuando la devuelva, actualiza `docs/input-windows.md` con los resultados.
