use reqwest::Error;
use rumqttc::{Client, MqttOptions, QoS};
use std::{time::Duration, process::exit};
use serde::Deserialize;
use bytes::Bytes;
use dotenvy::dotenv;
use std::env;

// {\"Time\":\"2024-02-03T23:16:58\",\"RfReceived\":{\"Data\":\"0xE0F118\",\"Bits\":24,\"Protocol\":1,\"Pulse\":200}}
#[derive(Deserialize)]
struct DataPoint {
    #[serde(rename(deserialize = "Time"))]
    #[allow(dead_code)]
    time: String,
    #[serde(rename(deserialize = "RfReceived"))]
    rf_received: RfData,
}

#[derive(Deserialize)]
struct RfData {
    #[serde(rename(deserialize = "Data"))]
    data: String,
    #[serde(rename(deserialize = "Bits"))]
    #[allow(dead_code)]
    bits: serde_json::Number,
    #[serde(rename(deserialize = "Protocol"))]
    #[allow(dead_code)]
    protocol: serde_json::Number,
    #[serde(rename(deserialize = "Pulse"))]
    #[allow(dead_code)]
    pulse: serde_json::Number,
}

struct MqttConn {
    id: Option<String>,
    host: Option<String>,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    topic: Option<String>,
}

struct WebhookData {
    discord_url: Option<String>,
    camera_url: Option<String>,
    message: Option<String>,
}

fn main() {
    match dotenv() {
        Err(e) => println!("INFO: .env file was not found or could not be loaded. {:?}", e),
        Ok(_) => (),
    }
    let mut mqtt_conn = MqttConn {
        id: None,
        host: None,
        port: 1883,
        username: None,
        password: None,
        topic: None,
    };
    let mut webhook_data = WebhookData {
        discord_url: None,
        camera_url: None,
        message: None,
    };
    let mut rf_code: Option<String> = None;
    for (name, value) in env::vars() {
        match name.as_str() {
            "MQTT_ID" => mqtt_conn.id = Some(value),
            "MQTT_HOST" => mqtt_conn.host = Some(value),
            "MQTT_PORT" => mqtt_conn.port = value.parse::<u16>().expect("Port should be a number!"),
            "MQTT_USERNAME" => mqtt_conn.username = Some(value),
            "MQTT_PASSWORD" => mqtt_conn.password = Some(value),
            "MQTT_TOPIC" => mqtt_conn.topic = Some(value),
            "DISCORD_URL" => webhook_data.discord_url = Some(value),
            "CAMERA_URL" => webhook_data.camera_url = Some(value),
            "DISCORD_MESSAGE" => webhook_data.message = Some(value),
            "RF_CODE" => rf_code = Some(value),
            _ => (),
        }
    }
    let discord_url = match webhook_data.discord_url {
        Some(d) => d,
        None => {
            println!("ERROR: You are missing the DISCORD_URL!");
            exit(2);
        }
    };
    let (camera_url, message) = match (webhook_data.camera_url, webhook_data.message) {
        (Some(c), Some(m)) => (c, m),
        (_, _) => {
            println!("ERROR: You need to provide either a CAMERA_URL os a DISCORD_MESSAGE");
            exit(3);
        }
    };
    let mut mqttoptions: MqttOptions;
    match (mqtt_conn.id, mqtt_conn.host) {
        (Some(id), Some(host)) => mqttoptions = MqttOptions::new(id, host, mqtt_conn.port),
        (_, _) => {
            println!("ERROR: You are either missing MQTT_ID, MQTT_HOST for the MQTT connection!");
            exit(4);
        }
    }
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    // rust_analyzer warned that using && here is still unstable... maybe one day...
    if let Some(password) = mqtt_conn.password {
        if let Some(username) = mqtt_conn.username {
            mqttoptions.set_credentials(username, password);
        }
    }

    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    let topic = match mqtt_conn.topic {
        Some(s) => s,
        None => {
            println!("ERROR: The MQTT_TOPIC is missing");
            exit(5);
        },
    };
    match client.subscribe(topic, QoS::AtMostOnce) {
        Ok(_) => {},
        Err(e) =>{
            println!("ERROR: {:?}", e);
            exit(6);
        },
    }
    
    println!("INFO: Connected to the mqtt server successfully");

    // Iterate to poll the eventloop for connection progress
    for _ in connection.iter().filter_map(|n| {
        match n {
            Ok(e) => Some(e),
            Err(_) => None,
        }
    }).filter_map(|m| {
        match m {
            rumqttc::Event::Outgoing(_) => None,
            rumqttc::Event::Incoming(e) => Some(e),
        }
    }).filter_map(|i| {
        match i {
            rumqttc::Packet::Publish(e) => Some(e),
            _ => None,
        }
    }).filter_map(|notification| {
        match serde_json::from_slice::<DataPoint>(&notification.payload) {
            Ok(d) => Some(d),
            Err(e) => {
                println!("ERROR: {:?}", e);
                None},
        }
    }).filter(|d| {
        match (&rf_code, d.rf_received.data.as_str()) {
            (None, _) => true,
            (Some(c), received_code) => c == received_code,
        }
    }) {
        let client = reqwest::blocking::Client::new();
        let pic = match get_picture(camera_url.as_str(), &client) {
            Err(e) => {
                println!("ERROR: {:?}", e);
                continue;
            },
            Ok(p) => p,
        };
        let pic = pic;
        let res = trigger_hook(discord_url.as_str(), &client, message.clone(), pic);
        match res {
            Ok(s) => {
                match s {
                    reqwest::StatusCode::OK | reqwest::StatusCode::NO_CONTENT => (),
                    _ => println!("{:?}", s),
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }
}

fn get_picture(url: &str, client: &reqwest::blocking::Client) -> Result<Bytes, Error> {
    let req = client.get(url).send()?;
    req.bytes()
}

fn trigger_hook(url: &str, client: &reqwest::blocking::Client, message: String, pic: Bytes) -> Result<reqwest::StatusCode, Error>{
    let pic = reqwest::blocking::multipart::Part::bytes(pic.to_vec())
        .file_name("files.jpg")
        .mime_str("image/jpeg")?;
    let multipart = reqwest::blocking::multipart::Form::new()
        .text("content", message)
        .part("files[0]", pic);
    let status = client
        .post(url)
        .multipart(multipart)
        .send()?
        .status();
    return Ok(status)
}
