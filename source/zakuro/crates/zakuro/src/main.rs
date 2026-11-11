// Builds the launcher as a Windows program without a console. The module console.rs attaches or creates one when needed.
#![windows_subsystem = "windows"]

fn main() {
    zakuro::run(None);
}
