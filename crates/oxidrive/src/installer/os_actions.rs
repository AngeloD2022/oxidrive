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
    use crate::installer::os_actions::{BackendResult, InstallActionBackend};
    use crate::installer::pim::RegistryCommand;
    use std::path::Path;

    pub struct WindowsBackend;

    impl InstallActionBackend for WindowsBackend {
        fn create_shortcut(&self, at_path: &Path, to_path: &Path) -> BackendResult<()> {
            todo!()
        }

        fn create_registry(&self, registry: &RegistryCommand) -> BackendResult<()> {
            todo!()
        }

        fn apply_folder_icon(&self, folder_path: &Path, icon_path: &Path) -> BackendResult<()> {
            todo!()
        }

        fn register_application(&self, path: &Path) -> BackendResult<()> {
            todo!()
        }

        fn set_permission(&self, path: &str, value: &str, user: Option<&str>) -> BackendResult<()> {
            todo!()
        }
    }
}
