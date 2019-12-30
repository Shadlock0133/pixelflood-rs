use minifb::{Window, WindowOptions};
use std::{io, net::Ipv4Addr, sync::Arc, time::Duration};
use tokio::{
    io::BufReader,
    net::{TcpListener, TcpStream},
    stream::StreamExt,
    sync::Mutex,
    task,
    time::timeout,
};

mod protocol;
pub mod error;

use protocol::{Color, Command, Response, write_response, parse_command};
use error::{MyError, MyResult};

const SIZE: Pos = (640, 480);
const TIMEOUT: Duration = Duration::from_secs(1);

pub type Pos = (usize, usize);

fn mix(x: u8, y: u8, a: u8) -> u8 {
    let a = (a as f32 / u8::max_value() as f32).min(1.).max(0.);
    (x as f32 * (1. - a) + y as f32 * a).min(u8::max_value() as f32).max(0.) as u8
}

fn mix_in_place(x: &mut u32, y: u32) {
    let [a, yr, yg, yb] = u32::to_be_bytes(y);
    let [_, xr, xg, xb] = u32::to_be_bytes(*x);
    *x = u32::from_be_bytes([0, mix(xr, yr, a), mix(xg, yg, a), mix(xb, yb, a)]);
}

async fn handle_client(mut stream: TcpStream, buffer: Arc<Mutex<Vec<u32>>>) -> MyResult<()> {
    let (read, mut write) = stream.split();
    let mut read = BufReader::new(read);
    while let Ok(command) = timeout(TIMEOUT, parse_command(&mut read)).await {
        match command {
            Ok(Command::Help) =>
                write_response(&mut write, Response::Help).await?,
            Ok(Command::Size) =>
                write_response(&mut write, Response::Size((SIZE.0, SIZE.1))).await?,
            Ok(Command::GetPx((x, y))) => {
                if x >= SIZE.0 || y >= SIZE.1 {
                    return Err(io::ErrorKind::InvalidInput.into());
                }
                let color = buffer.lock().await[y * SIZE.0 + x];
                write_response(&mut write, Response::Px((x, y), Color(color))).await?;
            }
            Ok(Command::SetPx((x, y), Color(color))) => {
                if x >= SIZE.0 || y >= SIZE.1 {
                    return Err(MyError::GetPxOutside((x, y)));
                }
                mix_in_place(&mut buffer.lock().await[y * SIZE.0 + x], color);
            }
            Err(e) => eprintln!("Got error: {}", e),
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let mut buffer = vec![0u32; SIZE.0 * SIZE.1];
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            buffer[y * SIZE.0 + x] = (x | y << 6) as u32;
        }
    }
    let buffer = Arc::new(Mutex::new(buffer));
    let buffer2 = buffer.clone();

    task::spawn(async move {
        let mut listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 5545)).await.unwrap();
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            eprintln!("Got connection");
            if let Ok(stream) = stream {
                let buffer2 = buffer2.clone();
                task::spawn(async move {
                    match handle_client(stream, buffer2).await {
                        Ok(()) => eprintln!("Connection ended"),
                        Err(e) => eprintln!("Connection error: {}", e),
                    }
                });
            }
        }
    });
    
    // task::spawn_blocking(|| {
        let mut window = Window::new("Pixel Flood", SIZE.0, SIZE.1, WindowOptions::default()).unwrap();
        while window.is_open() {
            let buffer = buffer.lock().await;
            window.update_with_buffer(&buffer, SIZE.0, SIZE.1).unwrap();
        }
    // });
}
