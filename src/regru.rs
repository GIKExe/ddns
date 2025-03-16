
use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::TcpStream;
use native_tls::{TlsConnector, TlsStream};

use crate::Config;

macro_rules! REQUEST {
	($zone:expr, $data:expr) => {format!("\
	POST /api/regru2/zone/{} HTTP/1.1\r\n\
	Host: api.reg.ru\r\n\
	Content-Type: application/x-www-form-urlencoded\r\n\
	Content-Length: {}\r\n\
	\r\n{}", $zone, $data.len(), $data)};
}

pub struct SocketTLS {
	stream: RefCell<TlsStream<TcpStream>>,
}

impl SocketTLS {
	pub fn connect(host: &str, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
		let tcp_stream = TcpStream::connect((host, port))?;
		let connector = TlsConnector::new()?;
		let tls_stream = connector.connect(host, tcp_stream)?;
		
		Ok(SocketTLS {
			stream: RefCell::new(tls_stream),
		})
	}

	pub fn recv(&self) -> Option<String> {
		let mut stream = self.stream.borrow_mut();
		let mut buf = [0; 1024];
		match stream.read(&mut buf) {
			Ok(n) => {
				Some(String::from_utf8_lossy(&buf[..n]).to_string())
			},
			Err(_) => None,
		}
	}

	pub fn send(&self, data: &[u8]) {
		let mut stream = self.stream.borrow_mut();
		stream.write_all(data).ok();
	}

	// pub fn close(&self) {
	// 	if let Ok(mut stream) = self.stream.try_borrow_mut() {
	// 		let _ = stream.shutdown();
	// 		let tcp_stream = stream.get_ref();
	// 	tcp_stream.shutdown(Shutdown::Both).ok();
	// 	}
	// }
}


pub fn replace_record(config: &Config, domain: String, ip: String) -> Result<(), String> {
	let subdomain = "@";

	let socket = match SocketTLS::connect("api.reg.ru", 443) {
		Ok(socket) => socket, Err(e) => return Err(e.to_string()),
	};

	let data = format!(
		"username={}&password={}&domain_name={}&subdomain={}&output_format=plain&output_content_type=plain",
		config.username, config.password, domain, subdomain);
	socket.send(REQUEST!("clear", data).as_bytes());
	let text = socket.recv().ok_or("Сервер не ответил")?;
	let text = text.split("\r\n\r\n").nth(1).ok_or("Ошибка в данных от сервера")?;
	let text = text.split("\r\n").nth(1).ok_or("Ошибка в данных от сервера")?;
	if !text.starts_with("Success") {return Err(text.to_string());}

	let data = format!(
		"username={}&password={}&domain_name={}&subdomain={}&ipaddr={}&output_format=plain&output_content_type=plain",
		config.username, config.password, domain, subdomain, ip);
	socket.send(REQUEST!("add_alias", data).as_bytes());
	let text = socket.recv().ok_or("Сервер не ответил")?;
	let text = text.split("\r\n\r\n").nth(1).ok_or("Ошибка в данных от сервера")?;
	let text = text.split("\r\n").nth(1).ok_or("Ошибка в данных от сервера")?;
	if !text.starts_with("Success") {return Err(text.to_string());}

	Ok(())
}