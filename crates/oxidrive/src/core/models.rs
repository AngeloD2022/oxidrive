use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Products {
    pub channels: Channels,

    pub builds: Builds,

    pub containers: Containers,

    pub entitlement_status: String,
}

#[derive(Serialize, Deserialize)]
pub struct Builds {
    pub build: Vec<Build>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    pub ngl_licensing_info: NglLicensingInfo,

    pub id: String,

    #[serde(rename = "type")]
    pub build_type: Type,

    pub version: String,

    pub platform: String,

    pub language_set: String,

    pub entitled: bool,

    pub maintenance_build: bool,

    pub go_live_time: i64,
}

#[derive(Serialize, Deserialize)]
pub enum Type {
    Desktop,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NglLicensingInfo {
    pub app_id: Option<String>,

    pub app_version: Option<String>,

    pub lib_version: Option<String>,

    pub build_id: Option<String>,

    pub ims_client_id: Option<String>,

    pub license_mode: Option<LicenseMode>,

    pub ims_app_profile_scope: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum LicenseMode {
    #[serde(rename = "FREE")]
    Free,

    #[serde(rename = "FREEMIUM")]
    Freemium,

    #[serde(rename = "PAID")]
    Paid,

    #[serde(rename = "RESIDUAL")]
    Residual,
}

#[derive(Serialize, Deserialize)]
pub struct Channels {
    pub channel: Vec<Channel>,

    pub version: String,

    pub timestamp: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub cdn: Cdn,

    pub products: ProductsClass,

    pub services: MobileApps,

    pub mobile_apps: MobileApps,

    pub latest_version_group: String,

    #[serde(rename = "custom-data")]
    pub custom_data: CustomData,

    pub display_name: String,

    pub name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cdn {
    pub secure: String,

    pub non_secure: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CustomData {
    pub custom_entry: Vec<CustomEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct CustomEntry {
    pub value: Vec<String>,

    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub struct MobileApps {}

#[derive(Serialize, Deserialize)]
pub struct ProductsClass {
    pub product: Vec<Product>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    #[serde(rename = "type")]
    pub product_type: Type,

    pub display_name: String,

    pub family: String,

    pub product_icons: ProductIcons,

    pub categories: Categories,

    pub platforms: Platforms,

    pub referenced_products: ReferencedProducts,

    #[serde(rename = "custom-data")]
    pub custom_data: CustomData,

    pub version: String,

    pub id: String,

    pub sort_index: i64,

    pub upgrades_older_version: bool,

    pub remove_conflicts: bool,

    pub product_info_page: Option<String>,

    pub app_lineage: Option<String>,

    pub family_name: Option<String>,

    pub groups: Option<String>,

    pub dependency_type: Option<DependencyType>,
}

#[derive(Serialize, Deserialize)]
pub struct Categories {
    pub category: Vec<Category>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Category {
    pub value: String,

    pub sort_order: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Visible,
}

#[derive(Serialize, Deserialize)]
pub struct Platforms {
    pub platform: Vec<PlatformElement>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformElement {
    pub language_set: Vec<LanguageSet>,

    #[serde(rename = "custom-data")]
    pub custom_data: CustomData,

    pub system_compatibility: SystemCompatibility,

    pub id: String,

    pub modules: Option<Modules>,

    pub suite_type: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageSet {
    pub locales: Locales,

    pub urls: Urls,

    pub dependencies: DependenciesClass,

    pub additional_product_codes: AdditionalProductCodes,

    pub upgrade_product_codes: MobileApps,

    #[serde(rename = "custom-data")]
    pub custom_data: CustomData,

    pub product_code: String,

    pub name: String,

    pub install_size: i64,

    pub package_type: PackageType,

    pub uwp_product: bool,

    pub engagement_build: bool,

    pub maintenance_build: bool,

    pub package_code: Option<String>,

    pub build_guid: Option<String>,

    pub base_version: Option<String>,

    pub product_version: Option<String>,

    pub esd_data: Option<EsdData>,

    pub bundle_id: Option<String>,

    pub bundle_version: Option<String>,

    pub file_type: Option<String>,

    pub windows_package: Option<WindowsPackage>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalProductCodes {
    pub product_code: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DependenciesClass {
    pub dependency: Option<Vec<Dependency>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub sap_code: String,
    pub base_version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EsdData {
    pub name: String,

    pub size: i64,

    pub asset_guid: String,
}

#[derive(Serialize, Deserialize)]
pub struct Locales {
    pub locale: Vec<LocaleElement>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LocaleElement {
    pub custom_data: CustomData,

    pub name: String,

    pub leid: Option<String>,

    pub entitled: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackageType {
    #[serde(rename = "applePackage")]
    ApplePackage,

    Application,

    #[serde(rename = "hdPackage")]
    HdPackage,

    #[serde(rename = "msiPackage")]
    MsiPackage,

    #[serde(rename = "RIBS")]
    Ribs,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Urls {
    #[serde(rename = "manifestURL")]
    pub manifest_url: Option<String>,

    #[serde(rename = "lbsURL")]
    pub lbs_url: String,

    #[serde(rename = "aamURL")]
    pub aam_url: String,

    #[serde(rename = "cc-uri")]
    pub cc_uri: CcUri,
}

#[derive(Serialize, Deserialize)]
pub struct CcUri {
    pub uri: Vec<Uri>,
}

#[derive(Serialize, Deserialize)]
pub struct Uri {
    pub value: String,

    pub version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsPackage {
    #[serde(rename = "launchURI")]
    pub launch_uri: String,

    pub identity: Identity,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub name: String,

    pub publisher: String,

    pub processor_architecture: String,

    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct Modules {
    pub module: Vec<Module>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    pub display_name: String,

    pub deployment_type: DeploymentType,

    pub requires_user_consent: Option<bool>,

    pub id: String,
}

#[derive(Serialize, Deserialize)]
pub enum DeploymentType {
    Deferred,

    #[serde(rename = "OnDemand")]
    OnDemand,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemCompatibility {
    pub operating_system: OperatingSystem,
}

#[derive(Serialize, Deserialize)]
pub struct OperatingSystem {
    pub range: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ProductIcons {
    pub icon: Vec<Icon>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    pub value: String,

    pub size: String,

    pub icon_guid: String,

    pub registry_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferencedProducts {
    pub referenced_product: Option<Vec<ReferencedProduct>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferencedProduct {
    pub sap_code: String,

    pub version: String,

    pub sequence: String,
}

#[derive(Serialize, Deserialize)]
pub struct Containers {
    pub container: Vec<Option<serde_json::Value>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Application {
    pub name: String,

    #[serde(rename = "SAPCode")]
    pub sap_code: String,

    pub codex_version: String,

    pub asset_guid: String,

    pub product_version: String,

    pub base_version: String,

    pub lbs_url: String,

    pub platform: String,

    pub supported_languages: SupportedLanguages,

    pub language_set: String,

    pub conflicting_processes: Option<ConflictingProcesses>,

    #[serde(rename = "AMTConfig")]
    pub amt_config: AmtConfig,

    pub packages: Packages,

    pub system_requirement: Option<SystemRequirement>,

    #[serde(rename = "version")]
    pub version: String,

    pub app_lineage: Option<String>,

    pub family_name: Option<String>,

    pub build_guid: String,

    #[serde(rename = "selfServeBuild")]
    pub self_serve_build: Option<bool>,

    #[serde(rename = "HDBuilderVersion")]
    pub hd_builder_version: Option<String>,

    #[serde(rename = "IsSTI")]
    pub is_sti: Option<bool>,

    pub auto_update: Option<String>,

    pub product_description: Option<ProductDescription>,

    pub cdn: CdnClass,

    #[serde(rename = "IsNonCCProduct")]
    pub is_non_cc_product: Option<bool>,

    pub compression_type: Option<CompressionType>,

    pub tutorial_url: Option<Url>,

    pub minimum_supported_client_version: Option<String>,

    pub install_dir: InstallDir,

    pub modules: Option<ModulesClass>,

    pub dependencies: Option<SoftDependenciesClass>,

    pub ngl_licensing_info: Option<NglLicensingInfoClass>,

    pub delta_update_config: Option<DeltaUpdateConfig>,

    pub apps_panel_full_app_update_config: Option<AppsPanelFullAppUpdateConfig>,

    pub reference_products: Option<ReferenceProducts>,

    pub whats_new_url: Option<Url>,

    pub app_launch: Option<String>,

    pub more_info_url: Option<Url>,

    pub add_remove_info: Option<AddRemoveInfo>,

    pub apps_panel_previous_version_config: Option<AppsPanelPreviousVersionConfig>,

    pub mac_thin_config: Option<String>,

    pub auto_install: Option<bool>,

    pub is_self_reference: Option<bool>,

    pub is_visible_product: Option<bool>,

    pub is_free_product: Option<bool>,

    pub update_description_url: Option<Url>,

    pub windows_package: Option<WindowsPackageClass>,

    pub vulcan_config: Option<VulcanConfig>,

    pub preserve_user_preferences_on_patch_rollback: Option<bool>,

    pub soft_dependencies: Option<SoftDependenciesClass>,

    #[serde(rename = "IsUWPProduct")]
    pub is_uwp_product: Option<bool>,

    pub esd_display_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AddRemoveInfo {
    pub display_name: SupportedLanguages,

    pub display_icon: Option<String>,

    pub display_version: Option<SupportedLanguages>,

    #[serde(rename = "URLInfoAbout")]
    pub url_info_about: Option<SupportedLanguages>,

    #[serde(rename = "URLUpdateInfo")]
    pub url_update_info: Option<SupportedLanguages>,

    pub help_link: Option<SupportedLanguages>,

    pub comments: Option<SupportedLanguages>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SupportedLanguages {
    pub language: Vec<Language>,
}

#[derive(Serialize, Deserialize)]
pub struct Language {
    pub value: String,

    pub locale: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmtConfig {
    #[serde(rename = "appID")]
    pub app_id: String,

    pub path: Option<String>,

    #[serde(rename = "LEID")]
    pub leid: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AppsPanelFullAppUpdateConfig {
    pub previous_version_range: PreviousVersionRange,

    pub show_dialog_box: bool,

    pub import_preference_check_box: CheckBox,

    pub remove_previous_version_check_box: CheckBox,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CheckBox {
    pub default_value: bool,

    pub show: bool,

    pub allow_toggle: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PreviousVersionRange {
    pub min: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AppsPanelPreviousVersionConfig {
    pub list_in_previous_version: bool,

    pub branding_name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CdnClass {
    pub secure: String,

    pub non_secure: String,
}

#[derive(Serialize, Deserialize)]
pub enum CompressionType {
    #[serde(rename = "Zip-Deflated")]
    ZipDeflated,

    #[serde(rename = "Zip-Lzma2")]
    ZipLzma2,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConflictingProcesses {
    pub conflicting_process: Vec<ConflictingProcess>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConflictingProcess {
    pub regular_expression: String,

    pub process_display_name: String,

    pub reason: String,

    pub parent_regular_expression: Option<String>,

    pub parent_display_name: Option<String>,

    pub relative_path: Option<String>,

    #[serde(rename = "headless")]
    pub headless: bool,

    #[serde(rename = "forceKillAllowed")]
    pub force_kill_allowed: bool,

    #[serde(rename = "adobeOwned")]
    pub adobe_owned: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeltaUpdateConfig {
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SoftDependenciesClass {
    pub dependency: Vec<DependencyElement>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DependencyElement {
    #[serde(rename = "SAPCode")]
    pub sap_code: String,

    pub base_version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallDir {
    pub value: String,

    pub max_path: Option<String>,

    pub is_fixed: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ModulesClass {
    pub module: Vec<ModuleElement>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ModuleElement {
    pub id: String,

    pub display_name: String,

    pub deployment_type: DeploymentType,

    pub requires_user_consent: Option<bool>,

    pub reference_packages: ReferencePackages,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReferencePackages {
    pub reference_package: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Url {
    pub stage: Option<SupportedLanguages>,

    pub prod: Option<SupportedLanguages>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NglLicensingInfoClass {
    pub app_id: String,

    pub app_version: String,

    pub lib_version: String,

    pub build_id: String,

    pub ims_client_id: String,

    pub ims_app_profile_scope: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Packages {
    pub package: Vec<Package>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
    #[serde(rename = "Type")]
    pub package_type: Option<PackageKind>,

    pub package_name: String,

    pub package_version: String,

    pub download_size: i64,

    pub extract_size: i64,

    pub path: String,

    pub features: Features,

    pub format: Option<String>,

    #[serde(rename = "ValidationURL")]
    pub validation_url: String,

    #[serde(rename = "packageHashKey")]
    pub package_hash_key: String,

    pub delta_packages: Vec<DeltaPackage>,

    #[serde(rename = "ValidationURLs")]
    pub validation_ur_ls: ValidationUrLs,

    pub processor_family: Option<ProcessorFamily>,

    pub install_sequence_number: i64,

    #[serde(rename = "fullPackageName")]
    pub full_package_name: Option<String>,

    pub package_validation: Option<String>,

    pub alias_package_name: String,

    pub package_scheme: Option<String>,

    pub condition: Option<String>,

    #[serde(rename = "RIBSCoexistenceCode")]
    pub ribs_coexistence_code: Option<String>,

    pub is_shared: Option<bool>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeltaPackage {
    pub schema_version: String,

    pub package_name: String,

    pub path: String,

    pub base_package_version: String,

    #[serde(rename = "ValidationURL")]
    pub validation_url: String,

    pub download_size: i64,

    pub extract_size: i64,

    pub metadata_file_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Features {
    pub feature: Vec<MobileApps>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageKind {
    Core,

    #[serde(rename = "non-core")]
    NonCore,

    Resources,
}

#[derive(Serialize, Deserialize)]
pub enum ProcessorFamily {
    #[serde(rename = "32-bit")]
    The32Bit,

    #[serde(rename = "64-bit")]
    The64Bit,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct ValidationUrLs {
    pub type2: Option<String>,

    pub type1: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProductDescription {
    pub tagline: SupportedLanguages,

    pub detailed_description: Option<SupportedLanguages>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReferenceProducts {
    pub reference_product: Vec<ReferenceProduct>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReferenceProduct {
    #[serde(rename = "SAPCode")]
    pub sap_code: String,

    pub base_version: String,

    #[serde(rename = "sequence")]
    pub sequence: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SystemRequirement {
    pub os_version: Option<PreviousVersionRange>,

    pub check_compatibility: Option<CheckCompatibility>,

    pub supported_os_version_range: Option<Vec<SupportedOsVersionRange>>,

    pub external_url: Option<Url>,

    pub microsoft_vc_runtime_list: Option<MicrosoftVcRuntimeList>,

    pub os_component: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CheckCompatibility {
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct MicrosoftVcRuntimeList {
    pub win64: String,

    pub win32: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SupportedOsVersionRange {
    pub min: String,

    pub max: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VulcanConfig {
    #[serde(rename = "HostID")]
    pub host_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WindowsPackageClass {
    #[serde(rename = "LaunchURI")]
    pub launch_uri: String,

    pub identity: IdentityClass,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IdentityClass {
    pub name: String,

    pub publisher: String,

    pub processor_architecture: String,

    pub version: String,
}
