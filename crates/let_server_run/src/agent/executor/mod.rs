mod arg_parser;

pub async fn execute(message: &str) -> Result<String, anyhow::Error> {
    match arg_parser::parser(message) {
        Ok((cmd, args)) => Ok(format!("{}: {:?}", cmd, args)),
        Err(err) => Err(err),
    }
}
