use std::{error::Error, fs::File, io::BufReader};
use log::{debug, info};

use crate::pdf_diff::models::{Args, User};

pub fn log_all_users(users: &[User]) -> () {
    println!("length: {:?}", users.len());
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