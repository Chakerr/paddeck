# PadDeck — Plan del proyecto

> Nombre provisional. Verificar disponibilidad en GitHub/dominios antes de publicar.

## 1. Visión

Un "Stream Deck" gratuito y de código abierto que convierte **cualquier pad numérico USB** en una botonera para OBS y atajos del sistema. Descargar, conectar el pad, presionar una tecla y listo.

**Diferenciador:** cualquier persona, cualquier pad numérico, configuración visual en minutos, sin drivers por defecto.

## 2. Principios (no negociables)

1. **Simple primero:** si una función necesita explicación larga, se rediseña.
2. **Cero configuración técnica:** sin editar archivos, sin drivers obligatorios, sin puertos manuales.
3. **Cualquier pad numérico:** detección por "presiona una tecla", no por listas de modelos.
4. **Las teclas del pad no se escapan** a otras aplicaciones mientras la app esté activa.
5. **Código abierto y legible:** fácil de contribuir, bien documentado, en español e inglés.
6. **Seguro:** credenciales en el almacén seguro del SO, sin telemetría por defecto.

## 3. Stack

| Capa | Tecnología |
|---|---|
| App de escritorio | **Tauri 2** |
| Backend | **Rust** (tokio) |
| UI | **React + TypeScript + Vite** |
| Captura Windows | crate `windows` (Raw Input + `WH_KEYBOARD_LL`) |
| OBS | crate `obws` (obs-websocket v5) |
| Emitir atajos | crate `enigo` |
| Credenciales | crate `keyring` |
| Persistencia | JSON con `serde` en el directorio de config del usuario |
| Plugins Tauri | `autostart`, `single-instance`, `updater`, `tray` |

Plataforma objetivo del MVP: **Windows 10/11**. Linux (evdev) y macOS (IOHIDManager) en fases posteriores, por eso la captura va detrás de un trait.

## 4. Arquitectura

```
┌──────────────┐   KeyEvent    ┌──────────────┐   Action    ┌──────────────────┐
│ InputBackend │ ────────────► │ MappingEngine│ ──────────► │ ActionExecutor   │
│ (por SO)     │               │ perfil+capa  │             │ obs / keys / app │
└──────────────┘               └──────────────┘             └──────────────────┘
       ▲                              ▲                              │
  DeviceRegistry                 ProfileStore                    OSD / UI
 (pad registrado)               (JSON + autoswitch)           (eventos Tauri)
```

### Estructura del repositorio

```
paddeck/
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ input/
│  │  │  ├─ mod.rs            # trait InputBackend, KeyEvent
│  │  │  ├─ windows.rs        # Raw Input + hook LL
│  │  │  └─ device.rs         # DeviceId, DeviceRegistry
│  │  ├─ engine/
│  │  │  ├─ mapping.rs        # tecla + capa -> acción
│  │  │  └─ layers.rs         # tecla Fn (mantener)
│  │  ├─ actions/
│  │  │  ├─ mod.rs            # trait Action, registro de acciones
│  │  │  ├─ obs.rs
│  │  │  ├─ hotkey.rs
│  │  │  ├─ media.rs
│  │  │  ├─ launch.rs         # app / URL
│  │  │  ├─ text.rs
│  │  │  └─ multi.rs          # secuencias con delays
│  │  ├─ integrations/obs.rs  # conexión, reconexión, listado de escenas/fuentes
│  │  ├─ profiles/            # carga, guardado, import/export, plantillas
│  │  ├─ osd.rs               # ventana de notificación al presionar
│  │  └─ commands.rs          # comandos Tauri expuestos a la UI
│  └─ tauri.conf.json
├─ src/                        # UI React
│  ├─ pages/ (Onboarding, Editor, Settings)
│  ├─ components/ (NumpadView, KeyEditor, ActionPicker, ObsStatus)
│  └─ i18n/ (es.json, en.json)
├─ templates/                  # perfiles predefinidos
├─ docs/
└─ .github/ (workflows, issue templates)
```

### Modelo de datos (perfil)

