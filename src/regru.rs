use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, HOST};
use std::collections::HashMap;
use std::error::Error;

pub fn replace_record(
    username: &str,
    password: &str,
    domain: &str,
    ip: &str,
) -> Result<(), Box<dyn Error>> {
    let subdomain = "@";
    let client = Client::new();

    let mut clear_form = HashMap::new();
    clear_form.insert("username", username);
    clear_form.insert("password", password);
    clear_form.insert("domain_name", domain);
    clear_form.insert("subdomain", subdomain);
    clear_form.insert("output_format", "plain");
    clear_form.insert("output_content_type", "plain");

    let clear_res = client
        .post("https://api.reg.ru/api/regru2/zone/clear")
        .form(&clear_form)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(HOST, "api.reg.ru")
        .send()?;

    let clear_text = clear_res.text()?;
    let clear_lines: Vec<&str> = clear_text.lines().collect();
    let clear_line = clear_lines.get(1).ok_or("Ошибка в данных от сервера")?;
    if !clear_line.starts_with("Success") {
        return Err(clear_line.to_string().into());
    }

    let mut add_form = clear_form;
    add_form.insert("ipaddr", ip);

    let add_res = client
        .post("https://api.reg.ru/api/regru2/zone/add_alias")
        .form(&add_form)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(HOST, "api.reg.ru")
        .send()?;

    let add_text = add_res.text()?;
    let add_lines: Vec<&str> = add_text.lines().collect();
    let add_line = add_lines
        .get(1)
        .ok_or::<Box<dyn Error>>("Ошибка в данных от сервера".into())?;
    if !add_line.starts_with("Success") {
        return Err(add_line.to_string().into());
    }

    Ok(())
}
