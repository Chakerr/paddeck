//! Phase 0, step 1: log every keyboard Raw Input event with its source device.
//! Console only, nothing is written to disk (see ADR 0002).

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::size_of;

use anyhow::{Context, Result};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER,
    RID_INPUT, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDI_DEVICENAME, RIM_TYPEKEYBOARD,
    RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG,
    RegisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_INPUT, WM_INPUT_DEVICE_CHANGE, WNDCLASSW,
};
use windows::core::w;

const RI_KEY_BREAK: u16 = 1;
const RI_KEY_E0: u16 = 2;
const GIDC_ARRIVAL: usize = 1;

thread_local! {
    // hDevice -> interface path, so each path is resolved once.
    static DEVICE_NAMES: RefCell<HashMap<usize, String>> = RefCell::new(HashMap::new());
}

fn main() -> Result<()> {
    let hwnd = create_message_window()?;
    register_keyboards(hwnd)?;
    println!("Listening. Press keys on each keyboard (Ctrl+C to quit).");
    println!("{:<8} {:<6} {:<4} {:<6} device", "scan", "e0", "up", "vkey");
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
    println!(
        "{:<8} {:<6} {:<4} {:<6} {}",
        format!("0x{:02X}", kb.MakeCode),
        kb.Flags & RI_KEY_E0 != 0,
        kb.Flags & RI_KEY_BREAK != 0,
        format!("0x{:02X}", kb.VKey),
        device_name(raw.header.hDevice),
    );
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
