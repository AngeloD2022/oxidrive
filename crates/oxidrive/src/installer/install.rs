use crate::core::condition::{ConditionEvaluator, EvalValue, parse_condition};
use crate::core::errors::GenericError;
use crate::core::models::{Application, CompressionType};
use crate::core::platform::ProductPlatform;
use crate::core::utils::{file_name_from_package, get_os_version};
use crate::installer::compression::HyperdriveLZMA2;
use crate::installer::inline_tokens::TokenExpander;
use crate::installer::os_actions::{BackendError, InstallActionBackend};
use crate::installer::pim::{Command, PIMXPackage, parse_pimx};
use globset::GlobBuilder;
use pathdiff::diff_paths;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fs, io};
use std::ops::Deref;
use tempfile::{TempDir, tempdir};
use thiserror::Error;
use zip::ZipArchive;
use zip::result::ZipError;

use crate::installer::install::InstallerError::{
    InvalidPackage, InvalidPrecondition, MalformedPIMX,
};

#[derive(Debug, Error)]
pub enum InstallerError {
    #[error("Invalid install precondition: {0}")]
    InvalidPrecondition(String),
    #[error("Invalid package: {0}")]
    InvalidPackage(String),
    #[error("Malformed package manifest: {0}")]
    MalformedPIMX(String),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Archive(#[from] ZipError),
    #[error(transparent)]
    Backend(#[from] BackendError),
}

pub type InstallerResult<T> = Result<T, InstallerError>;

pub trait InstallProgressSink {
    // todo: add more methods to this for more detailed progress reporting.

    fn item_completed(&mut self) {}
    fn error_occurred(&mut self) {}
}
pub struct NoopInstallProgress;
impl InstallProgressSink for NoopInstallProgress {}

pub struct InstallConfig {
    pub language: String,
}

struct ApplicationInstallContext {
    /// Main product SAP code.
    base_directory: PathBuf,
    application: Application,
}

impl ApplicationInstallContext {
    pub fn token_expander(&self) -> TokenExpander {
        TokenExpander::new(&self.application.install_dir.value)
    }

    pub fn condition_evaluator(&self, config: &InstallConfig) -> ConditionEvaluator {
        let mut vars = ProductPlatform::detect()
            .unwrap()
            .get_condition_vars()
            .unwrap();

        let os_ver = get_os_version();
        vars.insert("OSVersion".to_string(), EvalValue::Version(os_ver));
        vars.insert(
            "installLanguage".to_string(),
            EvalValue::String(config.language.clone()),
        );

        ConditionEvaluator::new(vars, false)
    }
}

// Product -> [Application] -> [Package]

// If download and installation occur together, the downloader can produce the necessary information
// to inform the installer without writing metadata files.

fn read_application_json(path: &Path) -> Result<Application, GenericError> {
    let data = fs::read_to_string(path)?;
    let application: Application = serde_json::from_str(&data)?;

    Ok(application)
}

pub struct ProductInstaller {
    main_sap: String,
    source_dir: PathBuf,
    config: InstallConfig,
    application_ctxs: Vec<ApplicationInstallContext>,
    backend: Box<dyn InstallActionBackend>,
}

impl ProductInstaller {
    pub fn new(
        main_sap: String,
        source_dir: PathBuf,
        config: InstallConfig,
        backend: Box<dyn InstallActionBackend>,
    ) -> Self {
        Self {
            main_sap,
            source_dir,
            config,
            application_ctxs: Vec::new(),
            backend,
        }
    }

    pub fn new_with_apps(
        main_sap: String,
        source_dir: PathBuf,
        config: InstallConfig,
        applications: Vec<Application>,
        backend: Box<dyn InstallActionBackend>,
    ) -> Self {
        let apps = applications
            .into_iter()
            .map(|e| ApplicationInstallContext {
                base_directory: source_dir.join(e.sap_code.to_ascii_uppercase()),
                application: e,
            })
            .collect();

        Self {
            main_sap,
            source_dir,
            config,
            application_ctxs: apps,
            backend,
        }
    }

    fn main_application(&self) -> Option<&ApplicationInstallContext> {
        self.application_ctxs
            .iter()
            .find(|e| e.application.sap_code == self.main_sap)
    }

    pub fn prewarm(&self) -> InstallerResult<()> {
        // TODO: check for sudo/admin

        // Ensure context base directory exists and is not empty.
        println!("SourceDir: {}", self.source_dir.display());
        if !self.source_dir.exists() {
            return Err(InvalidPrecondition(
                "Source directory doesn't exist.".to_string(),
            ));
        }

        let inner_dirs = fs::read_dir(&self.source_dir)
            .map_err(|e| InstallerError::IO(e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().into_string().unwrap())
            // .filter_map(|e| e.map(|s| s.file_name().into_string().unwrap()).ok())
            .collect::<Vec<_>>();
        if inner_dirs.len() == 0 {
            return Err(InvalidPrecondition("Base directory is empty.".to_string()));
        }

        let main_application = if let Some(ctx) = self.main_application() {
            &ctx.application
        } else {
            return Err(InvalidPrecondition(
                "Main application was not found in the install bundle.".to_string(),
            ));
        };

        if let Some(dependencies) = &main_application.dependencies {
            let has_all_deps = dependencies
                .dependency
                .iter()
                .all(|e| inner_dirs.contains(&e.sap_code));
            if !has_all_deps {
                return Err(InvalidPrecondition(
                    "Install is missing one or more dependencies.".to_string(),
                ));
            }
        }

        // Check if target platform matches our current platform.
        let allowed_platforms = ProductPlatform::detect()
            .ok_or(InvalidPrecondition(
                "Platform cannot be identified.".to_string(),
            ))?
            .allowed_platforms();
        if !allowed_platforms.contains(&main_application.platform) {
            return Err(InvalidPrecondition("Platform mismatch.".to_string()));
        }

        Ok(())
    }

    pub fn run<P: InstallProgressSink>(&self, progress: P) -> InstallerResult<()> {
        // Product
        // todo: use progress sink throughout this method.

        for app in &self.application_ctxs {
            println!("BEGIN1");
            // Application
            let eval = app.condition_evaluator(&self.config);
            let mut token_expander = app.token_expander();

            for package in &app.application.packages.package {
                if let Some(condition) = &package.condition
                    && !condition.is_empty()
                {
                    // todo: add error handling
                    println!("COND1: {}", condition);
                    let expression = parse_condition(condition).unwrap();
                    let should_proceed = eval.evaluate(&expression).unwrap();
                    if !should_proceed {
                        continue;
                    }
                }

                // Package
                let file_name = file_name_from_package(package);

                let path = app.base_directory.join(file_name.clone());
                println!("PATH: {}", path.to_string_lossy());

                let file = File::open(path)?;
                let f_clone = file.try_clone()?;
                let mut archive = ZipArchive::new(file)?;
                let mut interface = PackageInterface::new(
                    &mut archive,
                    f_clone,
                    &app.application
                        .compression_type
                        .as_ref()
                        .unwrap_or(&CompressionType::ZipDeflated),
                );

                let pimx = interface
                    .read_pimx()
                    .map_err(|e| {
                        InvalidPackage(format!(
                            "Failed to read install manifest for {}: {}",
                            file_name, e
                        ))
                    })?
                    .ok_or(InvalidPackage(format!(
                        "No install manifest for {}!",
                        file_name
                    )))?;

                let staging_dir = interface.staging_directory().ok_or(InvalidPackage(format!(
                    "Package {} does not appear to have a staging directory!",
                    file_name
                )))?;

                println!("Staging Directory: {}", staging_dir);

                let staging_dir = staging_dir
                    .strip_suffix('/')
                    .unwrap_or(staging_dir)
                    .to_string();

                // First, check if we should proceed with installation by running the condition expression if one exists.
                if let Some(condition) = &pimx.condition
                    && !condition.is_empty()
                {
                    let expression = parse_condition(condition).map_err(|_| {
                        MalformedPIMX(format!(
                            "Could not parse condition expression for package {}",
                            file_name
                        ))
                    })?;

                    let should_proceed = eval.evaluate(&expression).map_err(|e| {
                        MalformedPIMX(format!(
                            "Condition evaluation error for package {}: {}",
                            file_name, e
                        ))
                    })?;

                    if !should_proceed {
                        continue;
                    }
                }

                // Begin installation by extracting specified assets from the archive according to the manifest.
                for asset in &pimx.assets {
                    println!("asset source: {}", asset.source);
                    println!("asset target: {}", asset.target);

                    let source =
                        expand_token(&mut token_expander, &asset.source)?.replace("\\", "/");

                    let source = source.strip_prefix('/').unwrap_or(&source).to_string();

                    let target =
                        PathBuf::from_str(&expand_token(&mut token_expander, &asset.target)?)
                            .unwrap();

                    let staging_source = format!("{}/{}", staging_dir, source);
                    let mut source_glob = staging_source
                        .clone()
                        .strip_suffix('/')
                        .unwrap_or(&staging_source)
                        .to_string();
                    if interface.is_directory(&source_glob) {
                        if asset.recursive {
                            source_glob = format!("{}/**/*", source_glob);
                        } else {
                            source_glob = format!("{}/*", source_glob);
                        }
                    }

                    let sources = interface
                        .glob_match(&source_glob)
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>();

                    for path in sources {
                        let path_difference = diff_paths(&path, staging_source.clone()).unwrap();
                        let host_path = target.join(path_difference);

                        if interface.is_directory(&path) {
                            self.backend.create_dir(&host_path)?;
                            continue;
                        }

                        let content = match interface.read_file(&path) {
                            Err(e) if matches!(e.kind(), io::ErrorKind::InvalidData) => {
                                // this usually happens when the file in question is a symlink.
                                // warn!("Invalid LZMA data read for {}!", path);
                                continue;
                            }
                            Err(e) => {
                                println!("Failed to read file: {e}");
                                return Err(e.into());
                            }
                            Ok(content) => content,
                        };

                        match content {
                            ExtractedEntity::File(content, mode) => {
                                self.backend.create_file(&host_path, &content, mode)?;
                            }
                            ExtractedEntity::Symlink(to_path) => {
                                self.backend.create_shortcut(&host_path, to_path.as_ref())?;
                            }
                        }
                    }
                }

                // carry out post-install actions:
                for command in &pimx.commands {
                    match command {
                        Command::Registry(c) => self.backend.create_registry(c)?,
                        Command::FolderIcon(c) => {
                            let folder_path = expand_token(&mut token_expander, &c.folder_path)?;
                            let icon_path = expand_token(&mut token_expander, &c.icon_path)?;
                            if let Err(e) = self
                                .backend
                                .apply_folder_icon(folder_path.as_ref(), icon_path.as_ref())
                            {
                                println!("Warning: Failed to set folder icon: {}", e);
                            }
                        }
                        Command::RegisterApplication(c) => {
                            let path = expand_token(&mut token_expander, &c.path)?;
                            if let Err(e) = self.backend.register_application(path.as_ref()) {
                                println!("Warning: Failed to register application: {}", e);
                            }
                        }
                        Command::Permission(c) => {
                            // let path = expand_token(&mut token_expander, &c.path)?;
                            // self.backend.set_permission(
                            //     &path,
                            //     &c.permission_value,
                            //     c.user.as_deref(),
                            // )?;
                        }
                        Command::InstallUserPref(_) => {
                            // note: this seems to be a no-op on macOS.
                        }
                        Command::Owner(_) => {}
                        Command::RunProgram(c) => {
                            let _p = handle_path(
                                &mut interface,
                                &mut token_expander,
                                self.backend.as_ref(),
                                &c.path,
                            )?;

                            let path = _p.path();

                            let args = c.arguments.clone().map(|args| {
                                args.into_iter()
                                    .map(|arg| expand_token(&mut token_expander, &arg))
                                    .collect::<Result<Vec<_>, _>>()
                            });
                            let args = args.transpose()?;

                            let code = self.backend.run_program(path.as_ref(), args.as_deref())?;

                            if let Some(valid_codes) = &c.success_exit_codes {
                                if !valid_codes.contains(&code) {
                                    // warn!(
                                    //     "Program {} run resulted in invalid exit code: {}",
                                    //     path, code
                                    // );
                                }
                            }
                        }
                        Command::Shortcut(_) => {}
                    }
                }
            }
        }

        Ok(())
    }
}

enum HandledPath {
    Real(PathBuf),
    Temporary(TempDir, PathBuf),
}

impl HandledPath {
    pub fn path_string(&self) -> String {
        match self {
            HandledPath::Real(v) => v.to_string_lossy().to_string(),
            HandledPath::Temporary(_, v) => v.to_string_lossy().to_string(),
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            HandledPath::Real(v) => v.as_ref(),
            HandledPath::Temporary(_, v) => v.as_ref(),
        }
    }
}

///
/// This function exists because some path values contain `[StagingFolder]` in manifest commands.
///
/// We need this because we do not extract the full zip file before we install something. The files
/// that are not specified in the manifest for copying are not extracted from the zip.
///
/// Nevertheless, some commands do rely upon `[StagingFolder]` as though everything from the zip
/// exists in the filesystem, and so we need to handle these cases by creating a temporary
/// directory.
///
/// Long term, this is probably not a great solution. We could visit the entire PIMX beforehand and
/// record these particular instances- and copy everything to the temporary directory in one swoop.
///
fn handle_path(
    interface: &mut PackageInterface,
    token_expander: &mut TokenExpander,
    backend: &dyn InstallActionBackend,
    path: &str,
) -> InstallerResult<HandledPath> {
    if path.starts_with("[StagingFolder]") {
        let p = expand_token(token_expander, path)?;
        let p = p.strip_suffix('/').unwrap_or(&p);

        let glob = if interface.is_directory(&p) {
            format!("{}/**/*", p)
        } else {
            p.to_string()
        };
        println!("TEXTRACT_GLOB: {glob}");
        let temp = extract_temporary(interface, backend, &glob)?;

        // let staging_dir = interface
        //     .staging_directory()
        //     .unwrap_or("/")
        //     .to_string();

        let path = temp.path().join(p);
        Ok(HandledPath::Temporary(temp, path))
    } else {
        let p = expand_token(token_expander, path)?;
        Ok(HandledPath::Real(PathBuf::from(p)))
    }
}

fn extract_temporary(
    interface: &mut PackageInterface,
    backend: &dyn InstallActionBackend,
    glob: &str,
) -> InstallerResult<TempDir> {
    let temp = tempdir()?;
    let path = temp.path();

    // precondition: glob always begins with the staging directory.
    let staging = interface.staging_directory().unwrap().to_string();

    let sources = interface
        .glob_match(glob)
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    for file in &sources {
        let host_path = path.join(diff_paths(file, &staging).unwrap());

        let extracted = interface.read_file(file)?;
        match extracted {
            ExtractedEntity::File(content, mode) => {
                backend.create_file(&host_path, &content, mode)?;
            }
            ExtractedEntity::Symlink(to_path) => {
                backend.create_shortcut(&host_path, to_path.as_ref())?;
            }
        }
    }

    Ok(temp)
}

fn expand_token(expander: &mut TokenExpander, item: &str) -> InstallerResult<String> {
    expander
        .expand(item)
        .map_err(|_| MalformedPIMX(format!("{} contained an unrecognized token.", item)))
}

fn utf8_bytes_to_u64(bytes: &[u8]) -> Option<u64> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    let trimmed = &bytes[..end];
    let s = std::str::from_utf8(trimmed).ok()?;

    s.trim().parse::<u64>().ok()
}

enum ExtractedEntity {
    File(Vec<u8>, Option<u32>),
    Symlink(String),
}

struct PackageInterface<'a> {
    archive: &'a mut ZipArchive<File>,
    file: File,
    _arch_index: Vec<String>,
    _arch_dirs: HashSet<String>,
    _hdc_instance: Option<HyperdriveLZMA2>,
}

impl<'a> PackageInterface<'a> {
    pub fn new(
        archive: &'a mut ZipArchive<File>,
        file: File,
        compression: &CompressionType,
    ) -> Self {
        let hdc = match compression {
            CompressionType::ZipDeflated => None,
            CompressionType::ZipLzma2 => Some(HyperdriveLZMA2::new().unwrap()),
        };

        let mut dirs = HashSet::new();
        let mut files = Vec::new();

        for i in 0..archive.len() {
            let f = archive.by_index(i).unwrap();
            let name = f.name().to_string().replace("\\", "/");

            if name.ends_with('/') {
                dirs.insert(name);
            } else {
                files.push(name.clone());

                let path = Path::new(&name);
                let mut parent = path.parent();
                while let Some(p) = parent {
                    if let Some(p_str) = p.to_str() {
                        if !p_str.is_empty() {
                            let dir_name = format!("{}/", p_str);
                            dirs.insert(dir_name);
                        }
                    }
                    parent = p.parent();
                }
            }
        }

        let mut index = files;
        index.extend(dirs.clone().into_iter());

        Self {
            archive,
            file,
            _arch_index: index,
            _arch_dirs: dirs,
            _hdc_instance: hdc,
        }
    }

