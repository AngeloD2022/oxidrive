use crate::installer::os_actions::BackendError::OperationError;
use crate::installer::pim::RegistryCommand;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("{0}")]
    UnsupportedAction(String),
    #[error("{0}")]
    OperationError(String),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

pub type BackendResult<T> = Result<T, BackendError>;

/// Backend for carrying out installation tasks.
pub trait InstallActionBackend {
    fn create_file(
        &self,
        path: &Path,
        content: &[u8],
        unix_mode: Option<u32>,
    ) -> BackendResult<()> {
        println!(
            "Creating file: {} ({} bytes)",
            path.display(),
            content.len()
        );

        // Skip if the path already exists and is a directory
        if path.exists() && path.is_dir() {
            return Ok(());
        }

        // If path ends with '/', it's a directory - just create it
        if path.to_str().map(|s| s.ends_with('/')).unwrap_or(false) {
            fs::create_dir_all(path)?;
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(path)?;
        file.write_all(content)?;

        #[cfg(unix)]
        if let Some(mode) = unix_mode {
            use std::os::unix::fs::PermissionsExt;

            let perms = fs::Permissions::from_mode(mode);
            fs::set_permissions(path, perms)?;
        }

        Ok(())
    }

    fn create_dir(&self, path: &Path) -> BackendResult<()> {
        fs::create_dir_all(path)?;

        Ok(())
    }

    fn delete_file(&self, path: &Path, content: &[u8]) -> BackendResult<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }

        Ok(())
    }

    fn run_program(&self, path: &Path, args: Option<&[String]>) -> BackendResult<i32> {
        println!("Running program: {} with args {:?}", path.display(), args);

        let program = Command::new(path)
            .args(args.unwrap_or(&Vec::new()))
            .status()
            .map_err(|e| OperationError(format!("Failed to run {}: {}", path.display(), e)))?;

        let code = program.code().ok_or(OperationError(format!(
            "Running of {} did not exit normally!",
            path.display()
        )))?;

        Ok(code)
    }

    fn create_shortcut(&self, at_path: &Path, to_path: &Path) -> BackendResult<()>;

    fn create_registry(&self, registry: &RegistryCommand) -> BackendResult<()>;
    // fn delete_registry(&self, registy: RegistryCommand) -> BackendResult<()>;

    fn apply_folder_icon(&self, folder_path: &Path, icon_path: &Path) -> BackendResult<()>;
    fn register_application(&self, path: &Path) -> BackendResult<()>;

    fn set_permission(&self, path: &str, value: &str, user: Option<&str>) -> BackendResult<()>;

    // fn install_user_pref(&self, user_pref: InstallUserPrefCommand) -> BackendResult<()>;
    // fn create_shortcut(&self, shortcut: ShortcutCommand) -> BackendResult<()>;
}

pub fn system_default_backend() -> Box<dyn InstallActionBackend> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOSBackend {});

    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsBackend {});
}

pub struct DebugBackend;

impl InstallActionBackend for DebugBackend {
    fn create_file(
        &self,
        path: &Path,
        content: &[u8],
        unix_mode: Option<u32>,
    ) -> BackendResult<()> {
        println!(
            "Would create file: {} ({} bytes)",
            path.display(),
            content.len()
        );
        Ok(())
    }

    fn delete_file(&self, path: &Path, content: &[u8]) -> BackendResult<()> {
        println!("Would delete file: {}", path.display());
        Ok(())
    }

    fn create_dir(&self, path: &Path) -> BackendResult<()> {
        println!("Would create dir: {}", path.display());
        Ok(())
    }

    fn create_shortcut(&self, at_path: &Path, to_path: &Path) -> BackendResult<()> {
        println!(
            "Would create shortcut at {}, to {}",
            at_path.display(),
            to_path.display()
        );
        Ok(())
    }

    fn create_registry(&self, registry: &RegistryCommand) -> BackendResult<()> {
        println!("Would create registry entry: {:?}", registry);
        Ok(())
    }

    fn apply_folder_icon(&self, folder_path: &Path, icon_path: &Path) -> BackendResult<()> {
        println!(
            "Would apply icon '{}' to folder '{}'",
            icon_path.display(),
            folder_path.display()
        );
        Ok(())
    }

    fn register_application(&self, path: &Path) -> BackendResult<()> {
        println!("Would register application: {}", path.display());
        Ok(())
    }

