use crate::core::errors::GenericError;
use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use std::collections::HashMap;

#[derive(Debug)]
pub struct PIMXPackage {
    pub type_: String,
    pub package_name: String,
    pub package_scheme: String,
    pub processor_family: Option<String>,
    pub condition: Option<String>,
    pub assets: Vec<Asset>,
    pub commands: Vec<Command>,
}

#[derive(Debug)]
pub struct Asset {
    pub source: String,
    pub target: String,
    pub recursive: bool,
    pub is_recursive_delete: bool,
    pub is_user_preferences: bool,
}

#[derive(Debug)]
pub struct RegistryCommand {
    pub path: String,
    pub name: Option<String>,
    pub type_: Option<String>,
    pub value: Option<String>,
    pub is_recursive_delete: bool,
    pub is_user_preferences: bool,
}

#[derive(Debug)]
pub struct FolderIconCommand {
    pub folder_path: String,
    pub icon_path: String,
}

#[derive(Debug)]
pub struct PermissionCommand {
    pub path: String,
    pub user: Option<String>,
    pub permission_value: String,
}

#[derive(Debug)]
pub struct RegisterApplicationCommand {
    pub path: String,
}

#[derive(Debug)]
pub struct InstallUserPrefCommand {
    pub path: String,
}

#[derive(Debug)]
pub struct OwnerCommand {
    pub path: String,
    pub user: String,
    pub group: String,
}

#[derive(Debug)]
pub struct RunProgramCommand {
    // run_in_user_mode: Option<bool>,
    pub path: String,
    pub arguments: Option<Vec<String>>,
    pub success_exit_codes: Option<Vec<i32>>,
}

#[derive(Debug)]
pub struct ShortcutCommand {
    pub target: String,
    pub directory: String,
    pub name_languages: HashMap<String, String>,
}

#[derive(Debug)]
pub enum Command {
    Registry(RegistryCommand), // windows only
    FolderIcon(FolderIconCommand),
    Permission(PermissionCommand),
    RegisterApplication(RegisterApplicationCommand), // possibly mac only...?
    InstallUserPref(InstallUserPrefCommand),
    Owner(OwnerCommand),
    RunProgram(RunProgramCommand),
    Shortcut(ShortcutCommand),
}

pub fn parse_pimx(xml: &str) -> Result<PIMXPackage, GenericError> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    let mut type_ = String::new();
    let mut package_name = String::new();
    let mut package_scheme = String::new();
    let mut processor_family = None;
    let mut condition = None;
    let mut assets = Vec::new();
    let mut commands = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => match e.name().as_ref() {
                b"Type" => type_ = reader.read_text(e.name()).unwrap().into_owned(),
                b"PackageName" => package_name = reader.read_text(e.name()).unwrap().into_owned(),
                b"ProcessorFamily" => {
                    processor_family = Some(reader.read_text(e.name()).unwrap().into_owned())
                }
                b"PackageScheme" => {
                    package_scheme = reader.read_text(e.name()).unwrap().into_owned()
                }
                b"Condition" => {
                    condition = Some(unescape(&reader.read_text(e.name())?)?.into_owned())
                }
                b"Assets" => assets = parse_assets(&mut reader)?,
                b"Commands" => commands = parse_commands(&mut reader)?,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(PIMXPackage {
        type_,
        package_name,
        package_scheme,
        processor_family,
        condition,
        assets,
        commands,
    })
}

fn parse_simple(
    reader: &mut Reader<&[u8]>,
    start_evt: &BytesStart,
) -> Result<(Hms, Hms), GenericError> {
    let mut children = HashMap::new();
    let mut attributes = HashMap::new();

    for attr in start_evt.attributes() {
        let attr = attr?;
        attributes.insert(
            String::from_utf8(attr.key.as_ref().to_vec())?,
            String::from_utf8(attr.value.to_vec())?,
        );
    }

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = String::from_utf8(e.name().as_ref().to_vec())?;
                let value = reader.read_text(e.name()).unwrap().into_owned();
                children.insert(name, value);
            }
            Event::End(e) if e.name().as_ref() == start_evt.name().as_ref() => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Ok((attributes, children))
}

