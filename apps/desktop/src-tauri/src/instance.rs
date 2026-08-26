//! One Soul per signed-in user. AC-01's invisible half.
//!
//! Closing the window puts Soul in the tray rather than ending it, so the
//! natural way to get the window back — clicking Soul in the Start menu again
//! — starts a *second* `soul.exe`. Nothing about that is obvious to the person
//! doing it: they asked for their window back and they got one. What they also
//! got is a second session on `%LOCALAPPDATA%\Soul\soul.db`, which means
//! two SQLCipher write-ahead logs, two collectors, and two in-memory consent
//! ledgers that do not know about each other. Nothing crashes; the two just
//! stop agreeing about what was written. `tests/one_store.rs` keeps the *one
//! process* from opening two stores, and this module is the other half: the
//! second process does not get as far as opening one.
//!
//! The claim is a named mutex, and the name lives in the `Local\` namespace,
//! which Windows scopes to the logon session. Not `Global\`: two people signed
//! into the same machine each have their own `%LOCALAPPDATA%`, so each is
//! entitled to their own Soul, and a machine-wide name would let the first of
//! them keep the second out — for a name that, in some configurations, needs a
//! privilege Soul deliberately does not ask for.
//!
//! There is no lock *file*. A file next to `keys.dpapi` would be a second thing
//! in the data directory to explain, and it would survive a power cut as a
//! stale lock that keeps Soul from starting at all. A kernel object is released
//! when the process holding it dies, however it dies.
//!
//! The window is found by its title rather than by asking the first process
//! anything: there is no IPC between the two, and adding one would be a
//! listening socket or a pipe, which is a bigger thing than the problem.
//! `FindWindowW` finds hidden windows, which is exactly the case that matters —
//! the first Soul is in the tray with its window hidden.
//!
//! Compiled on every platform and only *does* anything on Windows. Linux `run`
//! is not a product surface — no runner has a notification area to put a tray
//! in either — so off Windows every launch is the first one.

/// The identifier `tauri.conf.json` gives the application.
///
/// Kept here as well because the mutex name is built from it and a test can
/// then read both and compare, rather than trusting a comment that says they
/// are the same string.
pub const APP_IDENTIFIER: &str = "local.soul.desktop";

/// Per logon session. See the module note on why not `Global\`.
pub const MUTEX_NAMESPACE: &str = r"Local\";

/// The name the first Soul claims and every later one finds taken.
pub const MUTEX_NAME: &str = r"Local\local.soul.desktop";

/// The title `tauri.conf.json` gives the main window, which is how the second
/// launch finds the first one's window.
pub const WINDOW_TITLE: &str = "Soul";

/// What a launch found when it reached for the name.
#[derive(Debug)]
pub enum Claim {
    /// This process is the Soul for this logon session. The guard has to be
    /// kept alive for as long as that is true.
    Ours(Guard),
    /// Another Soul already holds the name. This process must not open
    /// anything; [`reveal_running_instance`] is the whole of what is left for
    /// it to do.
    AlreadyRunning,
}

/// The live claim. Dropping it releases the name.
///
/// It holds an operating-system handle rather than a flag, so a process that
/// dies without unwinding still releases the name: this is the guard for the
/// tidy path, and the kernel is the guard for the rest.
#[derive(Debug)]
pub struct Guard {
    #[allow(dead_code)]
    mutex: platform::Mutex,
}

/// Claim the name for this process.
///
/// Called before anything opens a store, which is the point of it. A failure
/// to create the mutex that is not "it already exists" is reported as
/// [`Claim::AlreadyRunning`] too: the alternative is to carry on and possibly
/// open a second store on a database another process is writing to, and losing
/// a launch on a broken logon session is the cheaper of the two.
pub fn claim() -> Claim {
    match platform::create_mutex(MUTEX_NAME) {
        platform::Creation::Claimed(mutex) => Claim::Ours(Guard { mutex }),
        platform::Creation::Taken => Claim::AlreadyRunning,
    }
}

/// Ask the Soul that already holds the name to come back to the front.
///
/// Show, unminimize, focus — the same three things the tray menu's 「打开
/// Soul」 does, because it is the same request arriving through a different
/// door. Returns whether a window was found: the first Soul may still be
/// starting and have no window yet, and a launch that cannot find one exits
/// anyway rather than opening a store.
pub fn reveal_running_instance() -> bool {
    platform::reveal_window(WINDOW_TITLE)
}

#[cfg(not(windows))]
use self::elsewhere as platform;
#[cfg(windows)]
use self::win32 as platform;

