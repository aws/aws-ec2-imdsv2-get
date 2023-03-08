use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;

const IMDS_URL: &str = "169.254.169.254:80";

fn request(
    method: &str,
    path: &str,
    headers: HashMap<String, String>,
) -> std::io::Result<(u64, String)> {
    let mut socket = TcpStream::connect(IMDS_URL)?;

    let header = format!(
        "{} /{} HTTP/1.1\r\n{}\r\n\r\n",
        method,
        path,
        headers
            .iter()
            .map(|(i, x)| format!("{}: {}", i, x))
            .collect::<Vec<_>>()
            .join("\r\n")
    );
    socket.write(header.as_bytes())?;
    socket.flush()?;

    let mut text = String::new();
    socket
        .read_to_string(&mut text)
        .expect("failed to read response");

    // The text should be delimited by \r\n
    let segments: Vec<&str> = text.split("\r\n\r\n").collect();
    let response_headers = segments[0];

    let header_lines: Vec<&str> = response_headers.split("\r\n").collect();

    // The first line will contain the response type
    let response_status: Vec<&str> = header_lines[0].split_whitespace().collect();
    // The important part here is the part 2 status code
    let status_code = response_status[1];

    let response_text = segments[1];
    Ok((
        status_code.parse::<u64>().unwrap(),
        response_text.to_string(),
    ))
}

fn imdsv2_handle(headers: &mut HashMap<String, String>) -> std::io::Result<()> {
    let (status, token) = request(
        "PUT",
        "latest/api/token",
        HashMap::from([(
            "X-aws-ec2-metadata-token-ttl-seconds".to_string(),
            "1".to_string(),
        )]),
    )?;

    if status != 200 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "failed to fetch token",
        ));
    }

    headers.insert("X-aws-ec2-metadata-token".to_string(), token);
    Ok(())
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("imds_get <path>");
        std::process::exit(1);
    }
    let sub_uri = args[1].clone();

    // First let's check if imdsv2 is enabled
    let imdsv2 = match request("GET", "/", HashMap::new()) {
        Ok((status, _)) => status == 401,
        Err(e) => {
            if e.to_string().to_lowercase().contains("Unauthorized") {
                true
            } else {
                false
            }
        }
    };

    let mut headers: HashMap<String, String> = HashMap::new();

    if imdsv2 {
        imdsv2_handle(&mut headers)?;
    }

    let (_, text) = request("GET", sub_uri.as_str(), headers)?;

    println!("{}", text);
    Ok(())
}