type Hms = HashMap<String, String>;
type PRes<T> = Result<T, GenericError>;

fn parse_shortcut(reader: &mut Reader<&[u8]>) -> PRes<ShortcutCommand> {
    let mut buf = Vec::new();
    let mut target = String::new();
    let mut directory = String::new();
    let mut name_languages = HashMap::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => match e.name().as_ref() {
                b"Target" => target = reader.read_text(e.name()).unwrap().into_owned(),
                b"Directory" => directory = reader.read_text(e.name()).unwrap().into_owned(),
                b"Language" => {
                    let mut locale = None;
                    for attr in e.attributes() {
                        let attr = attr?;
                        if attr.key.as_ref() == b"locale" {
                            locale = Some(String::from_utf8(attr.value.to_vec())?);
                        }
                    }
                    if let Some(locale) = locale {
                        let value = reader.read_text(e.name()).unwrap().into_owned();
                        name_languages.insert(locale, value);
                    }
                }
                _ => {}
            },
            Event::End(e) if e.name().as_ref() == b"Shortcut" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(ShortcutCommand {
        target,
        directory,
        name_languages,
    })
}

fn extract_children_with_node_name(
    reader: &mut Reader<&[u8]>,
    parent_name: &[u8],
    node_name: &[u8],
) -> Option<Vec<String>> {
    let mut buf = Vec::new();
    let mut result = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).ok()? {
            Event::Start(e) if e.name().as_ref() == node_name => {
                result.push(reader.read_text(e.name()).unwrap().into_owned());
            }
            Event::End(e) if e.name().as_ref() == parent_name => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Some(result)
}

fn parse_install_commmand(reader: &mut Reader<&[u8]>) -> PRes<RunProgramCommand> {
    let mut buf = Vec::new();
    let mut path = String::new();
    let mut args = None;
    let mut success_exit_codes = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => match e.name().as_ref() {
                b"Path" => path = reader.read_text(e.name()).unwrap().into_owned(),
                b"Arguments" => {
                    args = extract_children_with_node_name(reader, b"Arguments", b"Argument")
                }
                b"SuccessExitCodes" => {
                    success_exit_codes =
                        extract_children_with_node_name(reader, b"SuccessExitCodes", b"ExitCode")
                            .map(|e| {
                                e.iter()
                                    .map(|c| c.parse::<i32>().unwrap())
                                    .collect::<Vec<_>>()
                            })
                }
                _ => {}
            },
            Event::End(e) if e.name().as_ref() == b"InstallCommand" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(RunProgramCommand {
        path,
        arguments: args,
        success_exit_codes,
    })
}