/// Off Windows there is no notification area to close a window into, so there
/// is nothing for a second launch to hand a window back to and every launch is
/// the first one. The module exists so that the two functions above are one
/// implementation rather than two: Linux CI type-checks the shape the shipped
/// binary uses, which is the same trade `soul-win-dpapi` makes by compiling
/// everywhere and answering `Unsupported`.
#[cfg(not(windows))]
mod elsewhere {
    #[derive(Debug)]
    pub struct Mutex;

    pub enum Creation {
        Claimed(Mutex),
        /// Never produced here; the Windows module is what can find a name
        /// already taken.
        #[allow(dead_code)]
        Taken,
    }

    pub fn create_mutex(_name: &str) -> Creation {
        Creation::Claimed(Mutex)
    }

    pub fn reveal_window(_title: &str) -> bool {
        false
    }
}

/// The Win32 calls, and the whole of this crate's unsafe code.
///
/// Seven declarations rather than a binding crate, the same way
/// `crates/soul-collect/src/windows.rs` and `crates/soul-win-dpapi/src/sys.rs`
/// do it: the shell's dependency list is something a reviewer reads top to
/// bottom, and seven `extern "system"` lines are cheaper to check than another
/// entry on it.
#[cfg(windows)]
mod win32 {
    #![allow(unsafe_code)]

    /// `CreateMutexW` succeeded and this process is the one that made the
    /// name, or it did not and something else has it.
    pub enum Creation {
        Claimed(Mutex),
        Taken,
    }

    /// A handle to the named mutex, closed when it is dropped.
    #[derive(Debug)]
    pub struct Mutex {
        handle: ffi::Handle,
    }

    // The handle is only ever closed, and the type owns it, so it can cross a
    // thread boundary safely. Tauri's builder does not move this one, but a
    // guard that could not be moved would be an odd thing to hand back.
    unsafe impl Send for Mutex {}
    unsafe impl Sync for Mutex {}

    impl Drop for Mutex {
        fn drop(&mut self) {
            // SAFETY: a handle this process created, used nowhere else, and
            // is done with. Closing it is what releases the name.
            unsafe { ffi::CloseHandle(self.handle) };
        }
    }

    /// The mutex existed already, so somebody else made it.
    const ERROR_ALREADY_EXISTS: u32 = 183;

    pub fn create_mutex(name: &str) -> Creation {
        let wide = wide(name);
        // `initial_owner` is false on purpose: nothing here waits on the
        // mutex, and ownership only matters to a waiter. The name existing is
        // the whole signal, which is also why an abandoned mutex — a Soul that
        // was killed rather than closed — is not a state this can get into.
        //
        // SAFETY: `CreateMutexW` reports an existing name by leaving
        // ERROR_ALREADY_EXISTS in the thread last-error even on a successful
        // handle. It does not clear a stale last-error when it creates a new
        // name, so a leftover 183 from process startup would make the first
        // Soul think it was already running and exit without opening a store.
        unsafe { ffi::SetLastError(0) };
        // SAFETY: no security attributes, and `wide` is a nul-terminated
        // UTF-16 buffer that outlives the call.
        let handle = unsafe { ffi::CreateMutexW(std::ptr::null_mut(), 0, wide.as_ptr()) };
        // SAFETY: reads this thread's last-error value, which is meaningful
        // because nothing has run since the two calls above.
        let error = unsafe { ffi::GetLastError() };

        if handle.is_null() {
            // Access denied is the interesting one: a Soul running at another
            // integrity level in this session made the name and this process
            // may not open it. Every other failure is a session with no
            // handles left. Both are answered the same way — see `claim`.
            return Creation::Taken;
        }

        let mutex = Mutex { handle };
        if error == ERROR_ALREADY_EXISTS {
            // The handle is real and has to be closed; `mutex` does that on
            // the way out of this branch.
            Creation::Taken
        } else {
            Creation::Claimed(mutex)
        }
    }

    /// `SW_SHOW`: display a window that is hidden, which is where a Soul that
    /// was closed into the tray keeps its window.
    const SW_SHOW: i32 = 5;
    /// `SW_RESTORE`: and un-minimize it if that is what happened to it
    /// instead.
    const SW_RESTORE: i32 = 9;

