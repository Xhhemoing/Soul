//! The shipped bridge and native state machine against synthetic SQLCipher
//! Sessions. The fake OS does not establish Windows/Tauri platform behavior.
// This test target exercises FakeOs paths without starting the native owner.
// Keep the full production modules compiled, including their unused entry points.
#[allow(dead_code)]
#[path = "../../../apps/desktop/src-tauri/src/notification_runtime/mod.rs"]
mod notification_runtime;
#[allow(dead_code)]
#[path = "../../../apps/desktop/src-tauri/src/notification_transport/mod.rs"]
mod notification_transport;
