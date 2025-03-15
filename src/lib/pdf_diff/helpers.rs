use lettre::{message::header::ContentType, Message};
use log::{debug, info};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};

use crate::pdf_diff::models::{Args, Config, Table, User};

pub fn log_all_users(users: &[User]) -> () {
    //println!("length: {:?}", users.len());
    for user in users.iter() {
        debug!(
            "Serving {}, who is watching tables {:?}",
            user.name, user.watch_tables
        );
    }
}

pub fn get_users(args: &Args) -> Result<Vec<User>, Box<dyn Error>> {
    //println!("inside get_users {:?}", &args.users_json_path);
    info!(
        "Reading users.json from {}",
        std::fs::canonicalize(&args.users_json_path)?.display()
    );
    let users_file = BufReader::new(File::open(&args.users_json_path)?);
    let users: Vec<User> = serde_json::from_reader(users_file)?;
    log_all_users(&users);
    Ok(users)
}

/* not sure if this is needed */
pub fn log_all_tables(tables: &[Table]) -> () {
    println!("length: {:?}", tables.len());
    for table in tables.iter() {
        debug!("Got table {}", table.table_name);
    }
}

pub fn get_tables(args: &Args) -> Result<Vec<Table>, Box<dyn Error>> {
    info!(
        "Reading previous_pdf_states.json from {}",
        std::fs::canonicalize(&args.previous_pdf_states_json_path)?.display()
    );
    let tables_file: BufReader<File> =
        BufReader::new(File::open(&args.previous_pdf_states_json_path)?);
    let tables: Vec<Table> = serde_json::from_reader(tables_file)?;
    log_all_tables(&tables);
    Ok(tables)
}

pub async fn fetch_and_hash_pdf(http_client: &Client, url: &str) -> Result<String, reqwest::Error> {
    info!("Getting pdf from link {}", url);
    let response = http_client.get(url).send().await?;
    let bytes = response.bytes().await?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}

pub fn generate_email(
    config: &Config,
    user: &User,
    watched_changes: &[&(String, String)],
) -> Result<Message, Box<dyn Error>> {
    let table_names: Vec<String> = watched_changes
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let table_list = table_names.join(", ");
    let body = format!(
        "Уважаемый(ая) {}!
        Следующие таблицы с расписаниями, которые вы отслеживаете, были изменены: {}.
        Данное письмо было сгенерировано автоматически, направление ответа не подразумевается.
        ",
        user.name, table_list
    );
    let email = Message::builder()
        .from(
            format!(
                "{} <{}>",
                config.email_sender_fullname, config.email_sender_username
            )
            .parse()?,
        )
        .to(format!("{} <{}>", user.name, user.email).parse()?)
        .subject("Обновление таблиц")
        .header(ContentType::TEXT_PLAIN)
        .body(body)?;

    Ok(email)
}

pub fn write_updated_table_hashes(
    args: &Args,
    updated_tables: &[(String, String)],
) -> Result<(), Box<dyn Error>> {
    let tables_path = Path::new(&args.previous_pdf_states_json_path);
    let mut tables: Vec<Table> = serde_json::from_reader(File::open(tables_path)?)?;
    for table in &mut tables {
        if let Some((_, new_hash)) = updated_tables
            .iter()
            .find(|(name, _)| name == &table.table_name)
        {
            table.hash = new_hash.clone();
        }
    }
    let file = File::create(tables_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &tables)?;
    info!(
        "Updated {} tables in {}",
        updated_tables.len(),
        tables_path.display()
    );
    Ok(())
}
