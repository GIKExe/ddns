use std::{fs, io::{Read, Write}, net::{Shutdown, TcpListener, TcpStream}};

use regru::replace_record;
mod regru;

macro_rules! RESPONSE {
	($data:expr) => {format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", $data.len(), $data)};
}

struct Config {
	port: u16,
	key: String,
	username: String,
	password: String,
}

struct Socket {
	stream: TcpStream,
}

impl Socket {
	pub fn new(stream: TcpStream) -> Self {
		Socket {stream}
	}

	pub fn recv(&self) -> Option<String> {
		let mut stream = &self.stream;
		let mut buf: [u8; 1024] = [0; 1024];
		let n = stream.read(&mut buf).ok()?;
		Some(String::from_utf8_lossy(&buf[..n]).to_string())
	}

	pub fn send(&self, data: &[u8]) {
		let mut stream = &self.stream;
		stream.write_all(data).ok();
	}

	pub fn close(&self) {
		let stream = &self.stream;
		stream.shutdown(Shutdown::Both).ok();
	}
}



fn read_config() -> Result<Config, String> {
	let mut port: Option<u16> = None;
	let mut key: Option<&str> = None;
	let mut username: Option<&str> = None;
	let mut password: Option<&str> = None;

	let content = fs::read_to_string("config.txt").map_err(|e| format!("Ошибка чтения файла: {}", e))?;

	for line in content.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {continue}

		let mut parts = line.splitn(2, '=');
		let name = parts.next()
			.ok_or_else(|| format!("Некорректный формат строки: {}", line))?;
		let value = parts.next()
			.ok_or_else(|| format!("Некорректный формат строки: {}", line))?
			.trim_matches('"')
			.trim();

		match name.trim() {
			"port" => {
				port = Some(value.parse::<u16>().map_err(|e| format!("Некорректный порт: {}", e))?);
			}
			"key" => key = Some(value),
			"username" => username = Some(value),
			"password" => password = Some(value),
			other => return Err(format!("Неизвестный параметр: {}", other)),
		}
	}

	Ok(Config {
		port: port.ok_or("Порт не указан")?,
		key: key.ok_or("Ключ не указан")?.to_string(),
		username: username.ok_or("Логин не указан")?.to_string(),
		password: password.ok_or("Пароль не указан")?.to_string(),
	})
}



fn process(config: &Config, socket: &Socket, addr: String) -> Option<(String, String)> {
	let (sip, _) = addr.split_once(":")?;

	let text = socket.recv()?;
	let (http_parts, _) = text.split_once("\r\n\r\n")?;
	let mut http_parts = http_parts.split("\r\n");
	
	let http_line = http_parts.next()?;
	let (_, query) = http_line.split_once("?")?;
	let mut domain: Option<&str> = None;
	let mut ip: Option<&str> = None;
	for q in query.split('&') {
		let (name, value) = q.split_once('=')?;
		match name {
			"hostname" => domain = Some(value), "myip" => ip = Some(value), _ => continue,
		}
	}

	let mut header_auth: Option<&str> = None;
	for header in http_parts {
		let (name, value) = header.split_once(": ")?;
		match name {
			"Authorization" => {
				let (_, x) = value.split_once(" ")?;
				header_auth = Some(x);
			},
			_ => continue,
		}
	}

	if ip? != sip {println!("запрещено обновлять другой хост"); return None}
	if header_auth? != config.key {
		println!("неверный пароль");
		socket.send(RESPONSE!("badauth").as_bytes());
		return None
	}

	Some((domain?.to_string(), ip?.to_string()))
}



fn listen(config: &Config) -> Result<(), String> {
	let bind_addr = "0.0.0.0:".to_string() + &config.port.to_string();
	println!("Запуск DDNS сервера на {bind_addr}");
	let listener = TcpListener::bind(&bind_addr).map_err(|e| format!("{}", e))?;
	loop {
		let (socket, addr) = match listener.accept() {
			Ok((stream, addr)) => (Socket::new(stream), addr), Err(_) => continue,
		};
		println!("Подключение: {addr}");

		match process(config, &socket, addr.to_string()) {
			Some((domain, ip)) => {
				socket.send(RESPONSE!(format!("good {}", ip)).as_bytes());
				println!("{domain} -> {ip}");
				match replace_record(&config, domain, ip) {
					Err(v) => println!("Ошибка регистратора: {v}"), Ok(_) => println!("Запись обновлена")
				}
			}, None => {},
		};
		
		socket.close();
	}
}


fn main() {
	let config = read_config().unwrap_or_else(|e| {
		println!("Ошибка конфига: {e}"); std::process::exit(0);
	});

	let _ = listen(&config).unwrap_or_else(|e| {
		println!("Ошибка слушателя: {e}")
	});
}