```json
{
  "version": 1,
  "name": "Streaming básico",
  "device": { "id": "HID#VID_XXXX&PID_XXXX#...", "label": "Mi pad" },
  "layerKey": "Numpad0",
  "autoSwitch": { "process": "obs64.exe" },
  "keys": {
    "Numpad1": {
      "label": "Cam 1",
      "base":  { "type": "obs.setScene", "scene": "Cámara 1" },
      "layer": { "type": "obs.setPreviewScene", "scene": "Cámara 1" }
    },
    "NumpadEnter": { "label": "Transición", "base": { "type": "obs.transition" } },
    "NumpadAdd":   { "label": "Mic", "base": { "type": "obs.toggleMute", "input": "Mic/Aux" } },
    "NumpadSubtract": {
      "label": "Intro",
      "base": { "type": "multi", "steps": [
        { "type": "obs.setScene", "scene": "Comenzando" },
        { "type": "delay", "ms": 5000 },
        { "type": "obs.setScene", "scene": "Cámara 1" }
      ]}
    }
  }
}
```

Las acciones se identifican por `type` con namespace (`obs.*`, `system.*`, `plugin.<id>.*`) para abrir la puerta a plugins.

## 5. Captura en Windows (riesgo técnico principal)

**Objetivo:** saber qué dispositivo envió cada tecla y **bloquear** solo las del pad registrado.

Enfoque sin driver:

1. Hilo dedicado con una ventana *message-only*.
2. `RegisterRawInputDevices` con `RIDEV_INPUTSINK` para teclados → `WM_INPUT` entrega `hDevice` + scancode/vkey.
3. `SetWindowsHookEx(WH_KEYBOARD_LL)` en el mismo hilo para poder bloquear.
4. Problema: el hook LL se invoca **antes** de que se procese `WM_INPUT`. Solución: dentro del hook, para cada tecla, buscar en un buffer de eventos Raw Input recientes coincidentes (vkey + make/break); si no hay, bombear mensajes (`PeekMessage` filtrando `WM_INPUT`) con timeout corto (≈ 30–50 ms) hasta resolver el dispositivo.
5. Si la tecla viene del pad registrado → devolver `1` (bloquear) y emitir `KeyEvent` al motor. Si no → dejar pasar.
6. Teclas emitidas por la propia app (`enigo`/`SendInput`) se marcan con `dwExtraInfo` para ignorarlas y evitar bucles.
7. Manejar NumLock: mapear por scancode, no por vkey, para que funcione igual con NumLock activo o no.

Modo exclusivo opcional (fase 4): backend alternativo con driver **Interception**. Advertir en la UI que requiere admin + reinicio y que algunos anticheats lo marcan. Revisar su licencia antes de distribuirlo.

**Criterios de aceptación de la Fase 0:**
- Con dos teclados conectados, el numpad registrado no escribe en Bloc de notas y el teclado principal sí, incluido su propio numpad.
- Funciona con NumLock activo e inactivo.
- Latencia añadida al teclado principal imperceptible (< 50 ms en el peor caso, típicamente < 5 ms).
- Sin teclas "pegadas" tras 10 minutos de pulsaciones rápidas alternando ambos teclados.
- Al desconectar y reconectar el pad se vuelve a reconocer.

## 6. Experiencia de usuario

### Onboarding (máximo 3 pasos)
1. **"Conecta tu pad y presiona cualquier tecla"** → se registra el dispositivo.
2. **"Conectar con OBS"** → probar `localhost:4455` automáticamente; si pide contraseña, mostrar dónde encontrarla en OBS (*Herramientas → Configuración del servidor WebSocket*) con captura de pantalla.
3. **"Elige una plantilla"** → *Streaming básico*, *Podcast*, *Clases/Reuniones*, *Empezar en blanco*. Las plantillas leen las escenas reales de OBS y las asignan en orden.

### Editor
- Dibujo del numpad estándar de 17 teclas; al presionar una tecla física se resalta en pantalla.
- Clic en una tecla → panel lateral: etiqueta + selector de acción por categorías (OBS, Teclado, Multimedia, Abrir, Texto, Secuencia). Los campos de OBS son desplegables con escenas/fuentes reales, nunca texto libre.
- Botón **Probar** en cada tecla.
- Pestaña de capa Fn.
- **Exportar etiquetas** → PDF con rótulos del tamaño de keycaps.

