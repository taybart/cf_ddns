use reqwest;
use std::{
    env,
    collections::HashMap,
};
use serde::{Deserialize, Serialize};
use tokio;

const URL: &str = "https://api.cloudflare.com/client/v4";

#[derive(Serialize, Deserialize, Debug)]
struct CfMessage {
    code: i32,
    message: String,
    r#type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AuthResponse {
    success: bool,
    errors: Vec<String>,
    messages: Vec<CfMessage>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DNSRecord {
    id: String,
    zone_id: String,
    name: String,
    zone_name: String,
    r#type: String,
    content: String,
    // proxiable:bool,
    proxied: bool,
    ttl: i32,
    // created_on: String,
    // modified_on: String
}

#[derive(Serialize, Deserialize, Debug)]
struct ZoneInfo {
    success: bool,
    errors: Vec<String>,
    messages: Vec<CfMessage>,
    result: Vec<DNSRecord>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ZoneInfoResponse {
    success: bool,
    errors: Vec<String>,
    messages: Vec<CfMessage>,
    result: DNSRecord,
}

async fn check_auth(client: &reqwest::Client) -> Result<bool, Box<dyn std::error::Error>> {
    let res = client
        .get(format!("{}/user/tokens/verify", URL))
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", env::var("API_KEY").unwrap()))
        .send()
        .await?
        .json::<AuthResponse>()
        .await?;

    Ok(res.messages[0].code != 10000)
}

async fn get_zone(client: &reqwest::Client) -> Result<ZoneInfo, reqwest::Error> {
    let res = client
        .get(format!(
                "{}/zones/82ced3b4f8270a500730e944e2beb69f/dns_records",
                URL
                ))
        .query(&[("type", "A")])
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", env::var("API_KEY").unwrap()))
        .send()
        .await?
        .json::<ZoneInfo>()
        .await?;

    Ok(res)
}

async fn get_current_ip(client: &reqwest::Client) -> Result<String, reqwest::Error> {
    let res = client
        .get("https://httpbin.org/ip")
        .send()
        .await?.json::<HashMap<String,String>>().await?;
    // let origin = res.get("origin");
    Ok(res.get("origin").unwrap().to_string())
}

async fn set_zone_ip(client: &reqwest::Client, mut dns: DNSRecord) -> Result<bool, reqwest::Error> {

    let ip = get_current_ip(client).await.unwrap();

    if ip == dns.content {
        return Ok(false)
    }
    dns.content = ip;

    let res = client
        .put(format!(
                "{}/zones/{}/dns_records/{}",
                URL, dns.zone_id, dns.id
                ))
        .query(&[("type", "A")])
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {:?}", env::var("API_KEY")))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&dns)
        .send()
        .await?
        .json::<ZoneInfoResponse>()
        .await?;

    println!("{:#?}",res);
    Ok(true)

}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    match check_auth(&client).await {
        Ok(authd) => {
            if authd {
                println!("not authorized");
            }
            let zoneinfo = get_zone(&client).await;
            for z in zoneinfo.unwrap().result {
                if z.name == "wg.bbl.systems" {
                    match set_zone_ip(&client, z).await {
                        Ok(updated) => {
                            println!("updated {}", updated);
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
        }
        Err(e) => eprintln!("not authorized: {}", e),
    }
    Ok(())
}
