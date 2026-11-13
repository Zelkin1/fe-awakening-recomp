//! The console window of the launcher on Windows.
//! The launcher has no console of its own, so this module attaches to the console of the parent process and shows or hides a console on request.

/// Attaches to the console of the parent process when this process has none.
/// Does nothing outside Windows.
pub fn attach() {
    #[cfg(windows)]
    windows::attach();
}

/// Shows or hides the console window.
/// Does nothing outside Windows.
pub fn show(on: bool) {
    #[cfg(windows)]
    windows::show(on);
    #[cfg(not(windows))]
    let _ = on;
}

#[cfg(windows)]
/// The Windows implementation, which calls the console functions of the system directly.
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

    /// The system menu command that closes a window.
    const SC_CLOSE: u32 = 0xF060;
    /// The process id that tells `AttachConsole` to use the console of the parent process.
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;

    /// The state after the console was last asked to hide.
    const HIDDEN: u8 = 1;
    /// The state after the console was last asked to show.
    const SHOWN: u8 = 2;
    /// The last state that was asked for, or 0 when nothing was asked yet.
    static STATE: AtomicU8 = AtomicU8::new(0);

    /// Attaches to the console of the parent process when this process has no console window.
    pub fn attach() {
        unsafe {
            if GetConsoleWindow().is_null() {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }

    /// Shows the console by creating one, or hides it by freeing it.
    /// Asking for the state that is already set does nothing.
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
            // The list holds at most two ids, which is enough to tell whether this process is the only one on the console.
            // A console that another process shares, such as the terminal that started the launcher, is left alone.
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

    /// Removes the close command from the system menu of the window, so closing the console cannot end the launcher.
    unsafe fn disable_close_button(window: *mut c_void) {
        if !window.is_null() {
            unsafe {
                DeleteMenu(GetSystemMenu(window, 0), SC_CLOSE, 0);
            }
        }
    }
}
