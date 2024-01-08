use nom::{
    branch::alt,
    bytes::complete::{is_a, is_not, tag, take},
    character::complete::{multispace0, multispace1},
    combinator::{all_consuming, peek, recognize},
    multi::many0,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

fn parser_escaped_identity<'a>(
    exclude_chars: &'a str,
) -> impl FnMut(&'a str) -> IResult<&'a str, &'a str> {
    move |raw: &str| {
        let mut f = alt((
            recognize(pair(is_a("\\"), take(1usize))),
            is_not(exclude_chars),
        ));

        let mut len = 0;
        let (remain, matched) = f(raw)?;
        let mut input = remain;
        len += matched.len();
        loop {
            match f(input) {
                Err(_) => break,
                Ok((remain, matched)) => {
                    input = remain;
                    len += matched.len();
                }
            }
        }
        Ok((&raw[len..], &raw[..len]))
    }
}

pub fn parser<'a>(raw: &'a str) -> anyhow::Result<(&'a str, Vec<&'a str>)> {
    match all_consuming(delimited(
        multispace0,
        pair(
            parser_escaped_identity(" \r\n\t\\"),
            many0(preceded(
                multispace1,
                alt((
                    recognize(pair(
                        peek(is_not("\"\'")),
                        parser_escaped_identity(" \r\n\t\\"),
                    )),
                    recognize(tuple((
                        tag("\""),
                        parser_escaped_identity("\\\""),
                        tag("\""),
                    ))),
                    recognize(tuple((tag("'"), parser_escaped_identity("\\'"), tag("'")))),
                )),
            )),
        ),
        multispace0,
    ))(raw)
    {
        Err(_) => Err(anyhow::anyhow!("parse failed")),
        Ok((_, result)) => Ok(result),
    }
}
