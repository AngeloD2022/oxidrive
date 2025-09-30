use anyhow::Result;
use console::Style;
use textwrap::fill;

pub fn show() -> Result<()> {
    use assgrave;
    let name = "assgrave";
    let description = "A tool for installing and managing Adobe Software";
    let version = assgrave::version();

    let header_style = Style::new().bold();
    let dim_style = Style::new().dim();
    let section_style = Style::new().bold();

    println!(
        "{}",
        header_style.apply_to(format!("{name} — {description}"))
    );
    println!("{}", dim_style.apply_to(format!("Version: {version}")));
    println!();

    println!("{}", section_style.apply_to("USAGE:"));
    println!("  {} <COMMAND> [OPTIONS]\n", name);

    println!("{}", section_style.apply_to("AVAILABLE COMMANDS:"));
    const CMD_COL_WIDTH: usize = 12;
    let commands = &[("show", "show this help / version info")];

    for (cmd, desc) in *commands {
        let wrapped = fill(desc, 60);
        let mut lines = wrapped.lines();
        if let Some(first) = lines.next() {
            println!("  {:<width$} {}", cmd, first, width = CMD_COL_WIDTH);
        }
        for line in lines {
            println!("  {:<width$} {}", "", line, width = CMD_COL_WIDTH);
        }
    }

    println!();
    println!("{}", section_style.apply_to("FLAGS:"));
    println!("  -h, --help     Prints help information");
    println!("  -V, --version  Prints version information\n");
    println!(
        "{}",
        dim_style.apply_to("For more details: <command> --help")
    );

    Ok(())
}
