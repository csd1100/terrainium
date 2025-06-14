use clap::builder::styling::{AnsiColor, Color, Style};

pub fn colored(string: &str, color: AnsiColor) -> String {
    let style = Style::new().fg_color(Some(Color::Ansi(color)));
    format!("{style}{string}{style:#}")
}

pub fn heading(string: &str) -> String {
    let heading = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlue)));
    format!("{heading}{:width$}{heading:#}", string, width = 30)
}

pub fn sub_heading(string: &str) -> String {
    let sub_heading = Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::BrightBlue)))
        .italic()
        .underline();
    format!("{sub_heading}{string}{sub_heading:#}")
}

pub fn value(string: &str) -> String {
    let value = Style::new().bold();
    format!("{value}{string}{value:#}")
}

pub fn sub_value(string: &str) -> String {
    let sub_value = Style::new().bold().italic();
    format!("{sub_value}{string}{sub_value:#}")
}

pub fn success(string: &str) -> String {
    let success = Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Green)))
        .bold();
    format!("{success}{string}{success:#}")
}

pub fn warning(string: &str) -> String {
    let warning = Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
        .bold();
    format!("{warning}{string}{warning:#}")
}

pub fn error(string: &str) -> String {
    let error = Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Red)))
        .bold();
    format!("{error}{string}{error:#}")
}
