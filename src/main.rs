use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::net::{SocketAddr, SocketAddrV4, TcpStream};
use std::str::FromStr;
use std::time::Duration;

const IMDS_URL_V4: &str = "169.254.169.254:80";
const IMDS_URL_V6: &str = "[fd00:ec2::254]:80";

fn get_imds_url() -> String {
    let ipv4: SocketAddr = SocketAddr::V4(SocketAddrV4::from_str(IMDS_URL_V4).unwrap());
    if TcpStream::connect_timeout(&ipv4, Duration::from_millis(100)).is_ok() {
        IMDS_URL_V4.to_string()
    } else {
        IMDS_URL_V6.to_string()
    }
}

fn request(
    method: &str,
    path: &str,
    headers: HashMap<String, String>,
) -> std::io::Result<(u64, Vec<u8>)> {
    let imds_url = get_imds_url();
    let mut socket = TcpStream::connect(imds_url)?;

    let header = format!(
        "{} /{} HTTP/1.1\r\n{}\r\n",
        method,
        path,
        headers
            .iter()
            .map(|(i, x)| format!("{}: {}\r\n", i, x))
            .collect::<Vec<_>>()
            .join("")
    );
    socket.write_all(header.as_bytes())?;
    socket.flush()?;

    let mut buf = Vec::new();

    socket
        .read_to_end(&mut buf)
        .expect("failed to read response");

    // We now want to extract the headers, we get each header line by ites delim "\r\n"
    let mut header_lines: Vec<String> = Vec::new();
    let mut header_buf: Vec<u8> = Vec::new();
    let mut index = 0;

    while index < buf.len() {
        if index <= buf.len() - 2 && buf[index] == b'\r' && buf[index + 1] == b'\n' {
            if header_buf.is_empty() {
                // We are at the end of our headers
                index += 2;
                break;
            } else {
                let header = String::from_utf8(header_buf).expect("failed to parse header");
                header_lines.push(header.clone());
                header_buf = Vec::new();
                index += 2;
            }
        } else {
            header_buf.push(buf[index]);
            index += 1;
        }
    }

    // The first line will contain the response type
    let response_status: Vec<&str> = header_lines[0].split_whitespace().collect();
    // The important part here is the part 2 status code
    let status_code = response_status[1];
    let data = buf[index..].to_vec();

    Ok((status_code.parse::<u64>().unwrap(), data))
}

fn imdsv2_handle(headers: &mut HashMap<String, String>) -> std::io::Result<()> {
    let (status, token_bytes) = request(
        "PUT",
        "latest/api/token",
        HashMap::from([(
            "X-aws-ec2-metadata-token-ttl-seconds".to_string(),
            "1".to_string(),
        )]),
    )?;
    let token = String::from_utf8(token_bytes).expect("failed to parse imdsv2 token");

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
        Err(e) => e.to_string().to_lowercase().contains("Unauthorized"),
    };

    let mut headers: HashMap<String, String> = HashMap::new();

    if imdsv2 {
        imdsv2_handle(&mut headers)?;
    }

    let (status_code, bytes) = request("GET", sub_uri.as_str(), headers)?;
    if status_code == 404 {
        std::process::exit(1);
    }

    let mut stdout = std::io::stdout();

    stdout
        .write_all(bytes.as_slice())
        .expect("failed to write imdsv2 data to output");
    stdout.flush().expect("failed to flush std output");
    Ok(())
}