fn parse_commands(reader: &mut Reader<&[u8]>) -> PRes<Vec<Command>> {
    let mut result = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let r = match e.name().as_ref() {
                    b"Registry" => {
                        let (a, c) = parse_simple(reader, &e)?;
                        let is_recursive_delete =
                            a.get("isRecursiveDelete").is_some_and(|v| v == "true");
                        let is_user_preferences =
                            a.get("isUserPreferences").is_some_and(|v| v == "true");
                        let path = c.get("Path").unwrap().to_owned();
                        let name = c.get("Name").cloned();
                        let type_ = c.get("Type").cloned();
                        let value = c.get("Value").cloned();
                        Command::Registry(RegistryCommand {
                            path,
                            name,
                            type_,
                            value,
                            is_recursive_delete,
                            is_user_preferences,
                        })
                    }
                    b"FolderIcon" => {
                        let (a, c) = parse_simple(reader, &e)?;
                        Command::FolderIcon(FolderIconCommand {
                            folder_path: c.get("FolderPath").unwrap().to_owned(),
                            icon_path: c.get("IconPath").unwrap().to_owned(),
                        })
                    }
                    b"RegisterApplication" => {
                        let (a, c) = parse_simple(reader, &e)?;
                        Command::RegisterApplication(RegisterApplicationCommand {
                            path: c.get("Path").unwrap().to_owned(),
                        })
                    }
                    b"InstallUserPref" => {
                        let (a, c) = parse_simple(reader, &e)?;
                        Command::InstallUserPref(InstallUserPrefCommand {
                            path: c.get("Path").unwrap().to_owned(),
                        })
                    }
                    b"Owner" => {
                        let (a, c) = parse_simple(reader, &e)?;
                        Command::Owner(OwnerCommand {
                            path: c.get("Path").unwrap().to_owned(),
                            user: c.get("User").unwrap().to_owned(),
                            group: c.get("Group").unwrap().to_owned(),
                        })
                    }
                    b"Permission" => {
                        let (a, c) = parse_simple(reader, &e)?;
                        Command::Permission(PermissionCommand {
                            path: c.get("Path").unwrap().to_owned(),
                            user: c.get("User").cloned(),
                            permission_value: c.get("PermissionValue").unwrap().to_owned(),
                        })
                    }
                    b"Shortcut" => Command::Shortcut(parse_shortcut(reader)?),
                    b"InstallCommand" => Command::RunProgram(parse_install_commmand(reader)?),
                    // b"LocalizeFileName" => todo!("Implement LocalizeFileName"),
                    // b"LocalizeDisplayName" => todo!("Implement LocalizeDisplayName"),
                    _ => continue,
                };
                result.push(r);
            }
            Event::End(e) if e.name().as_ref() == b"Commands" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear()
    }

    Ok(result)
}

fn parse_assets(reader: &mut Reader<&[u8]>) -> Result<Vec<Asset>, GenericError> {
    let mut result = Vec::new();
    let mut buf = Vec::new();
    //<Asset
    // source="[StagingFolder]/Preference Settings"
    // target="[UserPreferences]/Adobe Photoshop 2025 Settings"
    // recursive="true"
    // isRecursiveDelete="true"
    // isUserPreferences="true"/>

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Empty(e) | Event::Start(e) if e.name().as_ref() == b"Asset" => {
                let mut source = String::new();
                let mut target = String::new();
                let mut recursive = true;
                let mut is_recursive_delete = false;
                let mut is_user_preferences = false;

                for attr in e.attributes() {
                    let attr = attr?;
                    match attr.key.as_ref() {
                        b"source" => source = String::from_utf8(attr.value.to_vec())?,
                        b"target" => target = String::from_utf8(attr.value.to_vec())?,
                        b"recursive" => recursive = attr.value.as_ref() == b"true",
                        b"isRecursiveDelete" => {
                            is_recursive_delete = attr.value.as_ref() == b"true"
                        }
                        b"isUserPreferences" => {
                            is_user_preferences = attr.value.as_ref() == b"true"
                        }
                        _ => {}
                    }
                }
                result.push(Asset {
                    source,
                    target,
                    recursive,
                    is_recursive_delete,
                    is_user_preferences,
                });
            }
            Event::End(e) if e.name().as_ref() == b"Assets" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::installer::pim::parse_pimx;

    #[test]
    fn test_pim_parser() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Package>
    <PackageName>AdobeColorCommonSetCMYK_1_0-mul</PackageName>
    <ProcessorFamily>32-bit</ProcessorFamily>
    <IsShared>true</IsShared>
    <Features>
        <Feature>
            <Name>ColorCommon</Name>
        </Feature>
    </Features>
    <Assets>
        <Asset source="[StagingFolder]/AdobeColorCommonSetCMYK_1_0-mul" target="[AdobeCommon]/Color" checkVersionBeforeInstall="true" isPermanent="true">
            <AssetType>Color</AssetType>
        </Asset>
    </Assets>
</Package>"#;

        let package = parse_pimx(&xml).unwrap();
        println!("{:#?}", package);
    }
}