    fn set_permission(&self, path: &str, value: &str, user: Option<&str>) -> BackendResult<()> {
        println!(
            "Would apply permission {} to path {} for user {:?}",
            value, path, user
        );
        Ok(())
    }

    fn run_program(&self, path: &Path, args: Option<&[String]>) -> BackendResult<i32> {
        println!(
            "Would run program at {} with args {:?}",
            path.display(),
            args
        );
        Ok(0)
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use crate::installer::os_actions::BackendError::{OperationError, UnsupportedAction};
    use crate::installer::os_actions::{BackendResult, InstallActionBackend};
    use crate::installer::pim::RegistryCommand;
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    use core_foundation::url::CFURL;
    use core_foundation_sys::base::kCFAllocatorDefault;
    use core_foundation_sys::url::{CFURLCreateWithFileSystemPath, kCFURLPOSIXPathStyle};
    use objc2::AllocAnyThread;
    use objc2_app_kit::{NSImage, NSWorkspace, NSWorkspaceIconCreationOptions};
    use objc2_foundation::NSString;
    use std::fs;
    use std::path::Path;

    #[link(name = "CoreServices", kind = "framework")]
    unsafe extern "C" {
        // OSStatus LSRegisterURL(CFURLRef inURL, Boolean inUpdate);
        fn LSRegisterURL(url: *const CFURL, in_update: u8) -> i32;
    }

    pub struct MacOSBackend;
    impl InstallActionBackend for MacOSBackend {
        fn create_shortcut(&self, at_path: &Path, to_path: &Path) -> BackendResult<()> {
            println!(
                "Creating shortcut at {}, to {}",
                at_path.display(),
                to_path.display()
            );

            if let Some(parent) = at_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }

            std::os::unix::fs::symlink(&to_path, &at_path).map_err(|e| {
                OperationError(format!(
                    "could not create shortcut at {} to {}: {}",
                    at_path.display(),
                    to_path.display(),
                    e
                ))
            })
        }

        fn create_registry(&self, registy: &RegistryCommand) -> BackendResult<()> {
            Err(UnsupportedAction(
                "Registry actions are not supported on this OS.".to_string(),
            ))
        }

        fn set_permission(&self, path: &str, value: &str, user: Option<&str>) -> BackendResult<()> {
            todo!("This seems unimportant.");
        }

        fn apply_folder_icon(&self, folder_path: &Path, icon_path: &Path) -> BackendResult<()> {
            println!(
                "Would apply icon '{}' to folder '{}'",
                icon_path.display(),
                folder_path.display()
            );

            let icon_path_ns = NSString::from_str(&icon_path.to_string_lossy());
            let folder_path_ns = NSString::from_str(&folder_path.to_string_lossy());

            let image = NSImage::initWithContentsOfFile(NSImage::alloc(), &icon_path_ns).ok_or(
                OperationError(format!(
                    "Couldn't create NSImage from {}",
                    folder_path.to_string_lossy()
                )),
            )?;

            let workspace = NSWorkspace::sharedWorkspace();
            let result = workspace.setIcon_forFile_options(
                Some(&image),
                &folder_path_ns,
                NSWorkspaceIconCreationOptions::empty(),
            );

            if !result {
                Err(OperationError(format!(
                    "Could not set the icon for the path: {}",
                    folder_path.to_string_lossy()
                )))
            } else {
                Ok(())
            }
        }

        fn register_application(&self, path: &Path) -> BackendResult<()> {
            // HDPIM.dylib @ FileSystemUtilsMac::RegisterApplication

            println!("Registering application: {}", path.display());

            if !path.is_dir() {
                return Err(OperationError(
                    "The supplied path is not a bundle.".to_string(),
                ));
            }

            let cfstr = CFString::new(&path.to_string_lossy());

            let cfurl = unsafe {
                CFURLCreateWithFileSystemPath(
                    kCFAllocatorDefault,
                    cfstr.as_concrete_TypeRef(),
                    kCFURLPOSIXPathStyle,
                    true as _,
                )
            };

            if cfurl.is_null() {
                return Err(OperationError("Failed to create a CFURL.".to_string()));
            }

            let status = unsafe { LSRegisterURL(cfurl as _, 1) };
            // todo: implement the remainder of this routine.

            if status != 0 {
                return Err(OperationError("Failed to register the path.".to_string()));
            }

            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use crate::installer::os_actions::BackendError::{OperationError, UnsupportedAction};
    use crate::installer::os_actions::{BackendResult, InstallActionBackend};
    use crate::installer::pim::RegistryCommand;
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_SYSTEM, GetFileAttributesW,
        INVALID_FILE_ATTRIBUTES, SetFileAttributesW,
    };
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, IPersistFile,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::Win32::UI::WindowsAndMessaging::{
        IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadImageW,
    };
    use windows::core::{Interface, PCWSTR};
    use winreg::RegKey;
    use winreg::enums::{
        HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS,
    };

    // Yeah yeah, I get it. I'll figure out a proper placement for this
    fn path_to_pcwstr(path: &Path) -> Vec<u16> {
        // NUL-terminated UTF-16 for Win32 APIs
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    pub struct WindowsBackend;

    impl InstallActionBackend for WindowsBackend {
        fn create_shortcut(&self, at_path: &Path, to_path: &Path) -> BackendResult<()> {
            println!(
                "Creating shortcut at {}, to {}",
                at_path.display(),
                to_path.display()
            );

            if let Some(parent) = at_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }

            let link_path: PathBuf = match at_path.extension().and_then(|e| e.to_str()) {
                Some(ext) if ext.eq_ignore_ascii_case("lnk") => at_path.to_path_buf(),
                _ => at_path.with_extension("lnk"),
            };

            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                    .ok()
                    .map_err(|e| {
                        OperationError(format!("COM init (CoInitializeEx) failed: {}", e))
                    })?;
            }
            struct ComGuard;
            impl Drop for ComGuard {
                fn drop(&mut self) {
                    unsafe { CoUninitialize() }
                }
            }
            let _guard = ComGuard;

            let shell_link: IShellLinkW = unsafe {
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(|e| {
                    OperationError(format!("CoCreateInstance(ShellLink) failed: {}", e))
                })?
            };

            let target_w = path_to_pcwstr(to_path);
            unsafe {
                shell_link
                    .SetPath(PCWSTR(target_w.as_ptr()))
                    .map_err(|e| OperationError(format!("IShellLinkW::SetPath failed: {}", e)))?;
            }

            if let Some(parent) = to_path.parent() {
                let parent_w = path_to_pcwstr(parent);
                unsafe {
                    shell_link
                        .SetWorkingDirectory(PCWSTR(parent_w.as_ptr()))
                        .map_err(|e| {
                            OperationError(format!(
                                "IShellLinkW::SetWorkingDirectory failed: {}",
                                e
                            ))
                        })?;
                }
            }

            let persist: IPersistFile = shell_link.cast().map_err(|e| {
                OperationError(format!("Failed to cast IShellLinkW -> IPersistFile: {}", e))
            })?;

            let link_w = path_to_pcwstr(&link_path);
            unsafe {
                persist.Save(PCWSTR(link_w.as_ptr()), true).map_err(|e| {
                    OperationError(format!(
                        "IPersistFile::Save failed for {}: {}",
                        link_path.display(),
                        e
                    ))
                })?;
            }

            Ok(())
        }

        fn create_registry(&self, registry: &RegistryCommand) -> BackendResult<()> {
            println!("{registry:?}");
            let path = registry.path.replace('/', "\\");
            let root_part = path.split_once('\\').map(|x| x.0).unwrap_or(&path);

            let root = match root_part {
                "HKEY_CLASSES_ROOT" => HKEY_CLASSES_ROOT,
                "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
                "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
                "HKEY_USERS" => HKEY_USERS,
                "HKEY_CURRENT_CONFIG" => HKEY_CURRENT_CONFIG,
                _ => {
                    return Err(OperationError(format!(
                        "Unknown registry root in path: {path}"
                    )));
                }
            };
            let subkey_path = path
                .strip_prefix(&format!("{root_part}\\"))
                .unwrap_or(&path);

            let root = RegKey::predef(root);

            if registry.is_recursive_delete {
                root.delete_subkey_all(subkey_path).map_err(|e| {
                    OperationError(format!(
                        "Failed to delete registry tree '{}': {}",
                        subkey_path, e
                    ))
                })?;
                return Ok(());
            }

            let (key, _disp) = root.create_subkey(subkey_path).map_err(|e| {
                OperationError(format!(
                    "Failed to create/open key '{}': {}",
                    subkey_path, e
                ))
            })?;

            if let (Some(name), Some(value)) = (&registry.name, &registry.value) {
                let type_str = registry.type_.as_deref().unwrap_or("REG_SZ");
                match type_str.to_ascii_uppercase().as_str() {
                    "REG_DWORD" => {
                        let parsed: u32 = if value.starts_with("0x") || value.starts_with("0X") {
                            u32::from_str_radix(&value[2..], 16).map_err(|e| {
                                OperationError(format!(
                                    "Invalid REG_DWORD value '{}': {}",
                                    value, e
                                ))
                            })?
                        } else {
                            value.parse::<u32>().map_err(|e| {
                                OperationError(format!(
                                    "Invalid REG_DWORD value '{}': {}",
                                    value, e
                                ))
                            })?
                        };
                        key.set_value(name, &parsed).map_err(|e| {
                            OperationError(format!("Failed to set REG_DWORD '{}': {}", name, e))
                        })?;
                    }
                    _ => {
                        key.set_value(name, &value.as_str()).map_err(|e| {
                            OperationError(format!("Failed to set REG_SZ '{}': {}", name, e))
                        })?;
                    }
                }
            }

            Ok(())
        }

        fn apply_folder_icon(&self, folder_path: &Path, icon_path: &Path) -> BackendResult<()> {
            println!(
                "Applying icon '{}' to folder '{}'",
                icon_path.display(),
                folder_path.display()
            );

            if !folder_path.exists() {
                fs::create_dir_all(folder_path)?;
            }

            let icon_w = path_to_pcwstr(icon_path);
            unsafe {
                LoadImageW(
                    None,
                    PCWSTR(icon_w.as_ptr()),
                    IMAGE_ICON,
                    0,
                    0,
                    LR_LOADFROMFILE | LR_DEFAULTSIZE,
                )
                .map_err(|e| {
                    OperationError(format!("Couldn't load icon {}: {}", icon_path.display(), e))
                })?;
            }

            let ini_path = folder_path.join("desktop.ini");
            let ini_content = format!(
                "[.ShellClassInfo]\r\nIconResource={},0\r\n[ViewState]\r\nMode=\r\nVid=\r\nFolderType=Generic\r\n",
                icon_path.display()
            );
            fs::write(&ini_path, ini_content.as_bytes())?;

            // Set attributes: folder must be system; desktop.ini hidden+system+readonly
            unsafe {
                let folder_w = path_to_pcwstr(folder_path);
                let mut attrs = GetFileAttributesW(PCWSTR(folder_w.as_ptr()));
                let mut attrs_val: u32 = if attrs == INVALID_FILE_ATTRIBUTES {
                    0
                } else {
                    attrs
                };
                attrs_val |= FILE_ATTRIBUTE_SYSTEM.0;
                SetFileAttributesW(
                    PCWSTR(folder_w.as_ptr()),
                    windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(attrs_val),
                )
                .map_err(|e| OperationError(format!("SetFileAttributesW(folder) failed: {}", e)))?;

                let ini_w = path_to_pcwstr(&ini_path);
                let mut ini_attrs = GetFileAttributesW(PCWSTR(ini_w.as_ptr()));
                let mut ini_attrs_val: u32 = if ini_attrs == INVALID_FILE_ATTRIBUTES {
                    0
                } else {
                    ini_attrs
                };
                ini_attrs_val |=
                    FILE_ATTRIBUTE_HIDDEN.0 | FILE_ATTRIBUTE_SYSTEM.0 | FILE_ATTRIBUTE_READONLY.0;
                SetFileAttributesW(
                    PCWSTR(ini_w.as_ptr()),
                    windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(ini_attrs_val),
                )
                .map_err(|e| {
                    OperationError(format!("SetFileAttributesW(desktop.ini) failed: {}", e))
                })?;
            }

            Ok(())
        }

        fn register_application(&self, _path: &Path) -> BackendResult<()> {
            // No-op on Windows for now, registration typically uses registry/protocol handlers
            Ok(())
        }

        fn set_permission(
            &self,
            _path: &str,
            _value: &str,
            _user: Option<&str>,
        ) -> BackendResult<()> {
            Err(UnsupportedAction(
                "Setting permissions is not implemented for Windows backend".to_string(),
            ))
        }
    }
}
