use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use oxidrive::core::condition::EvalValue;
use oxidrive::core::platform::ProductPlatform;
use oxidrive::core::utils::get_os_version;
use std::path::PathBuf;

const HELP_TEMPLATE: &str = r#"
>>> {name} {version} <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
{about}

{before-help}USAGE:
  {usage}

COMMANDS:
{subcommands}

OPTIONS:
{options}

{after-help}
"#;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ProductField {
    #[value(alias = "ps")]
    Photoshop,
    #[value(alias = "ae")]
    AfterEffects,
    // #[value(alias = "fr")]
    // Fresco,
    #[value(alias = "me")]
    MediaEncoder,
    #[value(alias = "pr")]
    Premiere,
    #[value(alias = "an")]
    Animate,
    #[value(alias = "dw")]
    Dreamweaver,
    #[value(alias = "lrc")]
    LightroomClassic,
    #[value(alias = "il")]
    Illustrator,
    // #[value(alias = "fi")]
    // Firefly,
    #[value(alias = "au")]
    Audition,
    #[value(alias = "br")]
    Bridge,
    // Express,
    #[value(alias = "lr")]
    Lightroom,
    #[value(alias = "id")]
    InDesign,
    #[value(alias = "ch")]
    CharacterAnimator,
    // XD,
    Substance3DSampler,
    Substance3DDesigner,
    Substance3DPainter,
    UXPDeveloperTools,
}

impl ProductField {
    pub fn to_sap(&self) -> String {
        match self {
            ProductField::Photoshop => "PHSP",
            ProductField::AfterEffects => "AEFT",
            ProductField::MediaEncoder => "AME",
            ProductField::Premiere => "PPRO",
            ProductField::Animate => "FLPR",
            ProductField::Dreamweaver => "DRWV",
            ProductField::LightroomClassic => "LTRM",
            ProductField::Illustrator => "ILST",
            ProductField::Audition => "AUDT",
            ProductField::Bridge => "KBRG",
            ProductField::Lightroom => "LRCC",
            ProductField::InDesign => "IDSN",
            ProductField::CharacterAnimator => "CHAR",
            ProductField::Substance3DSampler => "SBSTA",
            ProductField::Substance3DDesigner => "SBSTD",
            ProductField::Substance3DPainter => "SBSTP",
            ProductField::UXPDeveloperTools => "UXPD",
        }
        .to_string()
    }
}

#[derive(Parser)]
#[command(name = "OXIDRIVE")]
#[command(author = "Angelo DeLuca")]
#[command(about = "A (currently experimental) bullshit-free Adobe software manager.")]
#[command(version = "0.0.1")]
#[command(help_template = HELP_TEMPLATE)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// UNIMPLEMENTED
    Wizard,
    Install(InstallCommand),
    /// UNIMPLEMENTED
    Package,
    /// UNIMPLEMENTED
    List,
}

#[derive(Parser)]
pub struct InstallCommand {
    pub product: ProductField,

    #[arg(long, default_value = "en_US")]
    pub language: String,

    #[arg(long, short = 'v')]
    pub version: Option<String>,

    #[arg(long, short = 'o')]
    pub out_dir: Option<PathBuf>,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub no_bloatware: bool,
}

pub fn parse_args() -> Cli {
    let mut cli = Cli::command();

    let before_help = if let Some(plat) = ProductPlatform::detect() {
        let os_version = get_os_version();
        let condition_vars = plat
            .get_condition_vars()
            .unwrap()
            .iter()
            .map(|(a, b)| {
                let EvalValue::String(s) = b else {
                    unreachable!()
                };
                format!("{}: {}", a, s)
            })
            .collect::<Vec<_>>()
            .join("\n ");

        format!(
            "TARGET:\n Platform: {}\n OSVersion: {}\n {}",
            plat.to_key(),
            os_version,
            condition_vars
        )
    } else {
        "TARGET: Unknown! Installation is unavailable.".to_string()
    };
    cli = cli.before_help(before_help);
    Cli::from_arg_matches(&cli.get_matches()).unwrap()
}
