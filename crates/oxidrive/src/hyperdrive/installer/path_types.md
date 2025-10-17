
## MacOS
### From HDPIM.dylib @ `FolderResolver::ExpandPathKey`
| Path Macro              | API Parameters                                     | Expanded Result                           |
|-------------------------|----------------------------------------------------|-------------------------------------------|
| `AdobeProgramFiles`     | `NSLocalDomainMask, NSApplicationDirectory`        | `/Applications`                           | 
| `ProgramFiles`          | ^                                                  | ^                                         |
| `Utilities`             | ^                                                  | ^ + `Utilities`                           |
| `Common`                | `NSLocalDomainMask, NSApplicationSupportDirectory` | `/Library/Application Support`            |
| `SharedApplicationData` | ^                                                  | ^                                         |
| `_OOBEHome`             | ^                                                  | ^                                         |
| `AdobeCommon`           | ^                                                  | `/Library/Application Support` + `/Adobe` |
| `FontsFolder`           | `NSLocalDomainMask, NSLibraryDirectory`            | `/Library` + `/Fonts`                     |
| `Library`               | ^                                                  | `/Library`                                |
| `LibraryPreferences`    | ^                                                  | `/Library` + `/Preferences`               |
| `ScriptingAdditions`    | ^                                                  | `/Library` + `/ScriptingAdditions`        |
| `InternetPlugins`       | ^                                                  | `/Library` + `/Internet Plug-Ins`         |
| `ColorSyncProfiles`     | ^                                                  | `/Library` + `/ColorSync` + `/Profiles`   |
| `UserInternetPlugins`   | `NSUserDomainMask, NSLibraryDirectory`             | `~/Library` + `/Internet Plug-Ins`        |
| `UserPreferences`       | ^                                                  | `~/Library` + `/Preferences`              |
| `UserCommon`            | `NSUserDomainMask, NSApplicationSupportDirectory`  | `~/Library/Application Support`           |
| `UserDocuments`         | `NSUserDomainMask, NSDocumentDirectory`            | `~/Documents`                             |
| `UserHome`              | `NSHomeDirectory()`                                | `~`                                       |
| `UserDesktop`           | `NSUserDomainMask, NSDesktopDirectory`             | `~/Desktop`                               |
| `SharedDocuments`       | -                                                  | -                                         |


# Windows
### From HDPIM.dll @ `sub_1011f0a0`

> API Function: `SHGetFolderPathW` parameter 2

| Path Macro              | API Parameters                                                 | Expanded Result |
|-------------------------|----------------------------------------------------------------|-----------------|
| `FontsFolder`           | CSIDL_FONTS                                                    |                 |
| `Common`                | CSIDL_PROGRAM_FILES_COMMON                                     |                 |
| `ProgramFiles`          | CSIDL_PROGRAM_FILES                                            |                 |
| `SharedApplicationData` | CSIDL_COMMON_APPDATA                                           |                 |
| `SharedDocuments`       | CSIDL_COMMON_DOCUMENTS                                         |                 |
| `StartMenu`             | CSIDL_COMMON_PROGRAMS                                          |                 |
| `System32Folder`        | CSIDL_SYSTEM                                                   |                 |
| `System`                | ^                                                              |                 |
| `UserHome`              | CSIDL_PROFILE                                                  |                 |
| `UserDocuments`         | CSIDL_PERSONAL                                                 |                 |
| `UserRoamingAppData`    | CSIDL_APPDATA                                                  |                 |
| `UserLocalAppData`      | CSIDL_LOCAL_APPDATA                                            |                 |
| `ProgramFilesx86`       | CSIDL_PROGRAM_FILESX86                                         |                 |
| `SystemX86`             | CSIDL_SYSTEMX86                                                |                 |
| `CommonX86`             | CSIDL_PROGRAM_FILES_COMMONX86                                  |                 |
| `Temp`                  | Create a temporary directory - `GetTempPathW(0x104, &buffer);` |                 |