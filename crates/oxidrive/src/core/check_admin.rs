pub fn check_admin() -> bool {
    #[cfg(target_os = "macos")]
    return macos::check_admin();

    #[cfg(target_os = "windows")]
    return windows::check_admin();
}

#[cfg(target_os = "macos")]
mod macos {
    use libc;

    pub(crate) fn check_admin() -> bool {
        unsafe { libc::geteuid() == 0 }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    pub(crate) fn check_admin() -> bool {
        todo!()
    }
}
