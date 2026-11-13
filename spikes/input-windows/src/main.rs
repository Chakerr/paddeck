//! Phase 0, step 2: can a WH_KEYBOARD_LL hook learn the source device of a key
//! by waiting for its WM_INPUT? Run 1 (50 ms wait): never, 136/136 timeouts.
//! Run 2 (no wait): WM_INPUT arrives ~0.4 ms after the hook returns.
//! Run 3: does WM_INPUT still arrive when the hook blocks the key? Blocks numpad keys only. Key identities are never
//! printed: only timing, direction and the device VID (see ADR 0002).

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::mem::size_of;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER,
    RID_INPUT, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDI_DEVICENAME, RIM_TYPEKEYBOARD,
    RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HC_ACTION,
    HWND_MESSAGE, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLKHF_UP, MSG, PM_REMOVE,
    PeekMessageW, RegisterClassW, SetWindowsHookExW, WH_KEYBOARD_LL, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_INPUT, WM_INPUT_DEVICE_CHANGE, WNDCLASSW,
};
use windows::core::w;

const RI_KEY_BREAK: u16 = 1;
const RI_KEY_E0: u16 = 2;
const GIDC_ARRIVAL: usize = 1;
/// Max time the hook waits for the matching WM_INPUT.
const WAIT_LIMIT: Duration = Duration::ZERO;
/// Raw events older than this are stale and never matched.
const RAW_MAX_AGE: Duration = Duration::from_millis(200);

/// A hook call still waiting for its WM_INPUT: scan, e0, up, when, blocked.
type PendingKey = (u16, bool, bool, Instant, bool);

/// One keyboard Raw Input event, reduced to what the hook can match on.
struct RawKey {
    scan: u16,
    e0: bool,
    up: bool,
    device: String,
    at: Instant,
}

thread_local! {
    // hDevice -> interface path, so each path is resolved once.
    static DEVICE_NAMES: RefCell<HashMap<usize, String>> = RefCell::new(HashMap::new());
    static RAW_KEYS: RefCell<VecDeque<RawKey>> = const { RefCell::new(VecDeque::new()) };
    // Keys the hook could not resolve, waiting for their late WM_INPUT.
    static PENDING: RefCell<VecDeque<PendingKey>> = const { RefCell::new(VecDeque::new()) };
}

fn main() -> Result<()> {
    let hwnd = create_message_window()?;
    register_keyboards(hwnd)?;
    install_hook()?;
    println!("Listening. Press single keys on each keyboard (Ctrl+C to quit).");
    println!("{:<5} {:<9} {:<10} wait", "dir", "device", "result");
    run_message_loop();
    Ok(())
}

fn create_message_window() -> Result<HWND> {
    // SAFETY: plain Win32 calls; the class name is a static wide string and
    // `wndproc` matches the WNDPROC signature.
    unsafe {
        let instance = GetModuleHandleW(None).context("GetModuleHandleW")?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: w!("PadDeckSpike"),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            anyhow::bail!("RegisterClassW failed");
        }
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("PadDeckSpike"),
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )
        .context("CreateWindowExW")
    }
}

fn register_keyboards(hwnd: HWND) -> Result<()> {
    // INPUTSINK: receive input even without focus. DEVNOTIFY: plug/unplug events.
    let device = RAWINPUTDEVICE {
        usUsagePage: 0x01, // Generic Desktop
        usUsage: 0x06,     // Keyboard
        dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
        hwndTarget: hwnd,
    };
    // SAFETY: the slice holds one valid RAWINPUTDEVICE and cbsize matches it.
    unsafe { RegisterRawInputDevices(&[device], size_of::<RAWINPUTDEVICE>() as u32) }
        .context("RegisterRawInputDevices")
}

fn install_hook() -> Result<()> {
    // The hook runs on this thread, inside the message loop below.
    // SAFETY: `hook_proc` matches HOOKPROC; the hook lives until the process exits.
    unsafe {
        let instance = GetModuleHandleW(None).context("GetModuleHandleW")?;
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), Some(instance.into()), 0)
            .context("SetWindowsHookExW")?;
    }
    Ok(())
}

fn run_message_loop() {
    let mut msg = MSG::default();
    // SAFETY: `msg` is a valid out-pointer for the whole loop.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_INPUT => on_input(lparam),
        WM_INPUT_DEVICE_CHANGE => {
            let event = if wparam.0 == GIDC_ARRIVAL {
                "connected"
            } else {
                "removed"
            };
            println!(
                "-- device {event}: {}",
                device_name(HANDLE(lparam.0 as *mut c_void))
            );
        }
        _ => {}
    }
    // WM_INPUT must still reach DefWindowProc so the system can clean up.
    // SAFETY: forwarding the unmodified message to the default handler.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for HC_ACTION, lparam points to a KBDLLHOOKSTRUCT owned by the system.
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if report(info) {
            return LRESULT(1);
        }
    }
    // SAFETY: passing the untouched hook arguments down the chain.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Logs the hook call; returns true when the key must be blocked.
