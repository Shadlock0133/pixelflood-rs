use crate::{
    error::{MyError, MyResult, ParseColorError},
    Pos,
};
use std::{fmt, io, marker::Unpin, str::FromStr};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

const HELP: &str = "Commands:
    - HELP - Show this message
    - SIZE - Get canvas size
    - PX [x] [y] - Get color from canvas at this position
    - PX [x] [y] [color] - Paint this color on canvas at this position
";

// Stores pixel in ARGB format
#[derive(Debug, PartialEq, Eq)]
pub struct Color(pub u32);

impl FromStr for Color {
    type Err = ParseColorError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 6 {
            // rrggbb
            u32::from_str_radix(s, 16)
                .map(|x| x | 0xff_00_00_00)
                .map(Color)
                .map_err(|_| ParseColorError::new(s))
        } else if s.len() == 8 {
            // rrggbbaa -> aarrggbb
            u32::from_str_radix(s, 16)
                .map(|color| {
                    let [r, g, b, alpha] = u32::to_be_bytes(color);
                    u32::from_be_bytes([alpha, r, g, b])
                })
                .map(Color)
                .map_err(|_| ParseColorError::new(s))
        } else {
            Err(ParseColorError::new(s))
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:06x}", self.0 & 0x00_ff_ff_ff)
    }
}

#[derive(Debug)]
pub enum Command {
    Help,
    Size,
    SetPx(Pos, Color),
    GetPx(Pos),
}

pub enum Response {
    Help,
    Size(Pos),
    Px(Pos, Color),
}

pub async fn parse_command<B: Unpin + AsyncBufReadExt>(mut r: B) -> MyResult<Command> {
    let mut buf = String::new();
    while buf.trim().is_empty() {
        r.read_line(&mut buf).await?;
    }
    let line = buf.trim();
    if line == "HELP" {
        Ok(Command::Help)
    } else if line == "SIZE" {
        Ok(Command::Size)
    } else if line.starts_with("PX ") {
        Ok(match parse_px(line).ok_or(io::ErrorKind::InvalidInput)? {
            (pos, Some(color)) => Command::SetPx(pos, color),
            (pos, None) => Command::GetPx(pos),
        })
    } else {
        Err(MyError::UnknownCommand(buf))
    }
}

pub async fn write_response<W: Unpin + AsyncWriteExt>(mut w: W, resp: Response) -> MyResult<()> {
    let msg = match resp {
        Response::Help => HELP.to_string(),
        Response::Size((x, y)) => format!("SIZE {} {}", x, y),
        Response::Px((x, y), color) => format!("PX {} {} {}", x, y, color),
    };
    Ok(w.write_all(msg.as_bytes()).await?)
}

fn parse_px(s: &str) -> Option<(Pos, Option<Color>)> {
    let mut args = s.split(' ');
    args.next().filter(|&arg| arg == "PX")?;
    let x = args.next()?.parse::<usize>().ok()?;
    let y = args.next()?.parse::<usize>().ok()?;
    match args.next() {
        Some(color) => Some(((x, y), Some(color.parse().ok()?))),
        None => Some(((x, y), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_px() {
        let tests = [
            (
                "PX 1 2 112233",
                Some(((1usize, 2usize), Some(Color(0xff112233u32)))),
            ),
            (
                "PX 1 2 11223344",
                Some(((1usize, 2usize), Some(Color(0x44112233u32)))),
            ),
            ("PX 1 2", Some(((1usize, 2usize), None))),
            ("PX 1 2 00", None),
            ("PX 1", None),
        ];
        for (msg, exp) in tests.iter() {
            assert_eq!(&parse_px(msg), exp, "msg: {}", msg);
        }
    }
}
