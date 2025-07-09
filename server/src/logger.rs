use env_logger::fmt::{Color, Style, StyledValue};
use log::Level;
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub fn init(level: Option<&str>) {
    let env = env_logger::Env::default().default_filter_or(level.unwrap_or("INFO"));
    let mut build = env_logger::builder();
    build.parse_env(env);
    build.format(|buffer, record| {
        use std::io::Write;
        static LAST_TARGET: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::from("")));

        let mut last = LAST_TARGET.lock().unwrap();
        let target = record.target();
        let target = if last.eq(target) {
            "|"
        } else {
            target.clone_into(&mut last);
            target
        };
        let target_width = max_target_width(target.len());
        let mut style = buffer.style();
        let target = style.set_color(Color::White).set_bold(true).value(target);

        let mut style = buffer.style();
        let level = colored_level(&mut style, record.level());

        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f");
        writeln!(
            buffer,
            "[{}] {} {: <width$} > {}",
            time,
            level,
            target,
            record.args(),
            width = target_width
        )
    });
    build.init();
}

fn max_target_width(width: usize) -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static MAX_TARGET_WIDTH: AtomicUsize = AtomicUsize::new(0);
    let max = MAX_TARGET_WIDTH.load(Ordering::Relaxed);
    if max < width {
        MAX_TARGET_WIDTH.store(width, Ordering::Relaxed);
        width
    } else {
        max
    }
}

fn colored_level<'a>(style: &'a mut Style, level: Level) -> StyledValue<'a, &'static str> {
    match level {
        Level::Trace => style.set_color(Color::Magenta).value("TRACE"),
        Level::Debug => style.set_color(Color::Blue).value("DEBUG"),
        Level::Info => style.set_color(Color::Green).value("INFO "),
        Level::Warn => style.set_color(Color::Yellow).set_bold(true).value("WARN "),
        Level::Error => style.set_color(Color::Red).set_bold(true).value("ERROR"),
    }
}
