use std::env;
use reqwest::{Client, StatusCode};

const IMDS_URL: &str = "http://169.254.169.254/";

async fn imdsv2_handle(client: &reqwest::Client, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let token_response: reqwest::Response = client.put(format!("{}latest/api/token", IMDS_URL))
        .header("X-aws-ec2-metadata-token-ttl-seconds", "1")
        .send()
        .await
        .expect("failed to fetch imdsv2 token");

    let token = token_response.text().await
        .expect("failed to parse token response");

    request.header("X-aws-ec2-metadata-token", token)
}

#[async_std::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("imds_get <path>");
    }
    let sub_uri = args[1].clone();
    let client = Client::new();

    // First let's check if imdsv2 is enabled
    let imdsv2 = match client.get(IMDS_URL)
        .send()
        .await {
        Ok(data) =>
            data.status() == StatusCode::from_u16(401).unwrap()
        ,
        Err(e) => match e.status() {
            Some(code) =>
                code == StatusCode::from_u16(401).unwrap()
            ,
            None => true
        }
    };

    let mut request = client.get(format!("{}{}", IMDS_URL, sub_uri));
    if imdsv2 {
        request = imdsv2_handle(&client, request).await;
    }
    let response = request.send().await.expect("imds request failed");

    println!("{}", response.text().await.expect("failed to read response from imds"));
}