fn report(info: &KBDLLHOOKSTRUCT) -> bool {
    let up = info.flags.0 & LLKHF_UP.0 != 0;
    let dir = if up { "up" } else { "down" };
    if info.flags.0 & LLKHF_INJECTED.0 != 0 {
        println!("{dir:<5} {:<9} injected", "-");
        return false;
    }
    let scan = info.scanCode as u16;
    let e0 = info.flags.0 & LLKHF_EXTENDED.0 != 0;
    // Numpad digits, dot, + and - (NumLock off or on); E0 variants are the nav cluster.
    let block = !e0 && (0x47..=0x53).contains(&scan);
    let start = Instant::now();
    let (device, result) = match take_raw(scan, e0, up) {
        Some(device) => (device, "buffered"),
        None => match wait_for_raw(scan, e0, up) {
            Some(device) => (device, "waited"),
            None => {
                PENDING.with_borrow_mut(|p| p.push_back((scan, e0, up, start, block)));
                ("?".into(), "TIMEOUT")
            }
        },
    };
    println!(
        "{dir:<5} {:<9} {result:<10} {:.2} ms",
        vid(&device),
        start.elapsed().as_secs_f64() * 1000.0
    );
    block
}

/// Pumps only WM_INPUT until the matching raw event shows up or WAIT_LIMIT passes.
// ponytail: busy-waits for up to 50 ms per key; switch to MsgWaitForMultipleObjects if plan A wins.
fn wait_for_raw(scan: u16, e0: bool, up: bool) -> Option<String> {
    let deadline = Instant::now() + WAIT_LIMIT;
    let mut msg = MSG::default();
    while Instant::now() < deadline {
        // SAFETY: `msg` is a valid out-pointer; the message is dispatched to our wndproc.
        unsafe {
            if PeekMessageW(&mut msg, None, WM_INPUT, WM_INPUT, PM_REMOVE).as_bool() {
                DispatchMessageW(&msg);
            }
        }
        if let Some(device) = take_raw(scan, e0, up) {
            return Some(device);
        }
    }
    None
}

/// Removes and returns the device of the oldest fresh raw event matching the key.
fn take_raw(scan: u16, e0: bool, up: bool) -> Option<String> {
    RAW_KEYS.with_borrow_mut(|keys| {
        keys.retain(|k| k.at.elapsed() < RAW_MAX_AGE);
        let i = keys
            .iter()
            .position(|k| k.scan == scan && k.e0 == e0 && k.up == up)?;
        keys.remove(i).map(|k| k.device)
    })
}

fn on_input(lparam: LPARAM) {
    let mut raw = RAWINPUT::default();
    let mut size = size_of::<RAWINPUT>() as u32;
    // SAFETY: `raw` is large enough for keyboard input and `size` reports its size.
    let read = unsafe {
        GetRawInputData(
            HRAWINPUT(lparam.0 as *mut c_void),
            RID_INPUT,
            Some(&mut raw as *mut RAWINPUT as *mut c_void),
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    if read == u32::MAX || raw.header.dwType != RIM_TYPEKEYBOARD.0 {
        return;
    }
    // SAFETY: dwType says the union holds keyboard data.
    let kb = unsafe { raw.data.keyboard };
    let (scan, e0, up) = (
        kb.MakeCode,
        kb.Flags & RI_KEY_E0 != 0,
        kb.Flags & RI_KEY_BREAK != 0,
    );
    let hooked_at = PENDING.with_borrow_mut(|p| {
        p.retain(|k| k.3.elapsed() < RAW_MAX_AGE);
        let i = p.iter().position(|k| (k.0, k.1, k.2) == (scan, e0, up))?;
        p.remove(i).map(|k| (k.3, k.4))
    });
    if let Some((at, blocked)) = hooked_at {
        let dir = if up { "up" } else { "down" };
        let device = device_name(raw.header.hDevice);
        println!(
            "{dir:<5} {:<9} {:<10} {:.2} ms",
            vid(&device),
            if blocked { "raw-blocked" } else { "raw-late" },
            at.elapsed().as_secs_f64() * 1000.0
        );
        // Already paired with its hook call; buffering it could mismatch a later key.
        return;
    }
    let key = RawKey {
        scan: kb.MakeCode,
        e0: kb.Flags & RI_KEY_E0 != 0,
        up: kb.Flags & RI_KEY_BREAK != 0,
        device: device_name(raw.header.hDevice),
        at: Instant::now(),
    };
    RAW_KEYS.with_borrow_mut(|keys| keys.push_back(key));
}

/// Short device label for logs, e.g. "VID_17EF".
fn vid(path: &str) -> &str {
    path.find("VID_")
        .and_then(|i| path.get(i..i + 8))
        .unwrap_or(path)
}

fn device_name(device: HANDLE) -> String {
    // Injected input (SendInput) arrives with a null device.
    if device.is_invalid() {
        return "<injected/unknown>".into();
    }
    DEVICE_NAMES.with_borrow_mut(|names| {
        names
            .entry(device.0 as usize)
            .or_insert_with(|| query_device_name(device))
            .clone()
    })
}

fn query_device_name(device: HANDLE) -> String {
    let mut len = 0u32;
    // SAFETY: first call only asks for the length (in u16 chars); second call
    // writes at most `len` chars into a buffer of that size.
    unsafe {
        GetRawInputDeviceInfoW(Some(device), RIDI_DEVICENAME, None, &mut len);
        let mut buf = vec![0u16; len as usize];
        let written = GetRawInputDeviceInfoW(
            Some(device),
            RIDI_DEVICENAME,
            Some(buf.as_mut_ptr() as *mut c_void),
            &mut len,
        );
        if written == u32::MAX {
            return format!("<handle {:?}>", device.0);
        }
        String::from_utf16_lossy(&buf[..written as usize])
            .trim_end_matches('\0')
            .into()
    }
}