### Durante el uso
- Ícono en bandeja: verde (pad + OBS ok), amarillo (pad ok, OBS desconectado), rojo (sin pad).
- OSD discreto al presionar ("Cámara 2 al aire"), desactivable.
- Reconexión automática a OBS.

## 7. Fases

### Fase 0 — Spike de captura (1–2 semanas)
- Binario de consola en Rust que registra un pad y bloquea sus teclas (sección 5).
- Documentar hallazgos en `docs/input-windows.md`.
- **No avanzar** hasta cumplir los criterios de aceptación.

### Fase 1 — Núcleo (2–3 semanas)
- Proyecto Tauri, trait `InputBackend` con la implementación de Windows integrada.
- `MappingEngine`, `ProfileStore` (JSON), capas.
- Acciones: `obs.setScene`, `obs.setPreviewScene`, `obs.transition`, `obs.toggleMute`, `obs.toggleSource`, `obs.toggleRecord`, `obs.toggleStream`, `system.hotkey`, `system.media`, `system.launch`, `system.text`, `multi`, `delay`.
- Integración OBS con reconexión y listado de escenas/fuentes/inputs.
- Tests unitarios del motor y de serialización de perfiles.

### Fase 2 — UI (2–3 semanas)
- Onboarding, editor visual, ajustes, bandeja, OSD.
- i18n español/inglés desde el inicio.
- Plantillas.

### Fase 3 — Distribución (1–2 semanas)
- Instalador MSI/NSIS con Tauri, inicio con Windows, instancia única.
- Auto-updater con GitHub Releases.
- Firma de código (evaluar SignPath para OSS o similar); si no, documentar el aviso de SmartScreen.
- Import/export de perfiles, exportación de etiquetas a PDF.
- **Release v1.0.**

### Fase 4 — Comunidad
- Sistema de plugins (definir API: manifest + acciones; evaluar WASM o procesos externos por seguridad).
- Backend Interception opcional.
- Linux (evdev + `EVIOCGRAB`) y macOS (IOHIDManager *seize*).
- Integraciones sugeridas por la comunidad: ATEM, Spotify, Discord, Teams/Zoom mute.
- Autoswitch de perfil por app activa.
- Soporte de varios pads a la vez.

## 8. Open source

- **Licencia:** GPL-3.0 (garantiza que las versiones derivadas sigan siendo libres). Alternativa: MIT si se prioriza adopción máxima.
- `README.md` (es/en) con GIF de demo, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`.
- Plantillas de issues: bug, feature, **"mi pad no funciona"** (pide VID/PID y modelo) → construye una lista de compatibilidad comunitaria.
- GitHub Actions: lint (`clippy`, `eslint`), tests, build en cada PR; release firmado al crear tag `v*`.
- Conventional Commits + changelog automático.
- Etiqueta `good first issue` para nuevas acciones/traducciones.

## 9. Seguridad y privacidad

- Contraseña de obs-websocket en `keyring` (Credential Manager), nunca en el JSON.
- Sin telemetría. Si en el futuro se agrega, opt-in explícito.
- El hook de teclado solo procesa eventos del dispositivo registrado y **no registra ni almacena** pulsaciones de otros teclados. Documentarlo en el README (las apps con hooks generan desconfianza legítima).
- Plugins (fase 4) con permisos declarados y aislados.

## 10. Instrucciones para Claude Code

1. Crear `CLAUDE.md` en la raíz con: principios (sección 2), stack, estructura y convenciones (Rust: `rustfmt` + `clippy` sin warnings; TS: estricto; comentarios justos).
2. Empezar por la **Fase 0** como crate independiente (`spikes/input-windows`). No crear el proyecto Tauri hasta validar los criterios de aceptación.
3. Trabajar fase por fase, con PRs pequeños y tests para la lógica que no dependa del SO.
4. Ante decisiones de UX, priorizar siempre la opción más simple para un usuario no técnico.
