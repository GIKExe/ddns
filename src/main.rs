use std::{
    error::Error,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::Arc,
    thread,
};

use config::Config;
use regru::replace_record;

mod config;
mod regru;

macro_rules! RESPONSE {
    ($data:expr) => {
        format!(
            "HTTP/1.1 200 OK\r
Content-Type: text/plain\r
Content-Length: {}\r
\r
{}",
            $data.len(),
            $data
        )
    };
}

fn read_request(text: &str) -> Option<(String, String, String)> {
    let (http_parts, _) = text.split_once("\r\n\r\n")?;
    let mut http_parts = http_parts.split("\r\n");

    let http_line = http_parts.next()?;
    let (_, query) = http_line.split_once("?")?;
    let mut domain: Option<&str> = None;
    let mut ip: Option<&str> = None;
    for q in query.split('&') {
        let (name, value) = q.split_once('=')?;
        match name {
            "hostname" => domain = Some(value),
            "myip" => ip = Some(value),
            _ => continue,
        }
    }

    let mut header_auth: Option<&str> = None;
    for header in http_parts {
        let (name, value) = header.split_once(": ")?;
        match name {
            "Authorization" => {
                let (_, x) = value.split_once(" ")?;
                header_auth = Some(x);
            }
            _ => continue,
        }
    }

    Some((
        ip?.to_string(),
        header_auth?.to_string(),
        domain?.to_string(),
    ))
}

/// stream, config -> (domain, ip)
fn process(mut stream: TcpStream, config: Arc<Config>) -> Result<(), Box<dyn Error>> {
    let sip = stream.peer_addr()?.ip().to_string();

    println!("Подключение: {}", &sip);

    let mut buf = vec![0; 1024];
    let buf_len = stream.read(&mut buf)?;
    buf.truncate(buf_len);

    let text = String::from_utf8(buf)?;

    let (ip, header_auth, domain) =
        read_request(&text).ok_or::<Box<dyn Error>>("ошибка запроса".into())?;

    if ip != sip {
        return Err("запрещено обновлять другой хост".into());
    }

    if header_auth != config.key {
        let _ = stream.write_all(RESPONSE!("badauth").as_bytes());
        return Err("неверный пароль".into());
    }

    stream.write_all(RESPONSE!(format!("good {}", ip)).as_bytes())?;

    println!("{domain} -> {ip}");

    match replace_record(&config.username, &config.password, &domain, &ip) {
        Err(v) => println!("Ошибка регистратора: {v}"),
        Ok(_) => println!("Запись обновлена"),
    }

    Ok(())
}

fn listen(config: Arc<Config>) -> Result<(), Box<dyn Error>> {
    let bind_addr = format!("0.0.0.0:{}", config.port);

    println!("Запуск DDNS сервера на {bind_addr}");

    let listener = TcpListener::bind(&bind_addr).map_err(|e| format!("{}", e))?;

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };

        let config = config.clone();

        thread::spawn(move || {
            match process(stream, config) {
                Ok(_) => {}
                Err(e) => {
                    println!("error: {}", e);
                }
            };
        });
    }

    Ok(())
}

fn main() {
    let config = Config::from_file(&PathBuf::from("config.yml")).unwrap_or_else(|e| {
        println!("Ошибка конфига: {e}");
        std::process::exit(0);
    });

    match listen(Arc::new(config)) {
        Ok(_) => {}
        Err(e) => {
            println!("Ошибка слушателя: {e}")
        }
    };
}
