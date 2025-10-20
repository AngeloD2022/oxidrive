# oxidrive

The Adobe product installer, remade in Rust.

---

## About

Oxidrive's name is inspired by HyperDrive, which is Adobe's internal product installer mechanism.

To install products, it will download all required dependencies and install each according to its
product install manifest file (.PIMX). This file contains the instructions to install the product
on the host system.

## Usage

```shell
# For a wizard-like experience:
oxidrive wizard

# To immediately install a specific product (will use default language preferences):
oxidrive install ae

# To download packages that can be installed later or transferred and installed on a different system:
oxidrive download ps
oxidrive download au --os windows --arch arm -o ./installers
```

## Requirements

- Windows or macOS machine
- You will also need to meet the requirements for the product(s) you install.

## Features

- Downloader
  - [x] Dependency resolution
  - [x] Config-dependent package filtering
  - [x] Parallelized downloader
  - [ ] Signature validation

- Installer
  - [x] Path macro resolution
  - [ ] PIMX analysis
  - [ ] Installer Core

## Note

This is by no means a perfect replica of the official installer. 
Some edge cases almost certainly exist. If you encounter these things, please make an issue.

## License

This project is licensed under version 3 of the GNU Affero General Public License.