    fn read_local_extra_data(&mut self, header_start: u64) -> io::Result<Vec<u8>> {
        // HDPIM.dylib @ UnzipHandler::unzipFile
        // 01020000 14000000 08007276 4F5BEFA4 7CBC2200 00002000 0000

        const LFH_SIG: u32 = 0x04034b50;
        self.file.seek(SeekFrom::Start(header_start))?;

        let mut sig = [0u8; 4];
        self.file.read_exact(&mut sig)?;
        let sig = u32::from_le_bytes(sig);
        if sig != LFH_SIG {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a local file header",
            ));
        }

        let mut fixed = [0u8; 26];
        self.file.read_exact(&mut fixed)?;

        let name_len = u16::from_le_bytes([fixed[22], fixed[23]]) as u64;
        let extra_len = u16::from_le_bytes([fixed[24], fixed[25]]) as u64;

        self.file.seek(SeekFrom::Current(name_len as i64))?;

        let mut extra = vec![0u8; extra_len as usize];
        self.file.read_exact(&mut extra)?;

        Ok(extra)
    }

    fn read_external_attributes(&mut self, central_header_start: u64) -> io::Result<u32> {
        const CH_SIG: u32 = 0x02014b50;
        self.file.seek(SeekFrom::Start(central_header_start))?;

        let mut sig = [0u8; 4];
        self.file.read_exact(&mut sig)?;
        if u32::from_le_bytes(sig) != CH_SIG {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a central file header",
            ));
        }

        const CH_EXTERNALDATA_OFF: u32 = 38 - 4;
        self.file
            .seek(SeekFrom::Current(CH_EXTERNALDATA_OFF as i64))?;

        let mut external_data = [0u8; 4];
        self.file.read_exact(&mut external_data)?;

        Ok(u32::from_le_bytes(external_data))
    }

    fn read_file(&mut self, path: &str) -> io::Result<ExtractedEntity> {
        let (buf, header_start, cd_header) = {
            let mut f = self.archive.by_name(path)?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;

            let header_start = f.header_start();
            let cd_header = f.central_header_start();
            let _fuck_you = f.unix_mode();

            (buf, header_start, cd_header)
        };

        let external_attrs = self.read_external_attributes(cd_header)?;
        let mode = (external_attrs >> 16) & 0o177777;
        let is_symlink = mode & 0o170000 == 0o120000;

        // compressed-with-LZMA2
        if self._hdc_instance.is_some() && !is_symlink {
            // Occasionally, there seems to be a discrepancy in the application metadata
            // regarding its compression type.
            // If this happens, fallback to normal decompression.
            if let Ok(extra) = self.read_local_extra_data(header_start) {
                let value = utf8_bytes_to_u64(&extra).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "invalid LZMA2 extra data")
                })?;

                let compression = self._hdc_instance.as_mut().unwrap();
                let buf = compression.decompress(&buf, value).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "LZMA2 decompress failed")
                })?;
                // todo: hold the unix mode in this enum as well so that we can write executables with +x
                Ok(ExtractedEntity::File(buf, Some(mode)))
            } else {
                Ok(ExtractedEntity::Symlink(String::from_utf8(buf).unwrap()))
            }
        } else {
            // normal zip
            if is_symlink {
                Ok(ExtractedEntity::Symlink(String::from_utf8(buf).unwrap()))
            } else {
                Ok(ExtractedEntity::File(buf, Some(mode)))
            }
        }
    }

    pub fn is_directory(&self, path: &str) -> bool {
        let path = if !path.ends_with('/') {
            format!("{}/", path)
        } else {
            path.to_string()
        };

        self._arch_dirs.contains(&path)
    }

    pub fn glob_match(&self, expr: &str) -> Vec<&str> {
        // todo: evaluate whether this is a potential performance issue.
        println!("CALLGLOB1: {}", expr);
        let pattern = GlobBuilder::new(expr)
            .literal_separator(true)
            .build()
            .unwrap()
            .compile_matcher();
        self._arch_index
            .par_iter()
            .filter(|p| pattern.is_match(p))
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
    }

    pub fn glob_match_single(&self, expr: &str) -> Option<&str> {
        // todo: evaluate whether this is a potential performance issue.
        println!("CALLGLOB2: {}", expr);
        let pattern = GlobBuilder::new(expr)
            .literal_separator(true)
            .build()
            .unwrap()
            .compile_matcher();
        self._arch_index
            .par_iter()
            .find_first(|p| pattern.is_match(p))
            .map(|s| s.as_str())
    }

    pub fn staging_directory(&self) -> Option<&str> {
        self.glob_match_single("*/")
    }

    pub fn read_pimx(&mut self) -> io::Result<Option<PIMXPackage>> {
        let name = {
            match self.glob_match_single("*.pimx") {
                Some(s) => s.to_string(),
                None => return Ok(None),
            }
        };

        // let bytes = self.read_file(&name)?;

        if let ExtractedEntity::File(bytes, _) = self.read_file(&name)? {
            let xml = String::from_utf8(bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            let pkg = parse_pimx(&xml)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Couldn't parse PIMX"))?;

            Ok(Some(pkg))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "PIMX file was a symlink and not a file",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::models::CompressionType;
    use crate::core::platform::ProductPlatform;
    use crate::core::platform::ProductPlatform::MacUniversal;
    use crate::installer::install::{
        InstallConfig, NoopInstallProgress, PackageInterface, ProductInstaller,
    };
    use crate::installer::os_actions::DebugBackend;
    use crate::remote::products::ProductsClient;
    use std::fs::File;
    use std::path::PathBuf;
    use std::str::FromStr;
    use zip::ZipArchive;

    #[test]
    fn test_archive_intf() {
        // let f = File::open("/Users/angelodeluca/RustroverProjects/oxidrive/dl_test/COSY/CoreSync-mul.zip").unwrap();
        let f = File::open(
            "/Users/angelodeluca/RustroverProjects/oxidrive/VCRedist14-64.zip",
        )
        .unwrap();
        // let f = File::open("/Users/angelodeluca/RustroverProjects/oxidrive/dl_test/CORG/AdobeColorCommonSetRGB_1_0-mul.zip").unwrap();
        // let f = File::open("/Users/angelodeluca/RustroverProjects/oxidrive/dl_test/CCXP/CCX-Process-mul.zip").unwrap();
        let f_clone = f.try_clone().unwrap();
        let mut archive = ZipArchive::new(f).unwrap();

        let mut interface =
            PackageInterface::new(&mut archive, f_clone, &CompressionType::ZipDeflated);

        let _stagedir = interface.staging_directory();

        let _manifest = interface.read_pimx();

        for f in
            interface.glob_match("1/")
        {
            println!("{}", f);
        }

        // let read = interface.read_file("1/Application/Adobe Photoshop 2026.app/Contents/Frameworks/AdobeAXEDOMCore.framework/AdobeAXEDOMCore").unwrap();
        // let read = interface.read_file("1/Application/Adobe Photoshop 2026.app/Contents/PkgInfo").unwrap();
        // let read = interface.read_file("1/CoreSync/Core Sync.app/Contents/Resources/Log Collector tool.app/Contents/PkgInfo").unwrap();
    }

    #[tokio::test]
    async fn test_pc() {
        let plat = ProductPlatform::MacAarch64;
        let mut client = ProductsClient::new(plat).await;

        let mut client = client.unwrap();

        let product_sap = "ACR";
        let ch = client.get_reduced_channel("STM").unwrap();
        let latest = ch.index.get_latest(product_sap, plat).unwrap();

        let guid = latest.build_guid.unwrap().to_owned();
        let application = client.get_application(&guid).await.unwrap();
    }

    #[tokio::test]
    async fn test_installer() {
        let plat = ProductPlatform::MacAarch64;
        let mut client = ProductsClient::new(plat).await;

        let mut client = client.unwrap();

        let product_sap = "AEFT";
        let ch = client.get_reduced_channel("CCM").unwrap();
        let latest = ch.index.get_latest(product_sap, MacUniversal).unwrap();

        let guid = latest.build_guid.unwrap().to_owned();
        let application = client.get_application(&guid).await.unwrap();

        let mut apps = client
            .get_download_dependencies(&application)
            .await
            .unwrap()
            .unwrap();

        apps.push(application);

        let backend = Box::new(DebugBackend {});
        let base_dir = "/Users/angelodeluca/RustroverProjects/oxidrive/dl_test";
        let install_cfg = InstallConfig {
            language: "en_US".to_string(),
        };
        let progress = NoopInstallProgress {};

        let installer = ProductInstaller::new_with_apps(
            product_sap.to_string(),
            PathBuf::from_str(base_dir).unwrap(),
            install_cfg,
            apps,
            backend,
        );
        installer.prewarm().unwrap();
        installer.run(progress).unwrap();

    }
}
