# oxidrive

The Adobe product installer, remade in Rust.

> [!CAUTION]
> This is experimental software, please use it at your own risk. I am not responsible for any damage caused.

---

## About

Oxidrive's name is inspired by HyperDrive, which is Adobe's internal product installer mechanism.

It is meant to be a robust alternative to the bloated electron-based Creative Cloud app.

To install products, it will download all required dependencies and install each according to its
product install manifest file (.PIMX). This file contains the instructions to install the product
on the host system.

> [!NOTE]
> This project attempts to reimplement the manifest-driven installs executed by Adobe HyperDrive.
> **It does not bypass, modify, disable, or interfere with Adobe licensing or DRM.**
> A valid license is required to use any installed software.
>
> This project is not affiliated with or endorsed by Adobe.

### Supports:

🚧 Installation

### Not (currently) supported:
❌ Uninstallation \
❌ Updating

## Usage

```shell
# To install a specific product (will use default language preferences):
oxidrive install ae
oxidrive install photoshop
# or use oxidrive install --help for more information
```

## Requirements

- macOS machine (or Windows soon™️)
- You will also need to meet the requirements for the product(s) you install.

## Features

- Downloader
  - [x] Dependency resolution (currently somewhat unstable)
  - [x] Config-dependent package filtering
  - [x] Parallelized downloader
  - [ ] Signature validation

- Installer
  - [x] Path macro resolution
  - [x] PIMX analysis
  - Installer
    - [x] Main logic
    - [ ] Installation state database
    - 🚧 MacOS Backend (functional, WIP)
    - ❌ Windows Backend (currently not functional)


## License

This project is licensed under version 3 of the GNU Affero General Public License.
