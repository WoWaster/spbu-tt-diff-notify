# 🛸Geraltt🛸: schedule change tracking tool for SPbU TimeTable

**Geraltt** is a tool for tracking changes in SPbU educators’ schedules via emails.  
It collects information about educators from the SPbU TimeTable website based on educators IDs, finds differences with old version, formats them into letter and sends it to all users, who are subscribed to said educators changes.

## How to get started

First thing you need to do is define files `users.json` and `config.json`, which are necessary for flexible adjusment of the tool. They must satisfy the following patterns:

### `users.json`

Provides the info about users who will receive notifications and the list of watched educators for each one of them.

```bash
[
    {
        "name": "User's Name", <- what the user prefers to be called
        "watch_educators": [
            5770, 1928, 1879 <- IDs of watched educators
        ],
        "watch_groups": [],
        "email": "example@gmail.com" <- user email address
    }
]
```

### `config.json`

Contains email sender configuration parameters.

```bash
{
    "email_relay": "mail.example.com", <- SMTP server address
    "email_sender_username": "sender@example.com", <- email address from which the letters will be sent
    "email_sender_fullname": "Notifications about schedule changes", <- sender display name
    "email_sender_password": "password" <- sender email password
}
```

### `previous_events.json`

Contains the information about schedule state at the time of the last Geraltt's launch. Shouldn't be made manually, you will only need to specify the path.

### Setup

Clone this repo:
```bash
  git clone git@github.com:WoWaster/spbu-tt-diff-notify.git
```

Open the project:
```bash
  cd spbu-tt-diff-notify
```

Run it with Cargo:
```bash
  cargo run --bin tt_diff -- \
  --users-json-path path/to/your/users.json \
  --config-json-path path/to/your/config.json \
  --previous-events-json-path path/to/your/previous_events.json
```

You might also want to set up automatic launch at certain time intervals for greater convenience.

## License

This project is distributed under the MIT License (check LICENSE for more info)

## Developers
* [Nikolai Ponomarev](https://github.com/WoWaster)
* [Ksenia Kotelnikova](https://github.com/p1onerka)
