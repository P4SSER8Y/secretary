pub async fn execute(_alias: &str, args: &Vec<&str>, _context: &str) -> Result<String, anyhow::Error> {
    if args.len() > 0 {
        Ok(args.join(" "))
    } else {
        Err(anyhow::anyhow!("What?"))
    }
}
