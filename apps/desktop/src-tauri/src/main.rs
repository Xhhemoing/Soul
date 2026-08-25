// A release build has no console window behind it; a debug build keeps one so
// panics are readable while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    soul_desktop::run();
}
