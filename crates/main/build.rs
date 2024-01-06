use anyhow::{anyhow, Result};

fn get_version_string() -> Result<String> {
    let nato_alphabet: Vec<&str> = vec![
        "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india",
        "juliet", "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra",
        "tango", "uniform", "victor", "whiskey", "xray", "yankee", "zulu",
    ];
    let child = std::process::Command::new("git")
        .args([
            "log",
            "--pretty=format:%cd",
            "--date=format:%Y.%m.%d",
            "--max-count=26",
        ])
        .output()?;
    let stdout = String::from_utf8(child.stdout)?;
    let lines: Vec<_> = stdout.lines().filter(|x| x.trim().len() > 0).collect();
    if lines.len() == 0 {
        Err(anyhow!("version not found"))
    } else {
        let date = lines[0];
        let mut cnt = 0;
        for item in lines {
            if item == date {
                cnt += 1;
            } else {
                break;
            }
        }
        Ok(format!("{}.{}", date, nato_alphabet[cnt - 1]))
    }
}

fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let output = std::path::Path::new(&out_dir).join("version");
    let version = match get_version_string() {
        Ok(version) => version,
        Err(err) => {
            eprintln!("get version failed: {}", err);
            format!("{}.unknown", chrono::Local::now().format("%Y.%m.%d"))
        }
    };

    println!("build version: {}", version);
    std::fs::write(output, version).unwrap();
}
