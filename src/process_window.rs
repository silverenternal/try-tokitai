//! Platform-specific child-process window behavior for desktop-safe command execution.

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub trait CommandWindowExt {
    fn hide_window(&mut self) -> &mut Self;
}

impl CommandWindowExt for std::process::Command {
    fn hide_window(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(CREATE_NO_WINDOW);
        }
        self
    }
}

impl CommandWindowExt for tokio::process::Command {
    fn hide_window(&mut self) -> &mut Self {
        #[cfg(windows)]
        self.creation_flags(CREATE_NO_WINDOW);
        self
    }
}