    pub fn reveal_window(title: &str) -> bool {
        let wide = wide(title);
        // A null class name matches any class. Tauri's window class is
        // generated, so the title is what there is to go on; the worst case is
        // that some other program's window is titled `Soul` and gets raised,
        // which costs the user one glance and no data.
        //
        // SAFETY: the buffer is nul-terminated UTF-16 and outlives the call,
        // and a window that is not there is reported as null rather than as a
        // failure.
        let window = unsafe { ffi::FindWindowW(std::ptr::null(), wide.as_ptr()) };
        if window.is_null() {
            return false;
        }

        // SAFETY: `window` is a handle Windows just returned. All three calls
        // are the documented way to bring a window forward and none of them
        // takes a pointer. A window that closed in between is reported by the
        // return value, which is why nothing here is checked: this is a best
        // effort at raising a window, and there is no recovery.
        unsafe {
            ffi::ShowWindow(window, SW_SHOW);
            ffi::ShowWindow(window, SW_RESTORE);
            ffi::SetForegroundWindow(window);
        }
        true
    }

    /// A nul-terminated UTF-16 copy, which is what the `W` entry points take.
    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    #[allow(non_snake_case)]
    mod ffi {
        use std::ffi::c_void;

        /// `HANDLE` and `HWND` are both opaque pointer-sized handles.
        pub type Handle = *mut c_void;

        #[link(name = "kernel32")]
        extern "system" {
            pub fn CreateMutexW(
                attributes: *mut c_void,
                initial_owner: i32,
                name: *const u16,
            ) -> Handle;
            pub fn CloseHandle(object: Handle) -> i32;
            pub fn GetLastError() -> u32;
            pub fn SetLastError(error: u32);
        }

        #[link(name = "user32")]
        extern "system" {
            pub fn FindWindowW(class_name: *const u16, window_name: *const u16) -> Handle;
            pub fn ShowWindow(window: Handle, command: i32) -> i32;
            pub fn SetForegroundWindow(window: Handle) -> i32;
        }
    }
}

/// The one part of the claim that has to be run rather than read.
///
/// `tests/shell_is_local_only.rs` asserts the *shape* of this file off its own
/// source — the name is `Local\`, the last-error is cleared before the create,
/// nothing touches a file — because a Linux runner has no `CreateMutexW` to
/// call and the `elsewhere` stub answers `Claimed` to everything. So the
/// branch that actually matters, the one a second launch takes, has only ever
/// been type-checked. These three run it on Windows.
///
/// Nothing here calls [`claim`] or names [`MUTEX_NAME`]. That name belongs to
/// whatever Soul is running on the machine the test is running on, and taking
/// it would stop the author's Soul from starting — the same reason `lib.rs`
/// keeps the claim out of `configure`. Each test builds its own name from the
/// process id and a label instead, so neither two tests in this binary nor two
/// binaries running at once can land on the same one. Still `Local\`: a test
/// has even less business in the machine-wide namespace than the product does.
///
/// Placed after `mod win32` deliberately: the test in `shell_is_local_only.rs`
/// that keeps every `unsafe` inside the Win32 block decides what is inside it
/// by source position, so anything added above that module reads as outside.
#[cfg(all(test, windows))]
mod runs_on_windows {
    use super::win32::{create_mutex, Creation};

    /// A name in this logon session that nothing else has a reason to hold.
    fn test_name(label: &str) -> String {
        format!(r"Local\soul-instance-test-{}-{label}", std::process::id())
    }

    /// The first Soul's answer: the name was free and this process made it.
    #[test]
    fn the_first_create_claims_the_name() {
        let claimed = create_mutex(&test_name("first"));
        assert!(
            matches!(claimed, Creation::Claimed(_)),
            "a name nothing holds came back taken",
        );
    }

    /// The second launch's answer, which is the whole point of the module:
    /// while the first handle is alive, the same name is reported taken.
    #[test]
    fn a_second_create_of_a_held_name_is_taken() {
        let name = test_name("held");
        let held = create_mutex(&name);
        assert!(
            matches!(held, Creation::Claimed(_)),
            "the first create did not claim the name, so the second proves nothing",
        );

        assert!(
            matches!(create_mutex(&name), Creation::Taken),
            "a name this process is still holding came back free",
        );

        drop(held);
    }

    /// And the release is the kernel's, not a file's: once the last handle is
    /// closed the name is free again with nothing left behind to clean up.
    /// This is what makes a Soul that was killed rather than closed harmless.
    #[test]
    fn the_name_is_free_again_once_the_handle_is_dropped() {
        let name = test_name("dropped");
        let first = create_mutex(&name);
        assert!(
            matches!(first, Creation::Claimed(_)),
            "the first create did not claim the name",
        );
        drop(first);

        assert!(
            matches!(create_mutex(&name), Creation::Claimed(_)),
            "the name stayed taken after its only handle closed, which is a stale lock",
        );
    }
}
