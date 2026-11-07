pub fn attach() {
    #[cfg(windows)]
    windows::attach();
}

pub fn show(on: bool) {
    #[cfg(windows)]
    windows::show(on);
    #[cfg(not(windows))]
    let _ = on;
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicU8, Ordering};

    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleWindow() -> *mut c_void;
        fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
        fn AllocConsole() -> i32;
        fn AttachConsole(process: u32) -> i32;
        fn FreeConsole() -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetSystemMenu(window: *mut c_void, revert: i32) -> *mut c_void;
        fn DeleteMenu(menu: *mut c_void, position: u32, flags: u32) -> i32;
    }

    const SC_CLOSE: u32 = 0xF060;
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;

    const HIDDEN: u8 = 1;
    const SHOWN: u8 = 2;
    static STATE: AtomicU8 = AtomicU8::new(0);

    pub fn attach() {
        unsafe {
            if GetConsoleWindow().is_null() {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }

    pub fn show(on: bool) {
        let wanted = if on { SHOWN } else { HIDDEN };
        if STATE.swap(wanted, Ordering::Relaxed) == wanted {
            return;
        }
        unsafe {
            let window = GetConsoleWindow();
            if window.is_null() {
                if on && AllocConsole() != 0 {
                    disable_close_button(GetConsoleWindow());
                }
                return;
            }
            let mut processes = [0u32; 2];
            let shared_with_other_processes = GetConsoleProcessList(processes.as_mut_ptr(), 2) != 1;
            if shared_with_other_processes {
                return;
            }
            if on {
                disable_close_button(window);
            } else {
                FreeConsole();
            }
        }
    }

    unsafe fn disable_close_button(window: *mut c_void) {
        if !window.is_null() {
            unsafe {
                DeleteMenu(GetSystemMenu(window, 0), SC_CLOSE, 0);
            }
        }
    }
}
